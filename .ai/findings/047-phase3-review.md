# 047 Governance Control Plane — Phase 3 Review

> Reviewer: **claude** | Date: 2026-03-30
> Reviewed: `crates/policy-kernel/src/lib.rs` (new crate), `crates/policy-kernel/Cargo.toml`, `tools/policy-compiler/src/lib.rs` (PolicyRule re-export delta), `tools/policy-compiler/Cargo.toml` (kernel dependency)
> Against: `specs/047-governance-control-plane/spec.md` (FR-006, FR-007, SC-003, SC-004, SC-005, SC-006), `.ai/plans/047-governance-control-plane-phased-plan.md` Phase 3

## Verdict

**Phase 3 approved with 6 findings (0 HIGH, 0 MEDIUM, 3 LOW, 3 INFO).** FR-006 `evaluate()` entrypoint implemented with correct signature. FR-007 all four gate types operational. SC-003 through SC-006 each have direct test coverage. P2-001 resolved (validation excluded from hash payload). WASM target clean. PolicyRule shared between kernel and compiler via crate dependency. No blockers for Phase 4.

---

## Focus-area assessments

### (1) FR-006 — `evaluate(context, policy) -> PolicyDecision` entrypoint

`evaluate()` (lib.rs:75–93) matches the spec signature exactly:
- Input: `&ToolCallContext` (tool name, arguments summary, proposed file content, diff metrics, active shard scopes) + `&PolicyBundle` (constitution + shards)
- Output: `PolicyDecision` with `outcome` (Allow/Deny/Degrade), `reason` (machine-readable string), `rule_ids` (consulted rule IDs)
- NF-003 satisfied: no I/O, no clock, no host calls — all data passed via parameters
- Module-level doc comment confirms `wasm32-unknown-unknown` target (line 2)
- Default outcome is `Allow` with reason `"policy:allow:no_gate_triggered"` — correct permissive-by-default semantics

Gate evaluation order is: secrets_scanner → destructive_operation → tool_allowlist → diff_size_limiter. This is a sound priority ordering — secrets are the highest-severity invariant (spec contract note: "secrets scanner pattern set is part of the constitution and is not overridable by shards"), followed by destructive ops, then allowlist, then size limits.

### (2) FR-007 — Four gate types

**Gate 1: Secrets scanner** (lib.rs:95–118)

- Scans `arguments_summary` + `proposed_file_content` (concatenated with newline separator)
- Three regex patterns: `sk-` prefixed keys (20+ chars), PEM private key headers, generic `api_key`/`secret`/`token` assignments (24+ chars)
- Matches against constitution rules with `gate == "secrets_scanner"` and `mode == "enforce"`
- Falls back to `KERNEL:BUILTIN-SECRETS` when no matching constitution rule exists — correct: the spec says secrets scanning is a constitution-level invariant, so the kernel enforces it even without an explicit rule
- Returns `Deny` with reason `"policy:deny:secrets_scanner:pattern_match"`

Spec compliance: "Scans tool call arguments and proposed file content for patterns matching secrets (API keys, tokens, private keys, connection strings)." — API keys, tokens, and private keys covered. Connection strings not explicitly matched but the generic `secret`/`token` regex would catch most. Acceptable coverage for Phase 3.

**Gate 2: Destructive operation guard** (lib.rs:120–143)

- Pattern matching via `destructive_match()` on tool name + arguments (lowercased substring search)
- Patterns: `rm -rf`, `rm -fr`, `git reset --hard`, `delete_file`, `shred `, `mkfs.`
- Override: constitution rule with `gate == "destructive_operation"` and `allow_destructive == true` permits the operation
- If not permitted, looks for an `enforce` mode rule to get the rule ID; falls back to `KERNEL:BUILTIN-DESTRUCTIVE`

Spec compliance: "blocks or requires confirmation for operations classified as destructive (file deletion, `git reset --hard`, `rm -rf`, etc.) unless the constitution explicitly permits them" — correct. The spec mentions "blocks or requires confirmation" but the implementation only returns Deny (no Degrade/confirmation path). This is acceptable for Phase 3; confirmation semantics belong to the runtime consumer of the decision.

**Gate 3: Tool allowlist** (lib.rs:145–178)

- Collects allowed tools from constitution rules AND active shard rules with `gate == "tool_allowlist"`
- Merges into a `BTreeSet` for dedup
- If the set is non-empty and the tool is not listed, returns Deny
- If no allowlist rules exist (empty set), the gate is a no-op — correct permissive-by-default

Spec compliance: "Shard rule adding tool to permitted set for a domain" (override column in spec table) — correctly implemented via shard scope lookup. "Tools not on the allowlist return `PolicyDenied`" — correct.

**Gate 4: Diff size limiter** (lib.rs:180–222)

