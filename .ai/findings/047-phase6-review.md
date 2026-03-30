# 047 Phase 6 Review — Axiomregent Integration + Benchmarks + Verification

> Reviewer: **claude** | Date: 2026-03-30 | Verdict: **APPROVED**

## Scope

Phase 6 deliverables per plan: axiomregent policy preflight integration, `POLICY_DENIED` wire code distinct from `PERMISSION_DENIED`, Criterion benchmarks for SC-010 (kernel latency) and NF-002 (compile time), `execution/verification.md` evidence document. Requirements: SC-010, SC-011, NF-001, NF-002, contract note on error distinguishability.

## Requirement-by-requirement assessment

### SC-011 / Dispatch order — grants → evaluate → handler ✅

Spec: "Tool dispatch adds a policy evaluation step after tier and permission checks: `check_tier() -> check_permissions() -> evaluate_policy() -> dispatch`."

Implementation (`router/mod.rs:121–184`): `preflight_tool_permission()` first calls `permissions::check_tool_permission()` (with lease) or `permissions::check_grants()` (without lease). Only on `Ok(())` does it call `self.policy_preflight_response()` (lines 147, 170). On permission failure, returns `json_rpc_permission_denied` immediately — policy evaluation never runs.

`policy_preflight_response()` (`router/mod.rs:187–207`): extracts `repo_root` from args → loads bundle from cache → builds `ToolCallContext` → calls `evaluate_loaded_policy`. Uses `?` operator on three fallible steps: missing `repo_root`, missing/invalid bundle, and allow outcome all return `None` (no objection). Only deny/degrade returns `Some(json_rpc_policy_denied(...))`.

Call site (`router/mod.rs:555–557`): `if let Some(resp) = self.preflight_tool_permission(...)` returns early on any denial (permission or policy), otherwise falls through to the tool handler `match name { ... }`.

**Dispatch order faithful to spec.**

### Contract note — `POLICY_DENIED` vs `PERMISSION_DENIED` wire distinguishability ✅

Spec: "The `PolicyDenied` error variant must be distinguishable from `PermissionDenied` (Feature 035) at the wire level."

Implementation:
- `AxiomRegentError::PolicyDenied(String)` variant (`mod.rs:49`) — code: `"POLICY_DENIED"` (line 61)
- `AxiomRegentError::PermissionDenied(String)` variant (`mod.rs:47`) — code: `"PERMISSION_DENIED"` (line 60)
- `json_rpc_policy_denied()` (`mod.rs:1239–1249`) emits `{ "code": "POLICY_DENIED", "message": ... }`
- `json_rpc_permission_denied()` emits `{ "code": "PERMISSION_DENIED", "message": ... }`

Test `policy_denied_wire_code_when_allowlist_excludes_tool` verifies: tool allowlist excludes `features.impact` → response error code is `"POLICY_DENIED"`.
Test `permission_denied_still_permission_code` verifies: file_write permission denied → response error code is `"PERMISSION_DENIED"`.

**Distinct wire codes confirmed. Both tests pass.**

### Fallback — missing bundle skips policy evaluation ✅

Spec risk note R-002: "Missing or unreadable bundle skips policy evaluation."

`PolicyBundleCache::bundle_for_repo_root()` (`policy_bundle.rs:23–40`): returns `None` when the bundle file is absent or cannot be parsed. `policy_preflight_response` propagates `None` via `?`, meaning the tool call proceeds without policy enforcement.

**Graceful fallback faithful to spec.**

### PolicyBundleCache — per-repo caching ✅

`PolicyBundleCache` (`policy_bundle.rs:11–41`): `RwLock<HashMap<String, Option<Arc<PolicyBundle>>>>`. Read lock for cache hit, write lock for miss + load. Cached `None` for repos without bundles avoids repeated filesystem probes. `Arc<PolicyBundle>` avoids cloning the bundle on every evaluation.

### build_tool_call_context — NF-003 compliance ✅

Spec NF-003: "The WASM kernel has no access to filesystem, network, or system calls — all inputs are passed via the evaluation function interface."

`build_tool_call_context()` (`policy_bundle.rs:72–94`): constructs `ToolCallContext` from the tool name and MCP argument map. Extracts `proposed_file_content` from `content`, `patch`, `text`, or `body` argument keys. Computes `diff_lines` and `diff_bytes` from proposed content. Reads `active_shard_scopes` from `OPC_POLICY_ACTIVE_SHARDS` env var.

**All kernel inputs are host-supplied — kernel performs no I/O.**

### SC-010 — Kernel evaluation latency benchmark ✅

Spec: "The WASM kernel evaluation latency is < 5ms p99 on a benchmark of 1000 synthetic tool call evaluations."

Benchmark (`crates/policy-kernel/benches/kernel_eval.rs`): Criterion benchmark `evaluate_x1000_allow_path` — 1000 × `evaluate()` per iteration on a bundle with tool allowlist + secrets scanner rules. Uses `black_box` to prevent compiler optimizations.

Verification evidence (`execution/verification.md`): observed ~886–892 µs total for 1000 evaluations (~0.9 µs per call). Far below the 5 ms per-call budget.

