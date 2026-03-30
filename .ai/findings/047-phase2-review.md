# 047 Governance Control Plane — Phase 2 Review

> Reviewer: **claude** | Date: 2026-03-30
> Reviewed: `tools/policy-compiler/src/lib.rs` (Phase 2 delta: b3318b5..e075749), `tools/policy-compiler/Cargo.toml`
> Against: `specs/047-governance-control-plane/spec.md` (FR-003, FR-004, FR-005, SC-001, SC-002), `.ai/plans/047-governance-control-plane-phased-plan.md` Phase 2

## Verdict

**Phase 2 approved with 6 findings (0 HIGH, 1 MEDIUM, 2 LOW, 3 INFO).** FR-003 classification correct. FR-004 bundle emission structurally complete with all required sections. FR-005 determinism satisfied via canonical JSON hashing excluding `compiledAt` and `policyBundleHash`. SC-001 and SC-002 both have direct test coverage. P1-001 (WalkDir `filter_entry`) confirmed fixed. No blockers for Phase 3.

---

## Focus-area assessments

### (1) FR-003 — Constitution/shard classification ✅

`classify_rules()` (lib.rs:185–199) correctly implements the spec's two-tier model:

- Constitution: `scope == "global" && mode == "enforce"` — matches FR-003 definition exactly
- Shards: all other rules, grouped by scope tag via `BTreeMap<String, Vec<PolicyRule>>`
- Both constitution and shard entries are sorted by rule ID for deterministic output (lines 195–198)

Edge cases handled correctly:
- A `scope=global` + `mode=warn` rule → shard (keyed under `"global"`) — this is spec-faithful since FR-003 requires both conditions
- A `scope=domain:payments` + `mode=enforce` → shard (keyed under `"domain:payments"`) — enforced but domain-scoped, correctly classified as shard
- Empty rule sets → empty constitution `[]` and empty shards `{}` — confirmed via real repo compilation

### (2) FR-004 — Bundle emission structure ✅

`build_bundle_json_value()` (lib.rs:94–128) emits a JSON object with all spec-required sections:

| FR-004 requirement | Bundle field | Present |
|---|---|---|
| (a) constitution section | `constitution` | ✅ Array of PolicyRule objects |
| (b) shard index mapping scope tags to content | `shards` | ✅ Object keyed by scope string |
| (c-i) version | `policyBundleVersion` | ✅ `"1"` |
| (c-ii) content hash | `metadata.policyBundleHash` | ✅ SHA-256 of canonical payload |
| (c-iii) compilation timestamp | `metadata.compiledAt` | ✅ RFC 3339, UTC seconds |
| (c-iv) source file manifest | `metadata.sources` | ✅ Array of `{path, precedence}` |

Additional metadata fields (`compilerId`, `compilerVersion`) are present — not spec-required but useful for auditability and consistent with the spec-compiler pattern.

The output file has been renamed from `phase1-validation.json` to `policy-bundle.json` — correct artifact name for the bundle.

### (3) FR-005 — Deterministic output ✅

Determinism strategy is sound:

1. **Canonical JSON hashing** — `hash_canonical_json()` (lib.rs:158–164) recursively sorts all JSON object keys via `sort_json_value()` before serializing to a compact string, then SHA-256 hashes it. This eliminates key-ordering non-determinism from `serde_json`.

2. **Excluded fields** — `bundle_hash_payload_value()` (lib.rs:131–151) constructs the hash input payload **without** `compiledAt` and `policyBundleHash`, matching FR-005's exclusion semantics ("excluding the compilation timestamp in metadata, consistent with Feature 001's `builtAt` treatment").

3. **Rule ordering** — Rules sorted by ID (lib.rs:245), constitution sorted by ID (lib.rs:195), shards sorted by ID within each scope (lib.rs:197), scope keys in `BTreeMap` (naturally sorted). Source precedence is fixed by discovery order. All inputs to the hash are deterministic.

