# Claude review (working notes)

> **Non-authoritative.** Deep-dive notes; cite files/lines when asserting behavior.

## Scope reviewed

- Feature / slice: Feature 032 (OPC inspect + governance wiring MVP), T000–T013 — **all complete**
- Code paths: Full Tauri backend (`apps/desktop/src-tauri/src/`), all 10 Rust crates, frontend features (`inspect/`, `git/`, `governance/`), spec system (`specs/`, `tools/`), `.ai/` workspace
- Post-T009 review: T010 "View spec" action (`actions.ts`, `RegistrySpecFollowUp.tsx`), `featureSummaries` backend extension (`analysis.rs:130-166`), vitest coverage, verification suite

## Main concerns

### 1. Governance is display-only (CRITICAL — post-032)

Every Claude/agent execution uses `--dangerously-skip-permissions` (`claude.rs:969`, `agents.rs:774`, `web_server.rs:494,607,695`). Agent permission flags (`enable_file_read/write/network`) in SQLite are stored and shown in UI but never translated into execution constraints. The governed execution thesis is not on the runtime path.

### 2. featuregraph scanner has a structural dependency on a nonexistent forbidden artifact

`Scanner::scan()` at `scanner.rs:167` reads `spec/features.yaml`. This file doesn't exist and is forbidden by Feature 000 (V-004). The governance panel gracefully handles this (`analysis.rs:55-58`), but the featuregraph half can never succeed on this repo.

### 3. axiomregent is the platform's most valuable integration and it's dead code

`spawn_axiomregent()` at `sidecars.rs:48` is fully implemented. `SidecarState` is managed at `lib.rs:189`. Port discovery works. The binary compiles. It is never called. This one missing function call represents the gap between "Claude wrapper" and "governed execution environment."

### 4. Dual feature identity systems with no bridge

Spec IDs (kebab: `032-opc-inspect-governance-wiring-mvp`) and code attribution IDs (UPPERCASE: `FEATUREGRAPH_REGISTRY`) coexist in the same governance response but cannot be cross-referenced.

## What appears resolved

- **Git authority is clean.** FR-002 is correctly implemented: native git is primary, gitctx MCP is additive. Well-separated in code (`useGitContext` vs `useGitCtxEnrichment`).
- **Registry authority is clean.** `read_registry_summary()` reads the deterministic, CI-gated `registry.json`. Contract-tested via Feature 029.
- **Inspect → enrich → display governance loop works.** T000–T009 delivered a real end-to-end inspect flow: xray scan, git context, governance status (degraded but explicit).
- **`.ai/` workspace is well-designed.** Non-authoritative, promotion-gated, baton-based. Follows the platform's own governance philosophy.
- **Degraded state handling is honest.** PR-6's governance implementation returns `{status: "degraded"}` with per-source reasons rather than hiding failures. This is good design.

## What still blocks convergence

### Feature 032: COMPLETE
- T010–T013 implemented by Cursor (2026-03-28). "View spec" action uses `featureSummaries` from compiled registry. Backend extended to emit `featureSummaries` (id, title, specPath) in `read_registry_summary`. `RegistrySpecFollowUp` component wired into both `InspectSurface` and `GovernanceSurface`. Vitest coverage added. Verification green.

### For post-032 platform thesis:
- **axiomregent activation** — needs its own spec
- **Agent permission enforcement** — needs axiomregent or alternative mechanism
- **Safety tier spec** — `safety.rs` tiers need to be spec-governed, not code-only
- **Feature ID reconciliation** — needs design decision before the two systems diverge further
- **featuregraph scanner fix** — adapt to use `registry.json` instead of `features.yaml`
- **Titor Tauri commands** — 5 stubs blocking temporal safety

## Recommended next move

**Feature 032 is done. Post-032 priorities (in order):**

1. **Spec: axiomregent activation** — write a feature spec for spawning axiomregent at startup and exposing its governed tool surface. This is the single move that transforms the platform from "Claude wrapper" to "governed execution environment."

2. **Spec: agent routing through axiomregent** — replace `--dangerously-skip-permissions` with governed dispatch. Make `enable_file_read/write/network` flags enforceable via safety tiers.

3. **Fix featuregraph scanner** — adapt `Scanner::scan()` to read from `registry.json` instead of the forbidden `spec/features.yaml`. This would immediately promote the governance panel from degraded to fully functional.

4. **Wire titor Tauri commands** — implement the 5 stubbed checkpoint commands. Enables temporal safety net for agent execution.

5. **Spec: safety tier model** — formalize `safety.rs` tier definitions. Make tier assignments governance-visible and spec-governed.

## Promotion candidates

- [x] `execution/verification.md` — governance backend tests and featuregraph degraded state documented (T013, 2026-03-28)
- [x] `execution/changeset.md` — T010–T013 recorded (2026-03-28)
- [x] `specs/033-axiomregent-activation/` — scaffolded (2026-03-29)
- [ ] Post-033 spec candidates — safety tier model, feature ID reconciliation, featuregraph scanner fix (Feature 034-class)

---

## Feature 033 review (2026-03-29)

### Spec assessment: `specs/033-axiomregent-activation/spec.md`

**Verdict: Spec is sound. Two issues to address before implementation.**

#### What's right

- **Scope is correctly bounded.** In-scope: spawn, port discovery, MCP UI visibility, safety tier display. Out-of-scope: agent rerouting, permission enforcement, scanner fix, titor. This is the right sequencing.
- **FR-001 through FR-004 are clear and testable.** Each has a verifiable condition.
- **Degraded state handling required (FR-002)** — consistent with 032's approach of explicit degradation rather than crashes.
- **Contract note about `tauri.conf.json` authority** is correct — `externalBin` already lists `binaries/axiomregent` (verified at `tauri.conf.json:61-63`).

#### Issue 1: axiomregent binary only exists for aarch64-apple-darwin

**Evidence:** `apps/desktop/src-tauri/binaries/` contains only `axiomregent-aarch64-apple-darwin`. No Windows (`x86_64-pc-windows-msvc`) or Linux binaries present.

**Impact:** FR-001 says "on supported builds" — which currently means only Apple Silicon macOS. On Windows/Linux, `spawn_axiomregent` will fail at `app.shell().sidecar("axiomregent")` because the binary doesn't exist.

