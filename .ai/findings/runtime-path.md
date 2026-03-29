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
| Claude execution | `invoke("execute_claude_code")` → find binary → spawn `claude --dangerously-skip-permissions -p ...` → stream JSONL via Tauri events | `apps/desktop/src-tauri/src/commands/claude.rs:947-974` |
| Agent execution | `invoke("execute_agent")` → read SQLite → write `.claude/settings.json` (hooks) → spawn `claude --dangerously-skip-permissions ...` → stream JSONL | `apps/desktop/src-tauri/src/commands/agents.rs:681-792` |
| Call graph | `invoke("stackwalk_index")` → stackwalk parser → call graph JSON | `apps/desktop/src-tauri/src/commands/analysis.rs` (via stackwalk crate) |

## Inferred / needs verification

- `featuregraph_impact` (`analysis.rs:76-82`) is implemented and callable but untested against real repo state — it will also fail on `spec/features.yaml` missing.
- Checkpoint tracking (`checkpoint/manager.rs`) monitors JSONL tool_use blocks during Claude sessions but does NOT create Titor snapshots — it's observation-only journaling to `~/.claude/checkpoints/`.

## Gaps (dead paths, stubs, partial integration)

| Gap | Location | Impact |
|-----|----------|--------|
| **axiomregent never spawned** | `sidecars.rs:48` `spawn_axiomregent()` defined; never called from `lib.rs` | Entire governed tool surface (gov.preflight, gov.drift, features.impact, snapshot.*, workspace.*, agent.*, run.*) is dead code from desktop perspective |
| **Titor Tauri commands** | `commands/titor.rs` — 5 of 6 commands are `todo!()` | No checkpoint/restore/diff/verify from desktop; library is production-grade underneath |
| **Blockoli Tauri commands** | `commands/search.rs` — both commands `todo!()` | Semantic search tab renders but cannot function |
| **`--dangerously-skip-permissions` hardcoded** | `claude.rs:969,1001,1036`, `agents.rs:774`, `web_server.rs:494,607,695` | All execution bypasses Claude's permission system; agent DB permission flags (`enable_file_read/write/network`) are stored but never enforced |
| **`spec/features.yaml` does not exist** | `crates/featuregraph/src/scanner.rs:167` expects it | featuregraph scanner always fails → governance panel permanently degraded |
| **Feature ID duality** | Spec IDs: `032-opc-inspect-governance-wiring-mvp` (kebab). Code headers: `FEATUREGRAPH_REGISTRY` (UPPERCASE). No mapping exists. | Registry and featuregraph data cannot be cross-referenced |

## Implications

- The **inspect → git → governance display** loop is real and working (T000–T009). Governance degrades gracefully rather than crashing.
- The **governed execution** loop does not exist at runtime. axiomregent has the tools; the desktop app doesn't start it.
- The gap between "show governance" and "enforce governance" is the platform's biggest structural debt.
- T010 (next task) should be implementable using existing wired paths without touching axiomregent or titor.

## Candidate promotions

- [x] `execution/changeset.md` — PR-6 governance wiring already recorded
- [ ] `execution/verification.md` — add command: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml commands::analysis::tests::` for governance backend tests
- [ ] `spec.md` or plan addendum — note that featuregraph half of governance panel returns degraded state due to `spec/features.yaml` not existing; this is bounded and expected for MVP scope
- [ ] Future spec — safety tier model and axiomregent activation are post-032 work items that need specs