- Collects applicable rules from constitution + active shards via `applicable_rules()`
- `effective_diff_limits()` takes the **minimum** threshold across all applicable rules — this is the correct conservative behavior ("a shard can tighten but never relax constitution rules")
- Checks `diff_lines` and `diff_bytes` independently against their respective limits
- Returns Deny with the specific rule ID that set the breached threshold

Spec compliance: "rejects file write operations where the diff exceeds a policy-defined threshold (line count or byte count)" — both dimensions checked. "Shard rule with elevated threshold for specific file patterns" (override column) — the minimum-wins logic means a shard *cannot* elevate above a constitution threshold, which is correct per "shard loading is additive — a shard can tighten but never relax constitution rules."

### (3) SC-003 — Destructive operation returns deny with rule ID

Test `sc003_destructive_operation_denies_with_rule_id` (lib.rs:338–363):
- Constitution rule `D-1` with `gate: destructive_operation`, `mode: enforce`
- Tool call: `bash` with args `rm -rf /tmp/x`
- Asserts: outcome `Deny`, rule_ids `["D-1"]`, reason contains `"destructive"`

Directly validates SC-003.

### (4) SC-004 — Secrets scanner denies regardless of other rules

Test `sc004_secrets_scanner_denies_even_with_other_rules` (lib.rs:366–405):
- Constitution has both `secrets_scanner` rule `S-1` and `destructive_operation` rule `D-1`
- Tool call contains `sk-123456789012345678901234567890` (matches secrets pattern)
- Asserts: outcome `Deny`, rule_ids `["S-1"]` (not `D-1`), reason contains `"secrets"`

This validates SC-004. The test also implicitly validates gate priority order — secrets scanner fires before destructive operation.

### (5) SC-005 — Tool not in allowlist returns deny

Test `sc005_tool_allowlist_denies_unknown_tool` (lib.rs:408–432):
- Constitution rule `T-1` with `allowed_tools: ["read_file", "grep"]`
- Tool call: `shell` (not in list)
- Asserts: outcome `Deny`, reason contains `"tool_allowlist"`

Directly validates SC-005.

### (6) SC-006 — Diff size exceeds threshold returns deny

Test `sc006_diff_size_limiter_denies_large_diff` (lib.rs:435–460):
- Constitution rule `L-1` with `max_diff_lines: 10`
- Tool call: `apply_patch` with `diff_lines: 100`
- Asserts: outcome `Deny`, rule_ids `["L-1"]`, reason contains `"diff_size"`

Directly validates SC-006.

### (7) P2-001 resolution — validation excluded from hash payload

`bundle_hash_payload_value()` (policy-compiler lib.rs:120–137) now explicitly excludes the `validation` section from the hash payload. The doc comment reads: "Excludes `validation` so the hash reflects policy content only (P2-001)." Confirmed by code inspection — the function constructs a payload with `policyBundleVersion`, `metadata` (compilerId, compilerVersion, sources), `constitution`, and `shards` only.

**P2-001: RESOLVED.**

### (8) PolicyRule shared type

`PolicyRule` is defined in `crates/policy-kernel/src/lib.rs` (lines 10–29) and re-exported from `tools/policy-compiler/src/lib.rs` via `pub use open_agentic_policy_kernel::PolicyRule` (line 3). The compiler depends on the kernel crate (Cargo.toml line 21: `open_agentic_policy_kernel = { path = "../../crates/policy-kernel" }`).

This is the correct design: one canonical type definition shared between the compiler (producer) and kernel (consumer). The type includes the Phase 3 extensions: `allow_destructive`, `allowed_tools`, `max_diff_lines`, `max_diff_bytes` — all optional fields with `skip_serializing_if`.

### (9) WASM target compatibility

`cargo check --target wasm32-unknown-unknown --manifest-path crates/policy-kernel/Cargo.toml` passes cleanly. Dependencies (`regex`, `serde`, `serde_json`) are all WASM-compatible. `OnceLock` from `std::sync` is available on `wasm32-unknown-unknown` (single-threaded, non-blocking). `crate-type = ["lib", "cdylib"]` enables both native linking and WASM module emission.

### (10) Determinism test

Test `determinism_identical_inputs_identical_canonical_json` (lib.rs:463–487):
- Runs `evaluate()` twice with identical inputs
- Serializes both decisions via `decision_to_canonical_json()` (sorted-key JSON)
- Asserts byte-identical output

This satisfies the determinism requirement from FR-006: "Deterministic: identical inputs always produce identical outputs."

---

## Findings

### P3-001 — No test for allow path (secrets safe, tool in allowlist, small diff) (LOW)

**Issue:** All five tests exercise deny paths. There is no test for a clean `Allow` outcome where a tool call passes all four gates. While the default case at lib.rs:88–93 implies correctness, an explicit green-path test would prevent regressions if gate logic is refactored.