**Recommendation:** T003 (packaging verification) should explicitly:
1. Document which targets have bundled binaries
2. For missing targets, either cross-compile axiomregent or ensure FR-002 degraded state works cleanly
3. Consider adding a `build:executables` step for axiomregent (similar to gitctx-mcp's `fetch-and-build.js` pattern at `apps/desktop/package.json`)

#### Issue 2: `spawn_axiomregent` uses `app.shell().sidecar()` which requires Tauri shell plugin

**Evidence:** `sidecars.rs:50` calls `app.shell().sidecar("axiomregent")`. The shell plugin is registered in `lib.rs` plugin chain. This should work, but the sidecar name must match the `externalBin` entry exactly (minus architecture suffix).

**Verification needed:** Confirm that Tauri 2's sidecar resolution correctly maps `"axiomregent"` to `binaries/axiomregent-{arch}` with the current `tauri.conf.json` externalBin config. This is a known Tauri 2 pattern but should be smoke-tested per T002.

#### Tasks assessment

- **T001–T002 (startup + smoke):** Correct sequencing. T001 should add `spawn_axiomregent(app)` after `SidecarState` management at `lib.rs:189`. T002 should verify port appears in `SidecarState`.
- **T003–T004 (packaging + verification):** Need to account for binary availability per platform. The gitctx-mcp approach (per-architecture bundled binary) is the pattern to follow.
- **T005–T006 (UI + safety tiers):** Well-scoped. T005 can use `get_sidecar_ports` which is already a Tauri command. T006 can read `safety.rs` tier definitions — but note these are only meaningful once axiomregent is actually dispatching tool calls.
- **T007–T008 (closure):** Standard.

### Feature 032 lifecycle status

**Confirmed: `status: active` is correct.** The registry enum (Feature 000/003) allows only `draft|active|superseded|retired`. There is no `implemented` value. Feature 032 remains `active` — it is current platform truth. Delivery is proven by `tasks.md` (all checked) + `execution/verification.md` (green run 2026-03-28).

### Recommendation (pre-implementation)

Feature 033 spec is ready for implementation with two additions:
1. Add a note to `spec.md` or `plan.md` acknowledging binary availability constraint (only macOS arm64 currently)
2. T003 should include cross-compilation or graceful degradation as explicit deliverables

### Post-implementation verification (2026-03-29)

**Verdict: Feature 033 is correctly implemented. All four FRs satisfied.**

| Requirement | Evidence | Status |
|-------------|----------|--------|
| FR-001: Start sidecar, record port | `lib.rs:190` calls `spawn_axiomregent`. `sidecars.rs:56-95` parses stderr. `main.rs` binds TCP probe + `eprintln!` | **Pass** |
| FR-002: Bounded degraded state | `sidecars.rs:60-63,68-69` logs and returns on failure. UI shows amber degraded message when port is `None` | **Pass** |
| FR-003: Operator-visible status | MCPManager shows "OPC axiomregent (bundled sidecar)" card with probe port or degraded. GovernanceSurface shows probe port + tier labels | **Pass** |
| FR-004: No registry-consumer changes | No changes to `tools/registry-consumer/` | **Pass** |

**Architecture notes:**
- **Probe port on stderr** is the right call — stdout reserved for MCP framing. `sidecars.rs:75` correctly matches `Stderr` first, `Stdout` as fallback.
- **Safety tier reference** (`analysis.rs:87-107`) is hardcoded labels matching `safety.rs` semantics (Tier1=Autonomous, Tier2=Gated, Tier3=Manual). These are read-only display — enforcement still only happens inside axiomregent's router (which is now running but not yet routing agent execution).
- **Binary constraint** documented in `verification.md`: only macOS arm64 binary present. Other platforms degrade gracefully.
- **Probe listener** (`main.rs`) binds `127.0.0.1:0` and holds the socket — diagnostics-only, no protocol. Clean pattern.

**Remaining gap (not 033 scope):** axiomregent is now live and visible, but agent execution still bypasses it (`--dangerously-skip-permissions`). This is the Feature 035-class work (agent routing through governed dispatch).

---

## Feature 034 review (2026-03-29)

### Spec-vs-implementation spot-check

**Verdict: Feature 034 is correctly implemented. All three FRs satisfied.**

| Requirement | Evidence | Status |
|-------------|----------|--------|
| FR-001: registry.json removes features.yaml dependency | `scanner.rs:170-186` — `load_feature_entries` checks `registry.json` first, returns immediately if found. Scanner never touches `features.yaml` when registry exists. | **Pass** |
| FR-002: Missing registry degrades explicitly | `scanner.rs:199-203` — `anyhow::bail!` with message naming both paths and instructing `spec-compiler compile`. Matches existing degraded patterns in GovernanceSurface. | **Pass** |
| FR-003: No regression to registry-consumer contracts | No files in `tools/registry-consumer/` modified. `registry_source.rs` is a new read-only consumer; `CompiledRegistry` deserializes the same shape spec-compiler emits. | **Pass** |

### Code quality notes

- **`registry_source.rs`** is minimal and correct. `#[serde(rename = "specPath")]` matches the actual registry JSON shape. Sort-by-id ensures deterministic output. Two unit tests cover parsing and ordering.
- **`FeatureEntry::from_registry_record`** (`scanner.rs:154-167`) maps registry records into the existing internal model cleanly. Empty defaults for `governance`, `owner`, `group`, `depends_on` are appropriate — these fields exist only in the legacy `features.yaml` schema and aren't present in the compiled registry.
- **`load_feature_entries`** (`scanner.rs:170-204`) has correct precedence: registry → yaml → explicit error. The backslash-to-forward-slash normalization (`replace('\\', "/")`) ensures Windows paths don't break the manifest path in violation messages.
- **`featuregraph_overview`** (`analysis.rs:17-20`) has updated doc comments citing the registry-first resolution. The command itself calls `FeatureGraphTools::new().features_overview()` which goes through `Scanner::scan()`, so the registry-first logic is picked up automatically.
- **Golden test** (`golden.rs`) and regenerated `features_graph.json` confirm the scanner produces valid output from the compiled registry.

### Concern addressed from prior review

Section 2 of the original review ("featuregraph scanner has a structural dependency on a nonexistent forbidden artifact") is now **resolved**. The scanner reads `build/spec-registry/registry.json` first, which exists after `spec-compiler compile`. The `spec/features.yaml` path is retained only as a fallback for repos that haven't migrated.

### Suggested fix message wording (minor)

`scanner.rs:275-276` suggests "Update the compiled registry / spec/features.yaml" for `MISSING_SPEC_FILE`. Now that registry is the primary source, consider leading with "Re-run `spec-compiler compile`" for consistency with the bail message at line 200. **Non-blocking.**

### Remaining open items (not 034 scope)

1. **Feature ID reconciliation** — spec IDs (kebab) vs code IDs (UPPERCASE) remain unbridged. Section 4 of original review still applies.
2. ~~**Agent execution bypass** — `--dangerously-skip-permissions` still in all agent paths. Feature 035-class.~~ **Resolved by Feature 035.**
3. **Titor command stubs** — still pending.
4. **Safety tier spec** — `safety.rs` tiers display-only, not yet spec-governed.

---

## Feature 035 review (2026-03-29)

### Verdict: Feature 035 is correctly implemented. All FRs satisfied. Two residual risks identified.

### FR assessment

| Requirement | Evidence | Status |
|-------------|----------|--------|
| FR-001: Tool calls routed through axiomregent MCP | `governed_claude.rs:95-107` — `GovernedPlan::Governed` adds `--mcp-config` + `--permission-mode default`; bypass removed from all 7 sites (`agents.rs:773-778`, `claude.rs:965-987,1012-1035,1062-1086`, `web_server.rs:488-501,605-619,699-714`) | **Pass** |
| FR-002: Router enforces tier + permission flags | `permissions.rs:56-79` — `check_tool_permission` checks `tier_rank(tool_tier) > max_allowed`, `requires_file_read/write/network` against `grants`. `router/mod.rs:489` calls `preflight_tool_permission` before every `tools/call` dispatch | **Pass** |
| FR-003: UI exposes permission toggles | `CreateAgent.tsx:327-347` — three Toggle switches for file_read, file_write, network. `api.ts` carries fields through `createAgent`/`updateAgent`. Defaults: read=true, write=true, network=false | **Pass** |
| FR-004: Degraded fallback with visible warning | `governed_claude.rs:103-105` — `Bypass` variant adds `--dangerously-skip-permissions`. `AgentExecution.tsx:563-574` and `ClaudeCodeSession.tsx:1502-1515` show amber "Bypass" badge when governance unavailable | **Pass** |
| FR-005: Structured audit log | `permissions.rs:84-100` — `audit_tool_dispatch` emits JSON to stderr with `op`, `tool`, `tier`, `decision`, `lease_id`, `ts`. Called on both allow and deny paths (`router/mod.rs:123-128,132-137`) | **Pass** |

### Architecture: MCP subprocess per session (correct design)

The implementation uses a **two-axiomregent** pattern:

1. **Sidecar axiomregent** (spawned at startup via `sidecars.rs`) — acts as a readiness probe. Its `announce_port` is checked in `plan_governed` (`governed_claude.rs:83`) to decide governed vs bypass.
2. **Per-session axiomregent** (spawned by Claude CLI via `--mcp-config`) — receives `OPC_GOVERNANCE_GRANTS` env with session-specific grants. `main.rs:36-38` reads this env into `PermissionGrants::from_env_or_default()` and creates the `LeaseStore` with those grants as defaults.

This is a good isolation model: each session gets its own grant scope, and the sidecar is only a health check for platform capability. The sidecar's own `LeaseStore` is irrelevant to governed sessions — it's the per-session subprocess that enforces permissions.

### Risk 1 (MEDIUM): No-lease tool calls bypass permission checks

`preflight_tool_permission` (`router/mod.rs:112-141`) uses the `?` operator on both `args.get("lease_id")` (line 118) and `self.lease_store.get_lease(lease_id)` (line 119). If either returns `None`, the function returns `None` — which means "no denial", and the tool executes **without any permission check**.

**Impact:** Any tool call without a `lease_id` argument silently bypasses all tier and permission enforcement. Many tools don't require `lease_id` in their schemas (e.g., `xray.scan`, `features.impact`, `gov.preflight`). A Claude CLI session could call these tools without a lease and skip governance entirely.

**Severity:** Medium. The practical attack surface is limited because:
- Claude CLI sends tool calls as the MCP client; users don't craft raw JSON-RPC.
- Read-only tools like `xray.scan` and `features.impact` are Tier 1 (autonomous) and would pass permission checks anyway.
- Write-path tools (`workspace.write_file`, `snapshot.create`) typically require `lease_id` for functional reasons (worktree mode).

**Recommended fix:** Default to `PermissionGrants::from_env_or_default()` (the session grants) when no lease is found, rather than silently allowing. This closes the bypass without requiring lease_id on every tool schema. Alternatively, audit-log the bypass so it's visible.

### Risk 2 (LOW): Agent max_tier=3 vs Claude default max_tier=2

`grants_json_for_agent` (`governed_claude.rs:46-53`) sets `max_tier: 3` for all agents, while `grants_json_claude_default` (`governed_claude.rs:55-63`) sets `max_tier: 2` for direct Claude sessions. This means agents can invoke Tier 3 (manual/dangerous) tools that interactive Claude sessions cannot.

**Rationale:** Agents have per-permission flags (`enable_file_read/write/network`) that individually constrain tool access, so Tier 3 tools are gated by those flags. Interactive Claude sessions have all permissions enabled but are tier-capped at 2 as a blanket safety measure.

**Assessment:** Defensible but non-obvious. The agent's explicit permission grants are the primary enforcement mechanism; `max_tier: 3` just means the tier ceiling doesn't redundantly block tools already gated by permission flags. Document this design decision in the spec's contract notes.

### Concern from prior review resolved

Section 1 ("Governance is display-only") of the original 032 review is now **fully resolved**. All 7 `--dangerously-skip-permissions` call sites are replaced with governed dispatch. Permission flags in SQLite are read at execution time (`agents.rs:774`, `governed_claude.rs:46`) and enforced by the per-session axiomregent subprocess.

Section 3 ("axiomregent is dead code") — also resolved. axiomregent is both live as a sidecar (033) and actively used for per-session governance (035).

### NF-001 (latency) status

~~Per `execution/verification.md`: "no automated p99 gate in-repo — manual profiling recommended when hardening."~~ **RESOLVED (Slice A).** `governed_dispatch_latency.rs` now measures permission check overhead: sub-microsecond per call across 10,000 iterations. Well within 50ms NF-001 budget. Note: this measures the enforcement overhead only — tool execution time is tool-specific and not governed by NF-001.

### Promotion candidates

- [x] Risk 1 fix (no-lease bypass) — **done** (Slice A)
- [x] NF-001 automated latency gate — **done** (Slice A)
- [x] Document max_tier agent vs claude rationale in `spec.md` contract notes — **done** (Slice A)
- [ ] Cross-platform axiomregent binaries (033 residual, still open)

---

## Slice A review (2026-03-29)

### Verdict: Slice A is correctly implemented. All 4 hardening tasks complete. One observation (non-blocking).

### Task assessment

| Task | Evidence | Status |
|------|----------|--------|
| Fix no-lease bypass | `router/mod.rs:112-175` — `preflight_tool_permission` now has two paths: lease-present (checks via `check_tool_permission`) and lease-absent (falls back to `lease_store.default_grants()` via `check_grants`). Both paths audit-log with distinct decision tags. | **Pass** |
| Document max_tier rationale | `spec.md:124-125` — two new contract notes explaining agent vs claude tier caps and no-lease fallback semantics | **Pass** |
| NF-001 benchmark | `tests/governed_dispatch_latency.rs` — 3 tests: `permission_check_under_50ms` (10k calls with lease), `permission_check_grants_fallback_under_50ms` (4k calls no-lease), `permission_denial_works` (correctness) | **Pass** |
| Scanner wording fix | `scanner.rs:274-276` — `suggested_fix` now leads with "Re-run `spec-compiler compile`" | **Pass** |

### Architecture: `check_grants` extraction (correct refactoring)

The factoring of `check_grants(tool_name, &grants)` out of `check_tool_permission(tool_name, &lease)` is clean. `check_tool_permission` is now a one-liner delegating to `check_grants(&lease.grants)`. This avoids duplicating the tier/permission logic across the lease and no-lease paths. The `pub` visibility on both functions is justified — `check_grants` is used directly in the no-lease fallback path, and the test file imports `permissions::check_grants`.

### Correctness of the no-lease fallback

The fix correctly closes Risk 1. Previously, `preflight_tool_permission` used `?` early-return on both `args.get("lease_id")` and `self.lease_store.get_lease(lease_id)`, meaning missing or unknown lease IDs silently bypassed all checks. Now:

1. **Missing `lease_id`**: `lease_id_str` is `None` → `lease` is `None` → falls into `None` arm → checks against `self.lease_store.default_grants()`.
2. **Invalid `lease_id`** (present but not in store): `lease_id_str` is `Some("...")` → `self.lease_store.get_lease(lid)` returns `None` → same fallback path.
3. **Valid `lease_id`**: normal lease-based enforcement, unchanged from Feature 035.

The default grants come from `LeaseStore::with_default_grants(PermissionGrants::from_env_or_default())` set in `main.rs:36-38`, which reads `OPC_GOVERNANCE_GRANTS` env (set by `governed_claude.rs:73` per session). This means the no-lease fallback inherits the **session-specific** grants, not a global permissive default. Correct.

### Audit log format

The four decision tags are well-chosen:
- `allowed` / `denied` — lease-based (existing, unchanged)
- `allowed_no_lease` / `denied_no_lease` — fallback path (new)

This makes it trivial to grep audit logs for no-lease events. The `lease_id` field is correctly `null` in the no-lease path.

### Observation (non-blocking): `mod permissions` is now `pub mod permissions`

Making the permissions module public was necessary for the integration test to import `permissions::check_grants` and `permissions::check_tool_permission`. This is acceptable — the module was already effectively public via its effects (JSON-RPC responses), and the functions have clear semantics. However, it does expand the crate's public API surface. If this becomes a concern, the test could alternatively be placed under `src/router/` as a `#[cfg(test)]` submodule. **No action needed now.**

### Remaining open items (not Slice A scope)

1. **Cross-platform axiomregent binaries** (Slice C) — still only macOS arm64
2. ~~**Safety tier governance spec** (Slice B) — tier definitions code-only~~ **Resolved by Feature 036.**
3. **Titor command stubs** (Slice D) — 5 stubs blocking temporal safety
4. **Feature ID reconciliation** (Slice E) — kebab vs UPPERCASE unbridged

---

## Feature 036 review (2026-03-29)

### Verdict: Feature 036 is correctly implemented. All FRs and SCs satisfied. Three minor cleanup items.

### FR assessment

| Requirement | Evidence | Status |
|-------------|----------|--------|
| FR-001: Every router tool has explicit tier | `safety.rs:41-71` — 14 Tier1, 6 Tier2 explicit entries; `run.execute`/`agent.execute` fall to catch-all (Tier3). All 21 router tools covered. | **Pass** |
| FR-002: Coverage test catches new unclassified tools | `tool_tier_coverage.rs:47-91` — `every_router_tool_has_explicit_tier` builds a real router, gets `tools/list`, checks every name against `explicitly_classified_tools()`. Fails with descriptive message if any tool is missing. | **Pass** |
| FR-003: UI shows per-tool tier assignments | `GovernanceSurface.tsx:77-91` — collapsible `<details>` section fetches `api.getToolTierAssignments()` and renders tool/tier pairs in monospaced grid. `analysis.rs:120-128` backend delegates to `explicitly_classified_tools()` + `get_tool_tier()`. | **Pass** |
| FR-004: Dual enums have distinct names | `safety.rs:9` → `ToolTier` (tool dispatch). `preflight.rs:38` → `ChangeTier` (change classification). Different crates, different semantics, clear naming. | **Pass** |

### Tier assignments verified

Every assignment matches the spec's proposed tier table:

- **Tier 1 (14 tools):** `gov.preflight`, `gov.drift`, `features.impact`, `snapshot.info`, `snapshot.list`, `snapshot.read`, `snapshot.grep`, `snapshot.diff`, `snapshot.changes`, `snapshot.export`, `xray.scan`, `run.status`, `run.logs`, `agent.verify` — all read-only/diagnostic. Correct.
- **Tier 2 (6 tools):** `workspace.apply_patch`, `workspace.write_file`, `workspace.delete`, `write_file` (legacy alias), `snapshot.create`, `agent.propose` — bounded mutations. Correct.
- **Tier 3 (2 explicit + catch-all):** `run.execute`, `agent.execute` — dangerous execution. Unknown tools also Tier3. Correct.

Verified by `tier_assignments_match_spec` test (`tool_tier_coverage.rs:94-132`).

### Spec contract note answers

1. **`snapshot.export` at Tier1** — Correct. Export reads existing snapshot data and marshals it into an output format. No workspace mutation occurs. The "file creation" is output, not a side effect.

2. **`write_file` legacy alias** — Confirmed still reachable via `internal_client.rs:98` (`"write_file" | "workspace.write_file" =>`). Keeping it in `get_tool_tier()` and `explicitly_classified_tools()` is justified. Note: `write_file` is NOT in the router's `tools/list` (only `workspace.write_file` is), so `explicitly_classified_tools()` has 22 entries while the router exposes 21 tools. The coverage test only checks router→classified direction, which is the critical direction. Benign.

3. **`requires_file_read/write/network` coverage** — All 21 router tools hit at least one permission flag. No gaps. `run.status` and `run.logs` are Tier1 but flagged `requires_network` — consistent with their `run.*` domain (command execution context). Defensible.

4. **Enum rename blast radius** — Contained as predicted. No external consumers of either old name. `permissions.rs:3` updated to `use agent::safety::{ToolTier, get_tool_tier}`. All `preflight.rs` references updated to `ChangeTier`.

### Minor cleanup items (non-blocking)

1. **Dead `Tier` alias can be removed** — `safety.rs:16-17` has `pub type Tier = ToolTier;` with comment "will be removed once all consumers migrate." Grep for `use.*safety::Tier\b` returns zero hits. No consumers exist. The alias is dead weight.

2. **Stale `bindings.ts` doc comment** — `bindings.ts:328` still says `featuregraph::preflight::SafetyTier` but the enum is now `ChangeTier`. This is auto-generated by specta; regenerating bindings would fix it. Cosmetic only — the type shape (`{id, label, description}`) is unchanged.

3. **Coverage test is one-directional** — `every_router_tool_has_explicit_tier` checks that every router tool is in the classified set, but not the reverse. Adding a ghost entry to `explicitly_classified_tools()` that doesn't exist in the router would silently pass. A reverse check would complete the coverage guarantee. Low priority — the critical direction (new router tool without classification) IS caught.

### Architecture quality

- **`explicitly_classified_tools()`** (`safety.rs:75-103`) is the right abstraction — a single source of truth for both the coverage test and the UI backend. Clean.
- **`get_tool_tier_assignments()`** (`analysis.rs:120-128`) correctly derives data from the authoritative source rather than hardcoding. No UI/backend divergence possible.
- **`GovernanceSurface.tsx`** correctly distinguishes `ChangeTier` (file changes) from `ToolTier` (MCP dispatch) in its inline documentation (`line 54-55`). Good developer communication.
- **`PreflightResponse.safety_tier`** field name was not changed to `change_tier` — this preserves API backward compatibility. The enum type name changed but the serialized field name stays stable. Correct.

### NF-001 (no regression)

No changes to enforcement logic in `permissions.rs` beyond the `Tier` → `ToolTier` import rename. `check_grants` and `check_tool_permission` are byte-for-byte identical to post-Slice-A state. Tier reclassifications only relax restrictions for read-only tools — sessions with `max_tier: 1` now correctly permit `snapshot.read`, `xray.scan`, etc. The permission flag layer (`enable_file_read`) still provides a second gate.

### Tasks T008–T009

`tasks.md` shows T008 (update verification.md) and T009 (spec-compiler compile) still unchecked. `verification.md` already has test evidence recorded from the implementation pass. T009 is a mechanical step. Neither blocks the review.

### Remaining open items (not Feature 036 scope)

1. ~~**Cross-platform axiomregent binaries** (Slice C) — still only macOS arm64~~ **Partially resolved by Feature 037** (Windows binary + CI workflow).
2. **Titor command stubs** (Slice D) — 5 stubs blocking temporal safety
3. **Feature ID reconciliation** (Slice E) — kebab vs UPPERCASE unbridged

---

## Feature 037 review (2026-03-29)

### Verdict: Feature 037 is correctly implemented for the Windows target. CI workflow and build script have minor issues. One stale doc comment found.

### FR assessment

| Requirement | Evidence | Status |
|-------------|----------|--------|
| FR-001: Binaries for all 5 targets | `binaries/` has macOS arm64 (pre-existing) + Windows x86_64 (new, 7.3 MB). macOS x86_64, Linux x86_64, Linux arm64 deferred to CI. | **Partial** (2/5 bundled; CI covers remaining 3) |
| FR-002: Sidecar spawn on macOS + Windows | Windows: binary starts, `OPC_AXIOMREGENT_PORT=49679` on stderr, MCP `initialize` handshake succeeds, `tools/list` returns all 21 tools. macOS: pre-existing. | **Pass** |
| FR-003: Build script produces binaries | `scripts/build-axiomregent.sh` — supports `--all`, specific triples, auto-detect host. Successfully built Windows binary. | **Pass** (with caveats — see below) |
| FR-004: Governed execution on Windows | Windows binary responds identically to macOS for MCP protocol. Full Tauri desktop e2e pending but protocol-level verification is complete. | **Pass** (protocol-level) |

### ToolTier fix in agent.rs: CORRECT

`agent.rs:8` updated from `{Tier, calculate_plan_tier}` to `{ToolTier, calculate_plan_tier}`. `agent.rs:115` updated `parse::<Tier>()` to `parse::<ToolTier>()`. Both changes are mechanical and correct. All 13 agent tests pass.

### Stale reference sweep (requested check #2)

**Comprehensive grep confirms only one remaining stale reference:**

- **`crates/axiomregent/src/snapshot/lease.rs:97`** — doc comment says `(see agent::safety::Tier)` but should say `agent::safety::ToolTier`. Non-blocking (doc comment only, no compilation impact).

All other imports, types, and references across the entire codebase correctly use `ToolTier` or `ChangeTier`. The Feature 036 wide pass + this 037 fix have cleaned up all functional references.

### T003/T004 deferral assessment (requested check #3)

**Deferral to CI is the correct call.** Rationale:

1. `rusqlite` with `bundled` feature and `zstd` both compile C source. Cross-compiling C from Windows to macOS/Linux requires platform-specific C toolchains that aren't available on Windows without complex setup (Docker, WSL, or specialized cross-compilers).
2. The CI workflow (`build-axiomregent.yml`) correctly handles this with a platform-native matrix: macOS runner for Apple targets, Ubuntu runner for Linux x86_64, Windows runner for Windows, and a separate cross-compile job for Linux arm64.
3. The most impactful deliverable — the Windows binary — is shipped. The dev team is on Windows, so this unblocks governed execution immediately.

### CI workflow issues (3 items)

#### Issue 1 (HIGH): Smoke test `timeout` command on macOS

`build-axiomregent.yml:73`:
```yaml
timeout 3 apps/desktop/src-tauri/binaries/${{ matrix.binary }} 2>&1 || true
```
The `timeout` command doesn't exist on macOS by default (it's a GNU coreutils command, not available in BSD userland). On macOS runners, this smoke test will fail with `timeout: command not found`. The `continue-on-error: true` suppresses the failure, so it won't block CI — but the test won't actually run on macOS.