4. **Hash format** — `sha256:{hex}` prefix (lib.rs:163) — clear, unambiguous, matches the spec's `sha256:abcdef...` example in the proof chain section.

### (4) SC-001 — Constitution + shard sections in bundle ✅

Test `sc001_constitution_and_shards_classification` (lib.rs:621–654):
- Creates a CLAUDE.md with one `scope=global, mode=enforce` rule (C-1) and one `scope=domain:payments, mode=warn` rule (S-1)
- Asserts `out.constitution.len() == 1` with `id == "C-1"`
- Asserts `out.shards` has one key `"domain:payments"` with one rule `id == "S-1"`
- Asserts `build_bundle_json_value()` has `constitution`, `shards`, and `metadata.policyBundleHash` present

This directly validates SC-001.

### (5) SC-002 — Byte-identical hash for identical inputs ✅

Test `sc002_identical_inputs_same_policy_bundle_hash` (lib.rs:657–679):
- Compiles the same fixture twice
- Asserts `policy_bundle_hash` values are equal
- Asserts `build_bundle_json_value().metadata.policyBundleHash` values are equal
- Comment correctly notes that `compiledAt` may match within the same UTC second but the hash stability is what SC-002 tests

This directly validates SC-002. The test is slightly weak in that both compilations happen in the same process invocation (same `CARGO_PKG_VERSION`, same filesystem state), but the exclusion of `compiledAt` from the hash payload is verified by code inspection of `bundle_hash_payload_value()`.

### (6) P1-001 resolution — WalkDir `filter_entry` ✅

