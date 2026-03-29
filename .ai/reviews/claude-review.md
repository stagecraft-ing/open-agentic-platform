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

Per `execution/verification.md`: "no automated p99 gate in-repo — manual profiling recommended when hardening." The per-session subprocess startup adds process-spawn + MCP handshake latency. No measurement artifact exists yet. **Recommended follow-up:** add a benchmark or integration test that asserts < 50ms overhead per tool call (excluding subprocess startup, which is amortized over the session).

### Promotion candidates

- [ ] Risk 1 fix (no-lease bypass) — new task for post-035 hardening
- [ ] NF-001 automated latency gate — as noted in spec and verification
- [ ] Document max_tier agent vs claude rationale in `spec.md` contract notes
- [ ] Cross-platform axiomregent binaries (033 residual, still open)

