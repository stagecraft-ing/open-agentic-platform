# 047 Governance Control Plane — Phase 1 Review

> Reviewer: **claude** | Date: 2026-03-30
> Reviewed: `tools/policy-compiler/src/lib.rs`, `tools/policy-compiler/src/main.rs`, `tools/shared/frontmatter/`
> Against: `specs/047-governance-control-plane/spec.md` (FR-001, FR-002, FR-011)

## Verdict

**Phase 1 approved with 7 findings (1 MEDIUM, 3 LOW, 3 INFO).** FR-001 discovery precedence correct. FR-002 parser faithfully extracts rules from fenced `policy` blocks with correct shape. FR-011 V-series codes V-101 through V-106 all implemented with clear messages. F-005 shared frontmatter extraction clean. F-006 syntax commitment consistent. G-006 decision recorded. No blockers for Phase 2.

---

## Focus-area assessments

### (1) FR-001 — Discovery precedence and exclusions ✅

Discovery follows spec-mandated order: repo root `CLAUDE.md` (precedence 0), `.claude/policies/*.md` (precedence 1), subdirectory `CLAUDE.md` (precedence 2). Exclusions correct:

- Root `CLAUDE.md` excluded from subdirectory walk (`path == root_claude` guard, line 179)
- `.claude/` subtree excluded from subdirectory walk (`rel.starts_with(".claude")` guard, line 185)
- `.git`, `node_modules`, `target`, `build` directories skipped (line 169 + `should_skip_dir`)
- `.claude/policies/` scoped to `.md` extension only (line 151)
- Alphabetical sorting within each precedence tier (lines 155, 192)

**Finding P1-001** below: the WalkDir skip logic has a performance defect but not a correctness defect.

### (2) FR-002 — Parser fidelity for fenced `policy` blocks ✅

Parser correctly:
- Detects ```` ```policy ```` open fence and ```` ``` ```` close fence (lines 219, 224)
- Accumulates block content preserving original indentation (line 234)
- Deserializes via `serde_yaml` into `RawRuleBlock` with all 5 fields (line 252)
- Validates required fields (`id`, `description`, `mode`, `scope`) and optional `gate`
- Produces `PolicyRule` with correct shape matching FR-002: `id`, `description`, `mode` (enforce|warn|log), `scope` (global|domain:<name>|task:<pattern>), optional `gate`

Rule shape serializes with `sourcePath` (camelCase) and skips `gate` when None — clean for JSON consumption in Phase 2.

### (3) FR-011 — V-series validation codes ✅

All 6 violation codes implemented:

| Code | Trigger | Verified |
|------|---------|----------|
| V-101 | Unterminated block / invalid YAML | ✅ Both paths (lines 240, 254) |
| V-102 | Missing required field | ✅ Via `required_field()` for id/description/mode/scope |
| V-103 | Duplicate rule ID | ✅ With precedence-based resolution (line 97) |
| V-104 | Invalid mode value | ✅ Match against enforce/warn/log (line 277) |
| V-105 | Invalid scope value | ✅ Via `valid_scope()` (line 288) |
| V-106 | Invalid gate value | ✅ Match against 4 gate types (line 299) |

Error messages are clear and include the offending value. All violations include source path for traceability. `validationPassed` correctly checks for any `severity == "error"` violations (line 126).

### (4) F-005 — Shared frontmatter parser extraction ✅

Clean resolution:
- `tools/shared/frontmatter/` crate extracted with `split_frontmatter_required()` and `split_frontmatter_optional()`
- `tools/spec-compiler/Cargo.toml` depends on `open_agentic_frontmatter` and uses `split_frontmatter_required` (strict mode)
- `tools/spec-lint/Cargo.toml` depends on `open_agentic_frontmatter` and uses `split_frontmatter_optional` (lenient mode)
- `tools/policy-compiler/Cargo.toml` depends on `open_agentic_frontmatter` and uses `split_frontmatter_optional` (lenient — correct, since policy files may lack frontmatter)
- All three tools build and test cleanly with the shared crate

No duplication remains. Shared crate is minimal (YAML splitting only, no business logic leakage). R-005 from the spec adequately mitigated.

### (5) F-006 — Syntax commitment consistency ✅

Plan G-002 commits to fenced `policy` code blocks. Parser implements exactly this syntax (` ```policy ... ``` `). No HTML directive parsing, no ambiguity. Consistent with R-001 mitigation strategy (explicit annotation over heuristic extraction).

### (6) G-006 — Host runtime decision alignment ✅