**Fix:** Use a cross-platform alternative or conditional:
```bash
# Portable: kill after 3 seconds
( apps/desktop/src-tauri/binaries/${{ matrix.binary }} & PID=$!; sleep 3; kill $PID 2>/dev/null ) 2>&1 || true
```

#### Issue 2 (LOW): macOS x86_64 cross-compile from arm64 runner

`macos-latest` on GitHub Actions runs on Apple Silicon. Building `x86_64-apple-darwin` from an arm64 macOS runner works because Cargo + Rosetta 2 handle this transparently — but it's fragile if GitHub ever disables Rosetta on CI runners. **Non-blocking for now.**

#### Issue 3 (LOW): Missing cargo cache

No `Swatinem/rust-cache@v2` or equivalent. Every CI run recompiles all dependencies from scratch. With rusqlite + zstd C compilation, this adds significant build time. **Optimization, not correctness.**

### Build script issues (2 items)

#### Issue 1 (LOW): `--all` flag is unrealistic from a single host

`scripts/build-axiomregent.sh --all` will attempt all 5 targets but will fail for most due to missing cross-compilation toolchains. The script's own header comments acknowledge this ("On CI, prefer matrix builds"). Not a bug — the flag exists for CI runners that might have cross-tools installed — but could confuse a developer.

