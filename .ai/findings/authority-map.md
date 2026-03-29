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
| Spec/feature registry | `build/spec-registry/registry.json` (compiled by `spec-compiler`, CI-gated) | `featuregraph_overview` → registry half (incl. `featureSummaries`) → `GovernanceSurface` + `InspectSurface` → `RegistrySpecFollowUp` "View spec" buttons | **Low** — deterministic compilation, contract-tested; `featureSummaries` emitted server-side at `analysis.rs:130-166` |
| Code→feature attribution | `// Feature: UPPERCASE_ID` file headers → `featuregraph::Scanner::scan()` → reads `registry.json` (with `codeAliases`) first | `featuregraph_overview` → featuregraph half → `GovernanceSurface` | **RESOLVED (Feature 039)** — `codeAliases` in registry.schema.json (1.1.0), spec-compiler emits from frontmatter, scanner resolves aliases via `FeatureEntry.aliases`. All 12 code tokens across 6 specs bridged. Zero orphan headers. |
| Agent permissions | SQLite `agents` table: `enable_file_read`, `enable_file_write`, `enable_network` | Agent creation/edit UI shows toggles | **RESOLVED (035+Slice A)** — flags enforced at runtime via per-session axiomregent subprocess. No-lease fallback now checks session default grants (Slice A hardening). |
| Safety tiers | `crates/agent/src/safety.rs` — ToolTier classification + `get_tool_tier()` + `explicitly_classified_tools()` | GovernanceSurface shows per-tool tier assignments from backend | **RESOLVED (036)** — all 21 router tools explicitly classified, dual enums renamed (ToolTier/ChangeTier), coverage test enforces classification, UI derives from authoritative source. |
| Governed tool execution | `axiomregent` MCP router (`crates/axiomregent/src/router/mod.rs`) — `gov.preflight`, `gov.drift`, `snapshot.*`, `workspace.*` | MCPManager shows sidecar status + probe port; GovernanceSurface shows safety tier labels | **RESOLVED (033+035)** — sidecar alive (033), agent execution routed through governed dispatch (035). All 7 `--dangerously-skip-permissions` sites replaced. |
| Checkpoint/temporal state | `crates/titor/` library (production-grade, ~17k LOC) + `TitorState` in Tauri `AppState` | All 6 Tauri commands wired via `TitorState` (`commands/titor.rs`) | **RESOLVED (Feature 038)** — `TitorState` created, all 5 stubs replaced, `titor_init` persists instance. Round-trip verified. |
| Registry consumer contracts | `tools/registry-consumer/` + CI contract tests (Feature 029) | CLI only; not surfaced in desktop UI | **Low** — contracts enforced via CI, not relevant to desktop runtime |
| Semantic code search | `crates/blockoli/` library (fastembed, KD-tree, SQLite) + `BlockoliState` in Tauri managed state | `SemanticSearchPanel.tsx` → `blockoli_index_project` / `blockoli_search` Tauri commands | **RESOLVED (Feature 040)** — `BlockoliState` wired, both commands implemented, app-data SQLite, embedded asterisk config. Build green, all FRs/SCs pass. |
| Checkpoint/restore UI | `crates/titor/` library + `TitorState` in Tauri `AppState` + 6 Tauri commands (`commands/titor.rs`) | `CheckpointSurface.tsx` → `useCheckpointFlow.ts` → `titor_init` / `titor_checkpoint` / `titor_list` / `titor_restore` / `titor_diff` / `titor_verify` | **RESOLVED (Feature 041)** — Singleton `checkpoint` tab, `CheckpointSurface` exposes all 6 titor commands, project-scoped. All 8 FRs pass, all 6 SCs pass. `tsc --noEmit` green. |
| Desktop UI state | React (Zustand/Context/localStorage) | Tab manager, settings, agent list | **Low** — no competing authority |

## Verified findings

- **Git authority is clean.** FR-002 explicitly states native git is source-of-truth; gitctx MCP is additive. Implementation matches: `useGitContext` is primary, `useGitCtxEnrichment` is optional overlay. Evidence: `apps/desktop/src/features/git/GitContextSurface.tsx`.
- **Registry authority is clean.** `read_registry_summary()` reads the compiler-emitted `registry.json`. Deterministic, CI-gated. Evidence: `apps/desktop/src-tauri/src/commands/analysis.rs:92-136`.
- ~~**Featuregraph authority is broken.**~~ **RESOLVED (Feature 034).** `Scanner::scan()` now reads `registry.json` as primary source (`scanner.rs:170-186`). Falls back to `features.yaml` for legacy repos.
- ~~**Agent permission authority is illusory.**~~ **RESOLVED (Feature 035+Slice A).** All 7 `--dangerously-skip-permissions` sites replaced with governed dispatch. Permission flags enforced by per-session axiomregent subprocess via `OPC_GOVERNANCE_GRANTS` env. No-lease fallback now checks session default grants (Slice A).
- ~~axiomregent is dead~~ **RESOLVED (Features 033+035).** Sidecar alive at startup (033). Agent execution routed through governed axiomregent MCP subprocess (035).
- ~~**Safety tier model is code-only.**~~ **RESOLVED (Feature 036).** All 21 router tools explicitly classified in `get_tool_tier()`. Dual enums renamed: `ToolTier` (tool dispatch) / `ChangeTier` (change classification). Coverage test enforces explicit classification. UI derives per-tool tier assignments from `explicitly_classified_tools()`. Spec: `specs/036-safety-tier-governance/spec.md`.

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

- **Feature 032 is complete.** The authority map is sound for the delivered inspect journey. Git and registry authorities are clean. Featuregraph degradation is bounded and explicit. The "View spec" action (T010) uses registry `specPath` — a clean, compiler-owned authority — avoiding the broken `features.yaml` path entirely. The permission/execution enforcement gaps are real but out of scope per Feature 032's own spec (no cockpit, no control-plane modules).
- **For post-041 work:** All CRITICAL/HIGH/MEDIUM items are now **RESOLVED** (Features 032–041). The governance stack is complete (033–036), cross-platform (037), temporal safety wired (038) and surfaced in UI (041), the dual identity system bridged (039), semantic code search wired (040), and checkpoint/restore UI live (041). The remaining LOW items are CI runner targets (037 T003/T004) and two minor code-level cleanups (V-005 message wording, `lease.rs:97` stale doc comment).

## Candidate promotions

- [ ] Spec clarification in `spec.md` — note that featuregraph half of governance panel operates in degraded state for MVP; full graph requires either `spec/features.yaml` or scanner adaptation (post-032)
- [ ] `execution/changeset.md` — no change needed; PR-6 already records governance wiring with explicit degraded/unavailable handling
- [x] ~~Future spec candidate — "Safety tier model"~~ **DELIVERED (Feature 036)**
- [x] ~~Future spec candidate — "axiomregent activation"~~ **DELIVERED (Feature 033)**
- [x] ~~Future spec candidate — "Feature ID reconciliation"~~ **DELIVERED (Feature 039)** — ADR 0001 accepted, schema 1.1.0, compiler+scanner+frontmatter, all verified
- [x] ~~Future spec candidate — "Cross-platform axiomregent binaries"~~ **DELIVERED (Feature 037)**
- [x] ~~Future spec candidate — "Titor command wiring"~~ **DELIVERED (Feature 038)**
- [x] ~~Future spec candidate — "Blockoli semantic search wiring"~~ **DELIVERED (Feature 040)**
- [x] ~~Future spec candidate — "Checkpoint/restore UI"~~ **DELIVERED (Feature 041)**