G-006 added to plan: dual-target native+WASM strategy. Native Rust kernel for axiomregent critical path, WASM artifact for non-native hosts. No Phase 1 code impact — this is a Phase 3 concern. Decision correctly defers WASM host runtime dependency (no `wasmtime`/`wasmer` needed in axiomregent until Phase 3, and G-006 may avoid it entirely via native path). Aligns with spec's portability intent while avoiding unnecessary overhead.

---

## Findings

### P1-001 — WalkDir directory skip does not prevent descent (MEDIUM)

**Issue:** `discover_policy_sources` at line 169 calls `should_skip_dir()` and `continue`s on matching directories (`.git`, `node_modules`, `target`, `build`). However, WalkDir's default iterator has already queued the directory's children by the time `continue` executes. The `continue` skips the directory entry itself but does **not** prevent the walker from descending into it.

**Impact:** Correctness is unaffected — only `CLAUDE.md` files are collected regardless. But performance is degraded: the walker traverses the entirety of `node_modules/`, `target/`, `.git/`, and `build/` directories, which can contain hundreds of thousands of entries in a typical project.

**Fix:** Replace the default iterator with `filter_entry`:

```rust
for entry in WalkDir::new(repo_root)
    .min_depth(1)
    .into_iter()
    .filter_entry(|e| !e.file_type().is_dir() || !should_skip_dir(e.path()))
{
```

This prevents descent into skipped directories entirely.

### P1-002 — Dead duplicate-replacement branch (LOW)

**Issue:** The duplicate resolution at line 106 checks `if source.precedence < *existing_precedence` and replaces the rule if the new source has higher precedence (lower number). However, sources are always iterated in precedence order (0 → 1 → 2), so the current source's precedence is always >= the existing entry's. This branch is unreachable.

**Impact:** No functional issue — the first-seen rule (highest precedence) is always correct. The dead branch is defensive code for an ordering invariant that discovery guarantees. If discovery ordering were ever broken, the branch would become load-bearing, but currently it cannot be tested.

**Recommendation:** Either (a) remove the branch and simplify to always-keep-first, or (b) add an assertion/debug_assert that sources arrive in non-decreasing precedence order. Option (a) is cleaner.

### P1-003 — V-103 message misleading for same-precedence duplicates (LOW)

**Issue:** When two files at the same precedence (e.g., `.claude/policies/a.md` and `.claude/policies/b.md`) define the same rule ID, the V-103 message says "ignored due to precedence" — but both sources have identical precedence. The actual behavior is first-wins (alphabetical order within a tier).

**Recommendation:** Adjust message to distinguish: "duplicate rule id ... (kept from {path}, same-or-higher precedence)" or split into two cases.

### P1-004 — No test for unterminated block (V-101 first path) (LOW)

**Issue:** The `V-101` unterminated block path (line 239) has no test coverage. Only the valid-block and invalid-mode/scope paths are tested.

**Recommendation:** Add a fixture test with an unclosed ` ```policy ` block and assert V-101 is reported.

### P1-005 — No test for missing required fields (V-102) (LOW)

**Issue:** The `required_field()` function and V-102 code path have no direct test coverage.

**Recommendation:** Add a fixture with a policy block missing `id` or `description` and assert V-102.

### P1-006 — No test for invalid gate (V-106) (INFO)

The V-106 gate validation path has no test. Consider adding a fixture with `gate: unknown` and asserting V-106.

### P1-007 — No test for frontmatter stripping in policy files (INFO)

The `split_frontmatter_optional` call (line 91) that strips YAML frontmatter from policy sources before parsing is not tested. A fixture with a policy file containing `---\ntitle: foo\n---\n```policy...` would verify the stripping path.

---

## Test results

```
4/4 tests pass (policy-compiler)
spec-compiler: builds clean with shared frontmatter crate
spec-lint: builds clean with shared frontmatter crate
```

---

## Compilation output structure

The `phase1-validation.json` artifact at `build/policy-bundles/phase1-validation.json` has correct structure: `sources`, `rules` (sorted by ID), `violations`, `validationPassed`. CLI exit codes are well-defined: 0 (clean), 1 (violations found), 3 (error). `validate` command prints violations to stderr in a greppable format.

---

## Summary for next agent

**Phase 1 approved.** All Phase 1 deliverables (FR-001 discovery, FR-002 parsing, FR-011 validation) are spec-faithful. F-005/F-006/G-006 all resolved correctly. P1-001 (WalkDir descent) is the only MEDIUM finding and should be fixed before Phase 2 to avoid surprising performance on real repositories. P1-002/P1-003 are cleanup items. P1-004/P1-005 are test coverage gaps for Phase 2.

Baton → **cursor** for Phase 2 (constitution/shard classification + deterministic bundle emission).
