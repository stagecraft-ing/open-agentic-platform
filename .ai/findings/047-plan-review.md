# 047 Governance Control Plane — Plan Review

> Reviewer: **claude** | Date: 2026-03-30
> Reviewed: `.ai/plans/047-governance-control-plane-phased-plan.md`
> Against: `specs/047-governance-control-plane/spec.md`

## Verdict

**Plan approved with 7 findings (1 HIGH, 3 MEDIUM, 3 LOW, 2 INFO).** All 11 FR, 4 NF, and 11 SC requirements are covered. Phase ordering is sound. F-001 (WASM host runtime) must be resolved before Phase 3 coding begins. F-003 (constitution append-only invariant) should be addressed in Phase 2 or Phase 6 design.

---

## Requirement coverage matrix

| Req | Phase | Covered | Notes |
|-----|-------|---------|-------|
| FR-001 | 1 | ✅ | Discovery with explicit precedence |
| FR-002 | 1 | ✅ | Rule extraction with id/description/mode/scope/gate |
| FR-003 | 2 | ✅ | Constitution vs shard classification |
| FR-004 | 2 | ✅ | Bundle emission with constitution, shard index, metadata |
| FR-005 | 2 | ✅ | Deterministic serialization (G-003) |
| FR-006 | 3 | ✅ | WASM kernel evaluate entrypoint |
| FR-007 | 3 | ✅ | Four gates: destructive, secrets, allowlist, diff size |
| FR-008 | 4 | ✅ | Coherence scheduler with privilege degradation |
| FR-009 | 5 | ✅ | Proof record schema with chained hashes |
| FR-010 | 5 | ✅ | Standalone verifier utility |
| FR-011 | 1 | ✅ | V-series violation codes |
| NF-001 | 6 | ✅ | <5ms p99 kernel evaluation benchmark |
| NF-002 | 6 | ✅ | <2s compilation for 50 source files |
| NF-003 | 3 | ✅ | G-004 ensures no FS/network/syscall access |
| NF-004 | 5 | ✅ | Fixed-size record budgeting |
| SC-001 | 2 | ✅ | Golden test for constitution + shard sections |
| SC-002 | 2 | ✅ | Byte-identical output golden test |
| SC-003 | 3 | ✅ | Destructive op deny scenario |
| SC-004 | 3 | ✅ | Secrets scanner deny scenario |
| SC-005 | 3 | ✅ | Tool allowlist deny scenario |
| SC-006 | 3 | ✅ | Diff size deny scenario |
| SC-007 | 4 | ✅ | Privilege degradation on violation count |
| SC-008 | 4 | ✅ | Monotonicity test — no self-promotion |
| SC-009 | 5 | ✅ | 100-record chain verification |
| SC-010 | 6 | ✅ | <5ms p99 benchmark |
| SC-011 | 6 | ✅ | execution/verification.md evidence |

---

## Findings

### F-001 — WASM host runtime unspecified (HIGH)

**Issue:** The plan targets `wasm32-unknown-unknown` (G-004) and the spec says axiomregent loads the WASM kernel at session start. However, axiomregent is a native Rust MCP server binary. Loading WASM from native Rust requires a host runtime (wasmtime, wasmer, etc.). The plan does not specify which runtime, nor does it account for the host runtime dependency in axiomregent's `Cargo.toml`.

**Why this matters:** The host runtime choice affects Phase 3 API design (how `evaluate()` is called from Rust), Phase 6 integration (axiomregent dispatch path changes), and NF-001 benchmarks (host overhead varies significantly between wasmtime and wasmer).

**Spec context:** The spec notes WASM portability for "browsers, server-side runtimes, and edge environments" — but the immediate consumer is axiomregent (native Rust). An alternative is to compile the kernel as a native Rust crate for axiomregent and separately as WASM for other targets, using a shared core with cfg-gated host bindings.

**Recommendation:** Resolve before Phase 3. Add a G-006 decision: either (a) wasmtime in axiomregent with the `wasmtime` crate, or (b) dual-target — native Rust library for axiomregent + WASM build for external consumers. Option (b) avoids the WASM host overhead on the critical path and trivially satisfies NF-001.

### F-003 — Constitution append-only invariant not addressed (MEDIUM)

**Issue:** The spec's contract notes state: *"Constitution rules are append-only across policy bundle versions within a session. A new compilation cannot remove a constitution rule that was active when the session started — this prevents policy downgrade attacks."* The plan does not address this invariant in any phase.

**Impact:** Without this, a mid-session recompilation could silently weaken enforcement. This is a security-relevant contract.

**Recommendation:** Address in Phase 2 (bundle emission should include a mechanism for session-scoped bundle pinning) or Phase 6 (axiomregent runtime should compare loaded constitution rule IDs against the new bundle and reject removals). The latter is simpler and keeps compilation stateless.

### F-004 — Governance config file not in plan (MEDIUM)

**Issue:** The spec says coherence thresholds "should be configurable per-repository via `.claude/governance.toml` or equivalent." The plan's Phase 4 mentions "decay factor defaults" but doesn't mention the config file, its schema, or discovery.

