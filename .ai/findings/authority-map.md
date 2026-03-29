# Authority map (working notes)

> **Non-authoritative.** Enforced policy and product contracts live in `specs/` and code; this file is for alignment checks.

## Purpose

Document **where truth lives** for governance, registry, git context, and UI: **enforced** vs **displayed**, and who can override whom.

## Canonical references (read first)

- `specs/032-opc-inspect-governance-wiring-mvp/spec.md` (FR-002–FR-004, contract notes)
- Registry/consumer contracts: `specs/029-*` … `specs/031-*` as applicable
- `specs/032-opc-inspect-governance-wiring-mvp/execution/changeset.md`

## Authority matrix

| Concern | Source of truth (enforced) | UI / UX surface | Mismatch risk |
|---------|----------------------------|-----------------|---------------|
| Git branch/status | `git2` library via Tauri `git_*` commands (local repo) | `GitContextSurface` / `useGitContext` | **Low** — cleanly separated; gitctx MCP is explicitly additive per FR-002 |
| GitHub enrichment | `gitctx-mcp` binary via stdio MCP bridge | `useGitCtxEnrichment` overlay in `GitContextSurface` | **Low** — additive only, absence shows "unavailable" not error |
| Spec/feature registry | `build/spec-registry/registry.json` (compiled by `spec-compiler`, CI-gated) | `featuregraph_overview` → registry half → `GovernanceSurface` | **Low** — deterministic compilation, contract-tested |
| Code→feature attribution | `// Feature: UPPERCASE_ID` file headers → `featuregraph::Scanner::scan()` → requires `spec/features.yaml` | `featuregraph_overview` → featuregraph half → `GovernanceSurface` | **HIGH** — scanner depends on missing file; always returns unavailable; UPPERCASE IDs have no mapping to kebab-case spec IDs |
| Agent permissions | SQLite `agents` table: `enable_file_read`, `enable_file_write`, `enable_network` | Agent creation/edit UI shows toggles | **CRITICAL** — flags stored and displayed but **never enforced**; all execution uses `--dangerously-skip-permissions` |
| Safety tiers | `crates/agent/src/safety.rs` — Tier1/2/3 classification + `calculate_plan_tier()` | None (not surfaced in UI) | **HIGH** — tier system exists in code but is only consulted inside axiomregent, which is never spawned |
| Governed tool execution | `axiomregent` MCP router (`crates/axiomregent/src/router/mod.rs`) — `gov.preflight`, `gov.drift`, `snapshot.*`, `workspace.*` | None (sidecar never activated) | **CRITICAL** — the platform's core governance enforcement surface is compiled but not on the runtime path |
| Checkpoint/temporal state | `crates/titor/` library (production-grade, ~17k LOC) | None (5 of 6 Tauri commands are `todo!()`) | **HIGH** — capability exists, no desktop access |
| Registry consumer contracts | `tools/registry-consumer/` + CI contract tests (Feature 029) | CLI only; not surfaced in desktop UI | **Low** — contracts enforced via CI, not relevant to desktop runtime |
| Desktop UI state | React (Zustand/Context/localStorage) | Tab manager, settings, agent list | **Low** — no competing authority |

## Verified findings

- **Git authority is clean.** FR-002 explicitly states native git is source-of-truth; gitctx MCP is additive. Implementation matches: `useGitContext` is primary, `useGitCtxEnrichment` is optional overlay. Evidence: `apps/desktop/src/features/git/GitContextSurface.tsx`.
- **Registry authority is clean.** `read_registry_summary()` reads the compiler-emitted `registry.json`. Deterministic, CI-gated. Evidence: `apps/desktop/src-tauri/src/commands/analysis.rs:92-136`.
- **Featuregraph authority is broken.** `Scanner::scan()` reads `spec/features.yaml` which doesn't exist and is forbidden by Feature 000 (V-004). Evidence: `crates/featuregraph/src/scanner.rs:167`. The governance panel gracefully degrades but the featuregraph half cannot succeed.
- **Agent permission authority is illusory.** `enable_file_read/write/network` in SQLite are never translated into execution constraints. Evidence: `agents.rs:774` always passes `--dangerously-skip-permissions`; grep for `enable_file_read` shows only CRUD, never conditional gating.
- **axiomregent is the would-be enforcement authority, but it's dead.** `spawn_axiomregent()` in `sidecars.rs:48` is never called. Evidence: grep for `spawn_axiomregent` shows only its definition, no call site.

## Evidence (file references)

- `apps/desktop/src-tauri/src/commands/analysis.rs:16-73` — featuregraph_overview impl
- `apps/desktop/src-tauri/src/commands/analysis.rs:92-136` — read_registry_summary
- `crates/featuregraph/src/scanner.rs:166-168` — Scanner::scan() reads features.yaml
- `crates/featuregraph/src/tools.rs:26-36` — features_overview delegates to scanner
- `crates/agent/src/safety.rs:38-52` — tier classification (hardcoded match arms)
- `apps/desktop/src-tauri/src/commands/agents.rs:774` — --dangerously-skip-permissions
- `apps/desktop/src-tauri/src/commands/claude.rs:969` — --dangerously-skip-permissions
- `apps/desktop/src-tauri/src/sidecars.rs:48` — spawn_axiomregent definition (never called)
- `apps/desktop/src-tauri/src/lib.rs:188-189` — SidecarState managed but axiomregent never spawned

## Implications

- **For Feature 032 (current scope):** The authority map is adequate for the MVP inspect journey. Git and registry authorities are clean. Featuregraph degradation is bounded and explicit. The permission/execution enforcement gaps are real but out of scope per Feature 032's own spec (no cockpit, no control-plane modules).
- **For post-032 work:** The three CRITICAL/HIGH items (agent permissions, axiomregent activation, safety tier enforcement) are the same integration. Activating axiomregent and routing agent execution through it would resolve all three simultaneously.
- **Feature ID duality** is a design debt that will compound: every new feature adds entries in both systems with no cross-reference. A reconciliation strategy needs a spec before it becomes unmanageable.

## Candidate promotions

- [ ] Spec clarification in `spec.md` — note that featuregraph half of governance panel operates in degraded state for MVP; full graph requires either `spec/features.yaml` or scanner adaptation (post-032)
- [ ] `execution/changeset.md` — no change needed; PR-6 already records governance wiring with explicit degraded/unavailable handling
- [ ] Future spec candidate — "Safety tier model" spec to formalize `agent/safety.rs` tiers and make them governance-visible
- [ ] Future spec candidate — "axiomregent activation" spec to move governed tool execution from dead-code to runtime-path
- [ ] Future spec candidate — "Feature ID reconciliation" to bridge kebab-case spec IDs and UPPERCASE code attribution IDs