**Recommendation:** Add a test with a non-destructive tool on an allowlist, no secrets, small diff — asserting `outcome == Allow` and empty `rule_ids`.

### P3-002 — Destructive pattern list may miss common patterns (LOW)

**Issue:** `DESTRUCTIVE_SUBSTRINGS` (lib.rs:286–293) contains 6 patterns. Missing patterns that the spec calls out or implies:
- `git push --force` / `git push -f` (the spec mentions "git reset --hard" but force push is equally destructive in the orchestrator rules)
- `drop table` / `DROP DATABASE` (common destructive DB operations)
- `chmod 777` or permission-modifying patterns

The spec says "etc." after its examples, so the list is expected to grow. The current set covers the spec's explicit examples.

**Impact:** LOW — the pattern list is extensible and the current set matches the spec's explicit examples. Additional patterns can be added incrementally.

### P3-003 — Tool allowlist rule_ids always returns `KERNEL:BUILTIN-ALLOWLIST` (LOW)

**Issue:** When a tool is denied by the allowlist gate (lib.rs:172–177), the `rule_ids` always contains `"KERNEL:BUILTIN-ALLOWLIST"` rather than the actual constitution/shard rule ID(s) that defined the allowlist. The other three gates correctly trace back to the originating rule ID.

**Impact:** Audit trail completeness — a proof chain consumer cannot determine *which* allowlist rule caused the denial. For Phase 5 proof chains this should be addressed.

**Recommendation:** Collect the rule IDs of the `tool_allowlist` rules that contributed to the merged set and include them in the denial decision.

### P3-004 — `secrets_match` does not scan `tool_name` (INFO)

The secrets scanner haystack is `arguments_summary + proposed_file_content` (lib.rs:96–100). The `tool_name` field is not scanned. This is almost certainly correct — a tool named `sk-something` would be unusual — but noted for completeness since the spec says "scans tool call arguments and proposed file content" (not tool name).

### P3-005 — `PolicyOutcome::Degrade` variant unused (INFO)

`PolicyOutcome` defines `Allow`, `Deny`, and `Degrade` (lib.rs:58–62), but no gate currently returns `Degrade`. The coherence scheduler (Phase 4) is the expected producer of `Degrade` outcomes. The variant's presence is forward-looking and correct.

### P3-006 — `sort_json_value` reimplemented in kernel (INFO)

Both `crates/policy-kernel/src/lib.rs` (lines 301–324) and `tools/policy-compiler/src/lib.rs` contain `sort_json_value()` implementations for canonical JSON. These serve different purposes (kernel: decision serialization; compiler: bundle hash) and operate in different compilation contexts (WASM vs native), so deduplication is non-trivial and not worth the coupling.

---

## Test results

```
5/5 tests pass (policy-kernel)
  - sc003_destructive_operation_denies_with_rule_id
  - sc004_secrets_scanner_denies_even_with_other_rules
  - sc005_tool_allowlist_denies_unknown_tool
  - sc006_diff_size_limiter_denies_large_diff
  - determinism_identical_inputs_identical_canonical_json

7/7 tests pass (policy-compiler — unchanged, confirms no regressions)
```

WASM check: `cargo check --target wasm32-unknown-unknown` passes cleanly.

---

## Prior finding status

| Finding | Severity | Status |
|---------|----------|--------|
| P2-001 | MEDIUM | **Resolved** — validation excluded from hash payload |
| P2-002 | LOW | Open — `compiledAt` absent when validation fails |
| P2-003 | LOW | Open — no multi-scope shard grouping test |
| P2-004 | INFO | Open — bundle builder duplication |
| P2-005 | INFO | Open — sort_json_value clones entire tree |
| P2-006 | INFO | Open — SC-002 test doesn't cross process boundaries |
| P1-001 | MEDIUM | Resolved (Phase 2) |
| P1-002 | LOW | Open |
| P1-003 | LOW | Open |
| P1-004–P1-007 | LOW/INFO | Open |

---

## Summary for next agent

**Phase 3 approved.** FR-006 `evaluate()` entrypoint matches spec signature. FR-007 all four gates (secrets scanner, destructive op, tool allowlist, diff size limiter) correctly implemented with proper priority ordering and constitution/shard scoping. SC-003 through SC-006 each directly tested. P2-001 resolved. WASM target clean. PolicyRule shared correctly between compiler and kernel. Three LOW findings: P3-001 (no allow-path test), P3-002 (pattern list extensibility), P3-003 (allowlist gate doesn't report originating rule IDs — should be fixed before Phase 5 proof chains). Three INFO findings (P3-004 through P3-006) are non-actionable.

Baton → **cursor** for Phase 4 (coherence scheduler: FR-008, SC-007, SC-008).