**Recommendation:** Add config file parsing to Phase 4 deliverables. Minimal schema: `[coherence] thresholds = { full = 0.8, restricted = 0.5, read_only = 0.2 }` and `window_size = 50`, `decay_lambda = 0.95`.

### F-005 — Frontmatter parser sharing strategy undefined (MEDIUM)

**Issue:** The spec says the policy compiler "reuses the spec-compiler's frontmatter YAML parser (extracted as a shared crate or vendored)." No shared crate exists today — `split_frontmatter()` is duplicated inline in both `tools/spec-compiler/src/lib.rs` (lines 460-486) and `tools/spec-lint/src/lib.rs` (lines 40-55).

**Impact:** Low risk but should be decided before Phase 1 to avoid a third copy.

**Recommendation:** Either (a) extract `tools/shared/frontmatter/` as a tiny crate with `split_frontmatter()` + `serde_yaml` re-export, or (b) vendor (copy) into policy-compiler and accept the duplication. Decide in Phase 1 scaffold step.

### F-006 — Rule annotation syntax not finalized (LOW)

**Issue:** G-002 says "support explicit machine-readable rule blocks first" but lists two candidate syntaxes without committing: fenced `policy` blocks vs HTML `<!-- policy: ... -->` directives. The spec's R-001 also lists both.

**Impact:** Affects Phase 1 parser implementation directly.

**Recommendation:** Commit to one syntax before Phase 1 coding. Suggestion: fenced code blocks with `policy` language tag — these render visibly in GitHub/editors and don't require HTML comment parsing.

### F-007 — PolicyBundle format not specified (LOW)

**Issue:** The plan says Phase 2 emits a "bundle artifact" but doesn't specify JSON vs MessagePack. The spec's architecture diagram shows "policy-bundle.json (or .msgpack for binary transport)."

**Impact:** Affects Phase 3 kernel deserialization. JSON is simpler; MessagePack is smaller but adds a dependency.

**Recommendation:** Start with JSON (consistent with spec-compiler's `registry.json`). Add MessagePack as an optional output format later if NF-001 benchmarks show deserialization overhead matters.

### F-008 — `policyBundleHash` in registry.json not tracked (LOW)

**Issue:** The spec states "The spec-compiler's `registry.json` gains an optional `policyBundleHash` field per feature." The plan does not mention this cross-tool integration.

**Recommendation:** Track as a Phase 6 follow-on or add to Phase 6 deliverables. It's a minor change to the spec-compiler but represents a contract between two tools.

### F-009 — Phase 3 scope is large (INFO)

Phase 3 introduces the WASM kernel scaffold, all 4 gate implementations, decision payloads, and determinism tests. This is the heaviest phase. Consider splitting into 3a (kernel scaffold + destructive op gate + secrets scanner) and 3b (tool allowlist + diff size limiter) if implementation velocity suggests it.

### F-010 — AxiomRegentError extension is straightforward (INFO)

Confirmed: the current `AxiomRegentError` enum in `crates/axiomregent/src/router/mod.rs` has 6 variants (NotFound, InvalidArgument, RepoChanged, PermissionDenied, TooLarge, Internal). Adding `PolicyDenied(String)` is a clean addition. The dispatch path in `preflight_tool_permission()` currently runs `check_tool_permission()` → the plan correctly inserts `evaluate_policy()` after this check in Phase 6.

---

## Phase ordering assessment

| Transition | Dependency | Risk |
|-----------|------------|------|
| 1 → 2 | Phase 2 classifies rules extracted in Phase 1 | Clean |
| 2 → 3 | Phase 3 kernel consumes bundles emitted in Phase 2 | Clean |
| 3 → 4 | Phase 4 coherence scheduler observes gate decisions from Phase 3 | Clean |
| 4 → 5 | Phase 5 proof chain records decisions from Phase 3+4 | Clean |
| 5 → 6 | Phase 6 integrates all components into axiomregent | Clean |

**No phase-ordering risks detected.** Each phase's outputs feed cleanly into the next. The only caveat is F-001 — the WASM host runtime decision affects Phase 3 API design and should be resolved before Phase 3 begins.

---

## Pre-implementation decision assessment (G-001 to G-005)

| Decision | Assessment |
|----------|-----------|
| G-001 (repo topology) | ✅ Sound — mirrors spec-compiler layout |
| G-002 (rule extraction) | ⚠️ Syntax not finalized (F-006) |
| G-003 (deterministic emission) | ✅ Sound — satisfies FR-005/SC-002 |
| G-004 (WASM target) | ⚠️ Host runtime unspecified (F-001) |
| G-005 (proof-chain storage) | ✅ Sound — host-side with shared hash logic |

---

## Summary for next agent

**Plan coverage: complete.** All 26 requirements mapped to phases. Seven findings, one blocking (F-001 must be resolved before Phase 3). Recommend cursor proceeds with Phase 1 after resolving F-006 (syntax choice) and acknowledging F-005 (parser sharing strategy). F-001 can be deferred to a G-006 decision before Phase 3.