#### Issue 2 (LOW): No PowerShell alternative

The script is bash-only. On Windows, developers must use Git Bash. The script works for the `x86_64-pc-windows-msvc` target from Git Bash (verified by cursor's successful build), but the path handling (`pwd` → POSIX paths) is fragile. A `.ps1` alternative would be more natural on Windows. **Non-blocking — Git Bash works.**

### Binary size analysis

| Target | Size | NF-001 (< 30 MB) |
|--------|------|-------------------|
| `aarch64-apple-darwin` | 22.2 MB | Pass |
| `x86_64-pc-windows-msvc` | 7.3 MB | Pass |

The 3x size difference between macOS (22.2 MB) and Windows (7.3 MB) is surprising but explainable: the macOS binary likely includes debug symbols (not stripped), while MSVC release builds strip by default. Stripping the macOS binary would likely bring it closer to 8-10 MB. **Not a concern — both are well under the 30 MB cap.**

### Remaining open items

1. **Stale doc comment** `lease.rs:97` — `agent::safety::Tier` → `agent::safety::ToolTier`. Recommend antigravity fix during wide pass.
2. **CI smoke test** — fix `timeout` command for macOS portability. Recommend cursor fix.
3. **T003/T004** — macOS x86_64 and Linux binaries will be produced once CI workflow runs. Not a blocker.
4. **T009** — `spec-compiler compile` still pending.
5. **Titor command stubs** (Slice D) — ~~5 stubs blocking temporal safety~~ **resolved by Feature 038**
6. **Feature ID reconciliation** (Slice E) — kebab vs UPPERCASE unbridged

---

## Feature 038: titor Tauri command wiring

**Reviewed**: 2026-03-29 (claude)
**Scope**: `commands/titor.rs` (full file), `lib.rs:185` (manage), spec FR-001–FR-007, SC-001–SC-006

### FR/SC checklist

| Requirement | Status | Evidence |
|-------------|--------|----------|
| FR-001 (init, idempotent) | **Pass** | `get_or_init` double-check pattern at `titor.rs:39-61`; test at `:183-240` |
| FR-002 (checkpoint) | **Pass** | `titor_checkpoint` at `:112-122`; `guard.checkpoint(message)` delegates correctly |
| FR-003 (list) | **Pass** | `titor_list` at `:124-133`; serializes `Vec<Checkpoint>` to JSON |
| FR-004 (restore) | **Pass** | `titor_restore` at `:135-147`; discards `RestoreResult` (returns `()`) — acceptable per existing signature |
| FR-005 (diff) | **Pass** | `titor_diff` at `:149-160` |
| FR-006 (verify) | **Pass** | `titor_verify` at `:162-174`; `VerificationReport` serialized, `.is_valid()` asserted in test |
| FR-007 (error without init) | **Pass** | `require_titor` at `:64-71`; dedicated test `require_titor_errors_without_init` |
| SC-001–SC-005 (round-trip) | **Pass** | Single integration test covers full cycle |
| SC-006 (error path) | **Pass** | Tested at `:243-253` |
| NF-002 (non-blocking) | **Pass** | `tokio::sync::RwLock` outer + `tokio::sync::Mutex` inner — matches spec architecture |

### Architecture conformance

- **`TitorState` design**: matches spec exactly — `Arc<RwLock<HashMap<PathBuf, Arc<Mutex<Titor>>>>>`. Extra `storage_paths` map is a reasonable addition not in the spec but useful for idempotent return values.
- **`.manage(TitorState::new())`**: correctly placed at `lib.rs:185`, alongside `CheckpointState` and other managed states.
- **`canonical_root`**: good defensive canonicalization prevents duplicate instances for the same directory via different paths (e.g., symlinks, trailing slashes).
- **`build_titor`**: uses `CompressionStrategy::Adaptive { min_size: 4096, skip_extensions: vec![] }` — sensible defaults. The builder pattern matches the titor crate's API.
- **Thread safety**: Titor is `Send` (no SQLite — uses file-based content-addressable storage with `DashMap` caching). R-001 risk from spec does not materialize. `tokio::sync::Mutex` wrapping is correct.

### Finding: race in `get_or_init` (LOW)

There is a narrow race window between `instances.write()` drop at `:58` and `storage_paths.write()` at `:59-60`. If a concurrent `get_or_init` for the same root enters between these two points:

1. It reads `instances` → finds key present (`:41-42`)
2. It reads `storage_paths` → key not yet inserted → falls through
3. It acquires `instances.write()` (`:48`) → finds key present (`:49`)
4. It reads `storage_paths` (`:50-51`) → still missing → returns error at `:52-54`

**Impact**: transient "internal: missing storage path" error on concurrent init of the same root. Self-healing on retry. Unlikely in practice (init is called once per project root from UI).

**Fix**: insert into `storage_paths` while still holding the `instances` write lock, before `drop(map)`. Alternatively, merge both maps into a single `HashMap<PathBuf, (Arc<Mutex<Titor>>, PathBuf)>`.

### What's resolved from prior reviews

- **Item 5** (titor command stubs / Slice D) from the Feature 032 review is now fully addressed. All 5 `todo!()` stubs are replaced with working delegations.

### Compilation & tests

```
cargo check (desktop crate): OK (warnings only — pre-existing, unrelated)
cargo test titor: 4/4 passed (2 in lib, 2 in opc-web)
```

### Verdict

**Feature 038: PASS.** All FRs and SCs met. One low-severity race condition noted (non-blocking for promotion). Implementation is clean, follows established patterns, and the round-trip test is thorough.

---

## ADR 0001 review: Feature ID reconciliation (2026-03-29)

**Reviewed**: `docs/adr/0001-feature-id-reconciliation.md`
**Cross-referenced**: `registry.schema.json`, `scanner.rs`, `tools/spec-compiler/src/lib.rs`, `crates/featuregraph/src/registry_source.rs`, all registry consumers

### Verdict: ADR 0001 is SOUND. Option (a) is the correct choice. Four gaps must be addressed in Feature 039 tasks before implementation.

### Decision assessment

The ADR correctly identifies the problem (two parallel ID systems with no bridge in compiled output) and selects the least-disruptive solution. Options (b) and (c) are rightly rejected — derivation from kebab slugs is lossy and many-to-one, and mass header churn has high blast radius for marginal gain.

### Gap 1 (HIGH): Schema bump strategy undefined

Adding `codeAliases` to `featureRecord` in `registry.schema.json` is a **breaking change** for strict consumers that use `"additionalProperties": false` (line 50). The schema currently enforces a closed set of properties — any JSON with `codeAliases` will **fail** schema validation until the schema is updated. The ADR says "Schema and compiler must be extended" but does not specify:

- Whether `specVersion` should be bumped (e.g. `0.1.0` → `0.2.0`) to signal the new field.
- Whether consumers should be given a migration window where `codeAliases` is absent vs present.
- The schema conformance tests (`tools/spec-compiler/tests/schema_conformance.rs`) validate output against the JSON Schema — these will break the moment the compiler emits the field without a corresponding schema update.

**Required ADR edit:** Add a "Schema versioning" subsection to Consequences specifying: (1) `specVersion` bump policy, (2) `codeAliases` is optional (omitted when empty, not `null` or `[]`), (3) schema and compiler must be updated atomically in the same commit.

### Gap 2 (MEDIUM): Validation rules for alias uniqueness only partially specified

The ADR says "each alias appears under at most one feature" but does not define the enforcement mechanism:

- **When**: compile-time only, or also runtime in featuregraph scanner?
- **Violation code**: should be a new `V-00x` in the registry schema's `violation` definition, or a scanner-level `DUPLICATE_ALIAS` violation?
- **Severity**: error (blocks compilation) or warning (advisory)?
- **Orphaned aliases**: the ADR mentions "orphaned mappings are forbidden by policy" — but an alias in frontmatter that no code file uses is harmless (the feature may not have code yet). This should be a warning, not an error.

**Required ADR edit:** Add a "Validation rules" subsection specifying: alias-uniqueness is a compile-time error (`V-005`), orphaned aliases are a warning (`V-006`), and scanner emits `DANGLING_FEATURE_ID` for code tokens not in any feature's `codeAliases`.

### Gap 3 (MEDIUM): `RegistryFeatureRecord` consumer contract

`crates/featuregraph/src/registry_source.rs` currently deserializes only 4 fields (`id`, `title`, `specPath`, `status`). The scanner in `scanner.rs:154-166` constructs `FeatureEntry` from these records with `aliases: Vec::new()` — meaning the **new `codeAliases` field will be silently ignored** when loading from the compiled registry.

This creates a subtle regression: legacy `features.yaml` path (`scanner.rs:188-193`) reads aliases from YAML entries, but the preferred compiled-registry path (`scanner.rs:174-185`) drops them. After ADR 0001 is implemented, the scanner must be updated to:

1. Deserialize `codeAliases` from `RegistryFeatureRecord` (add optional field with `#[serde(default)]`).
2. Map `codeAliases` → `FeatureEntry.aliases` in `from_registry_record()` (`scanner.rs:154`).

**Required ADR edit:** Add to "Consumers" consequence: "featuregraph scanner MUST populate `FeatureEntry.aliases` from `codeAliases` when loading from compiled registry, closing the current gap where the registry path produces empty alias maps."

### Gap 4 (LOW): Population strategy ordering and conflict resolution

The ADR lists two sources: (a) spec frontmatter, (b) scanner-derived attribution. It says "merge alias sets" but doesn't define:

- **Priority**: if frontmatter declares `codeAliases: [FOO]` but scanner finds `// Feature: BAR` in files under `specs/039-*/`, which wins?
- **Directionality**: does the scanner feed aliases *into* the compiler (compile-time enrichment), or is this a separate post-compilation step?

The simplest correct answer: **frontmatter is authoritative at compile time; scanner validates at scan time.** The compiler reads `codeAliases` from frontmatter only. The scanner then validates that scanned code tokens match declared aliases. This avoids circular dependencies (scanner can't run before compilation, compilation can't depend on scanner output).

**Recommended ADR edit:** Clarify in the Decision section that population at compile time is frontmatter-only; scanner-derived enrichment is a future enhancement, not part of the initial merge strategy.

### Minor observations (non-blocking)

1. **Token pattern mismatch**: ADR says `[A-Z][A-Z0-9_]{2,63}` matching scanner's `FEATURE_REGEX`. The schema should use the same pattern for the `codeAliases` items constraint. Verified: scanner regex at `scanner.rs:34` is `^(//|#)\s*Feature:\s*([A-Z][A-Z0-9_]{2,63})\s*$` — the capture group `[A-Z][A-Z0-9_]{2,63}` should be the schema item pattern.

2. **Sorted lexicographically**: ADR correctly specifies this. The compiler already sorts `sectionHeadings` and other arrays. Consistent.

3. **`extraFrontmatter` alternative**: The ADR could technically use `extraFrontmatter` to carry aliases without a schema change (since it allows string arrays). However, this would be incorrect — `codeAliases` has validation semantics (uniqueness, pattern) that `extraFrontmatter` cannot enforce. A dedicated field is the right call.

### Remaining open items from prior reviews

1. **Cross-platform axiomregent binaries** — Windows delivered (037), others pending CI
2. **Feature ID reconciliation** — ADR 0001 reviewed here; implementation in Feature 039

### Promotion candidates

- [x] ADR 0001 edits: schema bump strategy, validation rules, consumer contract, population ordering (4 gaps above) — **all 4 closed by Feature 039**
- [x] Feature 039 task list should include: schema update, compiler extension, scanner alias bridging, conformance tests — **delivered**

---

## Feature 039 review: codeAliases / Feature ID reconciliation (2026-03-29)

**Reviewed**: `specs/039-feature-id-reconciliation/spec.md`, `docs/adr/0001-feature-id-reconciliation.md` (post-implementation), `tools/spec-compiler/src/lib.rs`, `crates/featuregraph/src/registry_source.rs`, `crates/featuregraph/src/scanner.rs`, `specs/000-bootstrap-spec-system/contracts/registry.schema.json`, `tools/spec-compiler/tests/code_aliases.rs`, `specs/039-feature-id-reconciliation/execution/verification.md`

### Verdict: Feature 039 is correctly implemented. All FRs and SCs satisfied. All 4 ADR gaps closed. Two minor items noted (non-blocking).

### FR/SC assessment

| Requirement | Evidence | Status |
|-------------|----------|--------|
| FR-001: `codeAliases` in schema | `registry.schema.json:81-88` — optional `codeAliases` array on `featureRecord`, items pattern `^[A-Z][A-Z0-9_]{2,63}$`, `uniqueItems: true`. Not in `required`. | **Pass** |
| FR-002: Compiler reads `code_aliases` from frontmatter, emits sorted `codeAliases`, omits when absent | `lib.rs:24` (`KNOWN_KEYS`), `lib.rs:150-157` (`parse_code_aliases`), `lib.rs:228` (`skip_serializing_if`), `lib.rs:610-616` (empty→None). Sorting at `lib.rs:614`. | **Pass** |
| FR-003: V-005 duplicate alias across features | `lib.rs:580-596` — cross-feature collision check via `alias_owner` map. Test: `code_aliases.rs:14-61`. | **Pass** |
| FR-004: V-006 invalid pattern warning | `lib.rs:566-576` — `is_valid_code_alias` check. Invalid entries omitted from output. Test: `code_aliases.rs:63-97`. | **Pass** |
| FR-005: `RegistryFeatureRecord` deserializes `codeAliases` | `registry_source.rs:25-26` — `#[serde(rename = "codeAliases", default)]`. Test: `registry_source.rs:101-133`. | **Pass** |
| FR-006: Scanner resolves `// Feature: TOKEN` via `codeAliases` | `scanner.rs:164` — `aliases: r.code_aliases.clone()`. `scanner.rs:259-261` — alias_map populated from `entry.aliases`. Test: `scanner.rs:470-479`. | **Pass** |
| FR-007: Schema conformance + golden determinism | `repo_spec_version_is_1_1_0` test. `code_aliases_sorted_deterministically` test. Golden graph updated. | **Pass** |
| NF-001: No overhead when absent | `parse_code_aliases` returns `Ok(None)` immediately when key missing (`lib.rs:537-539`). Zero allocation. | **Pass** |
| NF-002: `specVersion` 1.1.0 | `lib.rs:12` — `SPEC_VERSION: &str = "1.1.0"`. Test confirms. | **Pass** |
| SC-001–SC-006 | All confirmed in `execution/verification.md` with commands. Cross-checked against test suite (15 tests pass). | **Pass** |

### ADR 0001 gap closure

All 4 gaps identified in the prior ADR review are now addressed in the accepted ADR:

| Gap | Resolution |
|-----|-----------|
| Gap 1 (schema bump) | ADR §"Schema versioning" — `specVersion` 1.1.0 minor bump, `codeAliases` optional/omit-when-empty, atomic commit. |
| Gap 2 (validation rules) | ADR §"Validation rules" — V-005 (error, cross-feature dup), V-006 (warning, pattern mismatch). Note: orphan alias policy relaxed vs. original review suggestion — orphaned aliases are acceptable per contract notes. Correct call: features may not have code yet. |
| Gap 3 (consumer contract) | ADR §"Consumer contract" — explicit requirement that `RegistryFeatureRecord` deserializes `codeAliases` and `FeatureEntry.aliases` is populated. Implemented. |
| Gap 4 (population ordering) | ADR §"Population ordering" — frontmatter-only at compile time, scanner validates at scan time. No circular dependency. |

### Frontmatter population verified

All 6 specs with code attribution tokens have `code_aliases` populated:

- `032` → `XRAY_ANALYSIS`, `XRAY_SCAN_POLICY`
- `033` → `MCP_ROUTER`, `MCP_ROUTER_CONTRACT`, `MCP_SNAPSHOT_WORKSPACE`, `MCP_TOOLS`
- `034` → `FEATUREGRAPH_REGISTRY`, `GOVERNANCE_ENGINE`
- `035` → `AGENT_AUTOMATION`
- `004` → `TASK_RUNNER`
- `005` → `VERIFICATION_SKILLS`, `VERIFY_PROTOCOL`

**No orphaned code headers.** Every `// Feature: TOKEN` in `crates/` and `apps/` has a matching `code_aliases` entry in some spec. Complete coverage.

### Minor item 1 (LOW): V-005 second violation message is inverted

`lib.rs:591-593`: the second V-005 violation says `"code alias {s:?} is already claimed by feature {feature_id:?} (first occurrence)"` and attaches `prev_path`. But `feature_id` is the *second* claimant, and `prev_path` points to the *first*. The message names the wrong feature for the path it's annotating. Compare the first violation at `:585` which correctly pairs `prev_id` with the current `spec_path`.

**Suggested fix:** change `:592` to `format!("code alias {s:?} conflicts — also declared by feature {prev_id:?}")` or swap to name `prev_id` since the path is `prev_path`. **Non-blocking** — both violations fire together, so the user sees both paths and both IDs regardless.

### Minor item 2 (LOW): `language` key in 039 frontmatter goes to `extraFrontmatter`

`specs/039-feature-id-reconciliation/spec.md:10` has `language: en`. This isn't in `KNOWN_KEYS` (`lib.rs:15-25`), so it flows into `extraFrontmatter` in the compiled registry. This is true for any spec that uses `language:`. Not a bug — `extraFrontmatter` is the designed catchall — but if `language` becomes standard across specs, it should be promoted to `KNOWN_KEYS`.

### What's resolved from prior reviews

**Section 4 of the original 032 review** ("Dual feature identity systems with no bridge") is now **fully resolved**. The `codeAliases` field in `registry.json` bridges spec IDs and code tokens. The featuregraph scanner resolves `// Feature: TOKEN` → canonical kebab ID via the compiled registry's `codeAliases` arrays.

This was the last remaining open item from the original 032 review. All 4 original concerns are now closed:
1. ~~Governance is display-only~~ → resolved by Feature 035
2. ~~Scanner depends on nonexistent artifact~~ → resolved by Feature 034
3. ~~axiomregent is dead code~~ → resolved by Feature 033
4. ~~Dual feature identity with no bridge~~ → resolved by Feature 039

### Remaining open items (not Feature 039 scope)

1. **Cross-platform axiomregent binaries** — Windows delivered (037), macOS x86_64 + Linux pending CI
2. **Stale doc comment** `lease.rs:97` — `agent::safety::Tier` → `agent::safety::ToolTier` (carried from 037 review)
3. **CI smoke test** — `timeout` command portability on macOS (carried from 037 review)