**SC-010 satisfied.**

### NF-002 — Compile time for large policy trees ✅

Spec: "Policy bundle compilation for a repository with up to 50 policy source files completes in < 2 seconds."

Benchmark (`tools/policy-compiler/benches/compile_many.rs`): Criterion benchmark `compile_50_policy_sources` — sets up a tempdir with 1 root `CLAUDE.md` + 49 `.claude/policies/*.md` files, each containing one fenced `policy` block.

Verification evidence: observed ~1.34–1.36 ms per `compile()` call. Well under the 2-second ceiling.

**NF-002 satisfied.**

### execution/verification.md ✅

SC-011: "`execution/verification.md` records commands and results for all criteria."

`specs/047-governance-control-plane/execution/verification.md` documents:
- SC-010 kernel benchmark command + observed result
- NF-002 compile benchmark command + observed result
- SC-011 axiomregent preflight test command + explanation
- Integration path (dispatch order, fallback behavior)

**Verification evidence document complete.**

## Verification

- **2/2 policy preflight tests pass** (`cargo test --manifest-path crates/axiomregent/Cargo.toml --test policy_preflight_test`)
- **14/14 policy-kernel tests pass** (unchanged from Phase 5)
- Audit log emitted correctly: `"decision":"policy_denied"` in test output
- Benchmark files compile (Criterion benches)

## Findings

### P6-001 — Bundle cache never invalidated (LOW)

`PolicyBundleCache` caches bundles permanently per `repo_root` with no invalidation mechanism. If a user recompiles `policy-bundle.json` during a session, the cache serves stale data. For long-running axiomregent sessions this means policy changes require a restart.

**Recommendation**: Add a `PolicyBundleCache::invalidate(repo_root)` method or timestamp-based staleness check. Could be triggered on `tools/call` to `gov.compile` or similar.

### P6-002 — `policy_preflight_response` returns `None` on `repo_root` absence (LOW)

When `args` has no `repo_root` key, `policy_preflight_response` returns `None` (line 194, `?` operator), silently skipping policy evaluation. This is correct for tools that don't operate on a repo (e.g., system introspection tools), but means any tool that omits `repo_root` from its arguments bypasses policy — even if the tool semantically operates on a repo.

**Impact**: Depends on whether all repo-operating tools consistently include `repo_root` in their argument schema. Currently appears true for all axiomregent tools. Low risk if this invariant is maintained.

### P6-003 — `diff_lines`/`diff_bytes` computed from `proposed_file_content` only (LOW)

`build_tool_call_context` (`policy_bundle.rs:82–85`) computes `diff_lines` as `proposed_file_content.lines().count()` and `diff_bytes` as `proposed_file_content.len()`. This measures the *entire* proposed content, not the actual diff. For file writes that replace existing content, this overstates the diff size.

**Impact**: False positives on the diff_size_limiter gate — policy may deny a write that changes 2 lines in a 500-line file because it measures all 500 lines. Acceptable as a conservative heuristic for now; accurate diff computation requires the original file content.

### P6-004 — Policy denial message includes Debug-formatted `rule_ids` (INFO)

`policy_preflight_response` line 205: `format!("{} {:?}", decision.reason, decision.rule_ids)` uses `{:?}` for rule IDs, producing output like `policy:deny:tool_allowlist:not_listed ["AL-1"]`. The Debug format includes brackets and quotes. For a client-facing wire message, a more structured approach (e.g., a `rule_ids` JSON field alongside `message`) would be cleaner.

### P6-005 — No benchmark for deny path (INFO)

SC-010 benchmark only exercises the allow path (`xray.scan` is in the allowlist). Deny-path evaluation (which triggers string matching, regex scanning, or set lookups that fail) may have different performance characteristics. Low risk since deny paths are generally faster (early return).

### P6-006 — `active_shard_scopes_from_env` reads env on every call (INFO)

`build_tool_call_context` calls `active_shard_scopes_from_env()` which reads `OPC_POLICY_ACTIVE_SHARDS` from the process environment on every tool call. Negligible cost, but the env read could be cached alongside the bundle for consistency.

## Summary

| Requirement | Status | Notes |
|---|---|---|
| SC-011 (dispatch order) | ✅ | grants → evaluate → handler, faithful to spec |
| Contract (wire codes) | ✅ | `POLICY_DENIED` vs `PERMISSION_DENIED` distinct |
| R-002 (fallback) | ✅ | Missing bundle skips policy, proceeds with 035-only |
| NF-003 (no kernel I/O) | ✅ | All inputs host-supplied via `build_tool_call_context` |
| SC-010 (kernel latency) | ✅ | ~0.9 µs/call, far below 5 ms budget |
| NF-002 (compile time) | ✅ | ~1.35 ms/compile, far below 2 s ceiling |
| verification.md | ✅ | Commands + results for SC-010, NF-002, SC-011 |

**Findings**: 6 total — 0 HIGH, 0 MEDIUM, 3 LOW (P6-001, P6-002, P6-003), 3 INFO (P6-004..P6-006). No blockers.

**047 Phase 6 approved. Feature 047 — Governance Control Plane — all 6 phases complete.**
