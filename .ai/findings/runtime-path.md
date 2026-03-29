# Runtime path (working notes)

> **Non-authoritative.** Verified facts belong in `specs/.../execution/changeset.md` and tests after promotion.

## Purpose

Trace **actual** execution from entrypoints (CLI/UI/commands) through IPC, sidecar, MCP, and filesystem — including **dead or partial** branches.

## Canonical references (read first)

- Active feature: `specs/032-opc-inspect-governance-wiring-mvp/spec.md`, `tasks.md`
- Recorded changes: `specs/032-opc-inspect-governance-wiring-mvp/execution/changeset.md`

## Verified path (source-grounded)

| Step | What runs | Evidence (file:symbol or path) |
|------|-----------|----------------------------------|
| App startup | Tauri init → SQLite (`agents.db`) → CheckpointState → ProcessRegistry → SidecarState(None) → QuickPane | `apps/desktop/src-tauri/src/lib.rs:109-195` |
| Inspect (xray) | `invoke("xray_scan_project")` → `xray::scan_target()` → deterministic JSON index | `apps/desktop/src-tauri/src/commands/analysis.rs:9-12` |
| Git context (native) | `invoke("git_context")` → `git2` library → branch/status/ahead-behind | `apps/desktop/src/features/git/useGitContext.ts` → Tauri `git_*` commands |
| Git context (enrichment) | `createMcpClient("gitctx")` → `invoke("mcp_call_tool")` → spawn `gitctx-mcp` binary via stdio → MCP JSON-RPC → kill | `apps/desktop/src-tauri/src/commands/mcp.rs:19-27,121-184` |
| Governance (registry half) | `invoke("featuregraph_overview")` → `read_registry_summary(build/spec-registry/registry.json)` → feature count, status counts, validation | `apps/desktop/src-tauri/src/commands/analysis.rs:18-31,92-136` |
| Governance (featuregraph half) | `FeatureGraphTools::features_overview()` → `Scanner::scan()` → `File::open("spec/features.yaml")` → **file not found** → returns `Err` → graceful degradation | `crates/featuregraph/src/tools.rs:26-36` → `crates/featuregraph/src/scanner.rs:167-168` |
| Governance (composite) | Returns `{status: "degraded", registry: {status: "ok"}, featuregraph: {status: "unavailable"}}` | `apps/desktop/src-tauri/src/commands/analysis.rs:61-72` |
| Governance (featureSummaries) | `read_registry_summary` now emits `featureSummaries` array (id, title, specPath) from registry features that have a `specPath` | `apps/desktop/src-tauri/src/commands/analysis.rs:130-166` |
| **View spec action (T010)** | `RegistrySpecFollowUp` component renders "View spec" buttons per feature; `resolveSpecAbsolutePath(repoRoot, specPath)` builds absolute path; `onViewSpec` opens `claude-md` tab with `specMarkdownAbsolutePath` | `apps/desktop/src/features/inspect/RegistrySpecFollowUp.tsx`, `actions.ts` |
| View spec surfaces | Wired into both `InspectSurface.tsx` (after successful scan + governance fetch) and `GovernanceSurface.tsx` | grep confirms 7 file references |
| Claude execution | `invoke("execute_claude_code")` → find binary → spawn `claude --dangerously-skip-permissions -p ...` → stream JSONL via Tauri events | `apps/desktop/src-tauri/src/commands/claude.rs:947-974` |
| Agent execution | `invoke("execute_agent")` → read SQLite → write `.claude/settings.json` (hooks) → spawn `claude --dangerously-skip-permissions ...` → stream JSONL | `apps/desktop/src-tauri/src/commands/agents.rs:681-792` |
| Call graph | `invoke("stackwalk_index")` → stackwalk parser → call graph JSON | `apps/desktop/src-tauri/src/commands/analysis.rs` (via stackwalk crate) |

## Inferred / needs verification

- `featuregraph_impact` (`analysis.rs:76-82`) is implemented and callable but untested against real repo state — it will also fail on `spec/features.yaml` missing.
- Checkpoint tracking (`checkpoint/manager.rs`) monitors JSONL tool_use blocks during Claude sessions but does NOT create Titor snapshots — it's observation-only journaling to `~/.claude/checkpoints/`.

## Gaps (dead paths, stubs, partial integration)

| Gap | Location | Impact |
|-----|----------|--------|
| ~~axiomregent never spawned~~ | ~~`sidecars.rs:48` defined; never called~~ | **RESOLVED (Feature 033)**: `lib.rs:190` now calls `spawn_axiomregent(app.handle())`. Sidecar starts, announces probe port on stderr, UI surfaces status in MCP manager + governance panel. Governed tool surface is live but not yet routing agent execution. |
| **Titor Tauri commands** | `commands/titor.rs` — 5 of 6 commands are `todo!()` | No checkpoint/restore/diff/verify from desktop; library is production-grade underneath |
| **Blockoli Tauri commands** | `commands/search.rs` — both commands `todo!()` | Semantic search tab renders but cannot function |
| **`--dangerously-skip-permissions` hardcoded** | `claude.rs:969,1001,1036`, `agents.rs:774`, `web_server.rs:494,607,695` | All execution bypasses Claude's permission system; agent DB permission flags (`enable_file_read/write/network`) are stored but never enforced |
| **`spec/features.yaml` does not exist** | `crates/featuregraph/src/scanner.rs:167` expects it | featuregraph scanner always fails → governance panel permanently degraded |
| **Feature ID duality** | Spec IDs: `032-opc-inspect-governance-wiring-mvp` (kebab). Code headers: `FEATUREGRAPH_REGISTRY` (UPPERCASE). No mapping exists. | Registry and featuregraph data cannot be cross-referenced |

## Implications

- The **inspect → git → governance display → follow-up action** loop is real and complete (T000–T013). Governance degrades gracefully rather than crashing. "View spec" action closes the loop from inspect to spec review.
- The **governed execution** loop does not exist at runtime. axiomregent has the tools; the desktop app doesn't start it.
- The gap between "show governance" and "enforce governance" is the platform's biggest structural debt.
- Feature 032 is **implemented** — all tasks complete, verification green.

## Candidate promotions

- [x] `execution/changeset.md` — T010–T013 recorded (2026-03-28)
- [x] `execution/verification.md` — T013 full verification recorded green (desktop build/test, cargo check, analysis tests, consumer tests, spec-compiler compile)
- [x] `spec.md` / `verification.md` — featuregraph degraded state documented as expected bounded behavior (FR-003)
- [ ] Future spec — safety tier model and axiomregent activation are post-032 work items that need specs