The Phase 1 MEDIUM finding (WalkDir skip logic doesn't prevent descent) is fully resolved:

- Old `should_skip_dir()` function removed
- New `should_prune_walk_entry()` (lib.rs:333–346) used with `.filter_entry()` on the WalkDir iterator (lib.rs:296)
- `filter_entry` correctly prunes the subtree — WalkDir does not descend into matched directories
- `.claude` added to the prune list (was missing in Phase 1's skip list — good addition)
- Regression test `walkdir_does_not_descend_into_target_for_subdir_claude` (lib.rs:541–561) verifies `target/nested/CLAUDE.md` is not discovered

---

## Findings

### P2-001 — Hash payload includes `validation` section (MEDIUM)

**Issue:** `bundle_hash_payload_value()` includes the `validation` block (`passed` and `violations`) in the hash payload. If validation produces violations but `validation_passed` is still `true` (e.g., warnings only — currently unused but the data model supports non-error severities), the violations array content would affect the hash. More importantly, `violations` contains `path` fields with absolute-path fragments that could differ across machines if source paths are resolved differently.

Currently mitigated because:
1. Hash is only computed when `validation_passed == true` (lib.rs:257–259)
2. All current violations have `severity: "error"`, so `validation_passed` would be `false` and no hash is computed
3. Source paths are normalized via `normalize_repo_path` (relative, forward-slash)

**Risk:** If warning-severity violations are added in the future, two machines with different warning sets would produce different hashes for the same policy content. The spec says "identical inputs produce byte-identical output" — warnings from the same input would be identical, but this is a fragile coupling between validation diagnostics and the content hash.

**Recommendation:** Consider excluding `validation` from the hash payload entirely (the hash should represent the *policy content*, not the compilation diagnostics). Alternatively, document the inclusion as intentional.

### P2-002 — `compiledAt` and `policyBundleHash` absent when validation fails (LOW)

**Issue:** In `build_bundle_json_value()`, `compiledAt` and `policyBundleHash` are only inserted into the metadata object when `out.policy_bundle_hash.is_some()` (lib.rs:107–116). When validation fails, `policy_bundle_hash` is `None`, so the emitted JSON has neither field.

**Impact:** This is arguably correct behavior — a failed compilation shouldn't claim a content hash. But `compiledAt` is independent of validation success; it records *when* the compilation attempt occurred, which could be useful for debugging failed compilations.

**Recommendation:** Consider always emitting `compiledAt`, only gating `policyBundleHash` on validation success. Low priority — current behavior is defensible.

### P2-003 — No test for multi-scope shard grouping (LOW)

**Issue:** `sc001_constitution_and_shards_classification` tests one constitution + one shard scope. It does not test:
- Multiple rules in the same shard scope (verifying array accumulation)
- Multiple distinct shard scopes (verifying map has multiple keys)
- A `scope=global, mode=warn` rule (verifying it lands in shards under `"global"` key, not in constitution)

**Recommendation:** Extend the SC-001 test or add a companion test with a richer fixture covering these edge cases before Phase 3.

### P2-004 — `build_bundle_json_value` duplicates serialization logic (INFO)

`build_bundle_json_value()` and `bundle_hash_payload_value()` are nearly identical, differing only in the presence of `compiledAt` and `policyBundleHash`. The `serde_json::to_value()` calls for `sources`, `constitution`, `shards`, and `violations` are duplicated across both functions.

A shared builder that produces the base value, with `build_bundle_json_value` adding the two extra fields, would reduce the duplication. Not a correctness issue — purely structural.

### P2-005 — `sort_json_value` clones entire value tree (INFO)

`sort_json_value()` takes ownership of a `Value` and returns a new sorted copy, recursively cloning objects through the `BTreeMap` intermediate. For the current bundle sizes (< 100 rules) this is negligible. For very large policy sets (hundreds of shards with many rules each), the double traversal + allocation could matter.

Not a Phase 2 concern given NF-002's 50-source-file / 2-second budget is easily met.

### P2-006 — SC-002 test does not cross process boundaries (INFO)

As noted in the assessment, both compilations in the SC-002 test run in the same process. A stronger golden test would serialize the hash payload to a file, then assert a known expected hash value. This would catch regressions where a dependency update (e.g., `serde_json` key ordering) silently changes the canonical form.

Not blocking — the current test is valid for same-version determinism.

---

## Test results

```
7/7 tests pass (policy-compiler)
  - discovers_sources_in_precedence_order
  - walkdir_does_not_descend_into_target_for_subdir_claude  (NEW — P1-001 regression)
  - parses_valid_policy_block
  - reports_invalid_mode_and_scope
  - duplicate_rule_prefers_higher_precedence_source
  - sc001_constitution_and_shards_classification  (NEW — SC-001)
  - sc002_identical_inputs_same_policy_bundle_hash  (NEW — SC-002)
```

Live compilation on actual repo produces valid `build/policy-bundles/policy-bundle.json` with empty constitution/shards (no `policy` blocks in repo CLAUDE.md), correct metadata, and stable hash.

---

## Phase 1 finding status

| Finding | Severity | Status |
|---------|----------|--------|
| P1-001 | MEDIUM | ✅ **Resolved** — `filter_entry` prune + regression test |
| P1-002 | LOW | Open — dead duplicate-replacement branch still present (lib.rs:226) |
| P1-003 | LOW | Open — V-103 "ignored due to precedence" message still misleading for same-precedence |
| P1-004 | LOW | Open — no V-101 unterminated block test |
| P1-005 | LOW | Open — no V-102 missing field test |
| P1-006 | INFO | Open — no V-106 invalid gate test |
| P1-007 | INFO | Open — no frontmatter stripping test |

---

## Summary for next agent

**Phase 2 approved.** FR-003 classification, FR-004 bundle emission, FR-005 deterministic hashing all spec-faithful. SC-001 and SC-002 directly tested. P1-001 resolved. One MEDIUM finding (P2-001: validation in hash payload) worth addressing before the hash becomes load-bearing in Phase 5 proof chains. P2-002/P2-003 are cleanup items. P1-002 through P1-007 remain open from Phase 1 but are non-blocking.

Baton → **cursor** for Phase 3 (WASM policy kernel + gate enforcement: FR-006, FR-007, SC-003, SC-004, SC-005, SC-006).
