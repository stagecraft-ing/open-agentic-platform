# 044 Phase 4 Review — Async Governed Dispatch + Tauri Commands

**Reviewer:** claude
**Date:** 2026-03-30
**Scope:** `crates/orchestrator/src/lib.rs` (async `dispatch_manifest`, traits, P3-001/P3-002 fixes) + `apps/desktop/src-tauri/src/commands/orchestrator.rs` (Tauri surface)
**Verdict:** Phase 4 is spec-faithful. All prior findings addressed. Orchestrator contract surface fully exposed via Tauri commands.

## What Phase 4 delivers

1. **P3-001 fix** — Output artifact paths now injected into `build_step_system_prompt()` (`lib.rs:204–213`)
2. **P3-002 fix** — Quick effort text now includes "no sub-agent calls" (`lib.rs:175`)
3. **P3-003 coverage** — New test `build_step_system_prompt_handles_no_inputs` exercises zero-input branch (`lib.rs:716–731`)
4. **`AgentRegistry` trait** — async `has_agent(&str) -> bool` (`lib.rs:88–91`)
5. **`GovernedExecutor` trait** — async `dispatch_step(DispatchRequest) -> Result<DispatchResult, String>` (`lib.rs:93–96`)
6. **`DispatchRequest` / `DispatchResult`** — typed request/response for the executor interface (`lib.rs:73–86`)
7. **`dispatch_manifest()`** — async dispatcher with registry lookup, governed execution, output verification, failure cascade (`lib.rs:405–523`)
8. **Tauri commands** — `orchestrate_manifest`, `get_run_status`, `cancel_run`, `cleanup_artifacts` (`commands/orchestrator.rs`)

## Spec contract checklist

| Spec operation | Implemented? | Evidence |
|---------------|-------------|----------|
| `orchestrate_manifest(manifest_path)` → Run ID | **Yes** | `commands/orchestrator.rs:65–108` — reads YAML, loads agent registry from SQLite, dispatches via `dispatch_manifest()`, returns `RunSummary` (contains `run_id`) |
| `get_run_status(run_id)` → Step statuses, artifact paths, token usage | **Yes** | `commands/orchestrator.rs:111–128` — checks in-memory cache, falls back to `summary.json` on disk |
| `cancel_run(run_id)` → Pending/Running steps → Cancelled | **Yes** | `commands/orchestrator.rs:131–146` — marks `Pending`/`Running` as `Cancelled`, persists to disk |
| `cleanup_artifacts(run_id)` → deletes run directory | **Yes** | `commands/orchestrator.rs:149–159` — removes from in-memory state + calls `ArtifactManager::cleanup_run()` |
| `StepFailed { step_id, reason }` | **Yes** | `lib.rs:490–495` (missing output), `lib.rs:510–515` (executor error) |
| `DependencyMissing { step_id, artifact_path }` | **Yes** | `lib.rs:444–449` |
| `AgentNotFound { agent_id }` | **Yes** | `lib.rs:454–457` |
| `CycleDetected { cycle }` | **Yes** | Inherited from `validate_and_order()` (Phase 1) |

All 4 spec contract operations + all 4 error semantics are implemented.

## FR coverage (cumulative)

| FR | Status | Phase |
|----|--------|-------|
| FR-001 | Partial | P1 (step model); NL decomposition deferred |
| FR-002 | **Done** | P1 + P2 + P4 (input gating in async path) |
| FR-003 | **Done** | P1 (path convention) |
| FR-004 | **Done** | P3 + P4 (output paths now in prompt — P3-001 resolved) |
| FR-005 | **Done** | P1 + P3 + P4 ("no sub-agent calls" — P3-002 resolved) |
| FR-006 | **Done** | P1 + P2 + P4 (token usage recorded from `DispatchResult`) |
| FR-007 | **Done** | P1 (YAML manifest) |
| FR-008 | **Done** | P1 + P2 + P4 (failure cascade in async path) |

## SC coverage

| SC | Status | Evidence |
|----|--------|----------|
| SC-001 | Ready (needs real executor) | `dispatch_manifest()` wires the full pipeline; `FileBackedGovernedExecutor` is a scaffold that writes placeholder files |
| SC-002 | Deferred | Token measurement requires real agent execution |
| SC-003 | **Done** | `DependencyMissing` error + `Skipped` cascade in both noop and async dispatchers |
| SC-004 | **Done** | YAML manifest loads, validates (acyclic, no dup outputs), dispatches correctly |
| SC-005 | Deferred | Requires real agent execution with different effort levels |
| SC-006 | **Done** | `cleanup_artifacts()` calls `ArtifactManager::cleanup_run()` which does `remove_dir_all` |
| SC-007 | Deferred | Verification doc not yet written |

## Tests: 14/14 pass

| Test | Phase | Covers |
|------|-------|--------|
| `build_step_system_prompt_handles_no_inputs` | **P4** | P3-003: zero-input branch + "no sub-agent calls" in Quick |
| `dispatch_manifest_async_writes_summary_and_tokens` | **P4** | Single-step async dispatch → Success, tokens recorded, summary persisted |
| `dispatch_manifest_async_returns_agent_not_found` | **P4** | Registry miss → `AgentNotFound` error |
| (11 prior tests) | P1–P3 | Unchanged, still passing |

## Findings

### P4-001: `cancel_run` only touches in-memory state (LOW)

**File:** `commands/orchestrator.rs:131–146`
If a run completes and is evicted from `RUN_STATE` (or the process restarts), `cancel_run` silently succeeds without cancelling anything. The spec says `cancel_run(run_id)` → "running step finishes, pending steps marked cancelled". Currently the executor is synchronous within `orchestrate_manifest`, so the run is always either fully complete or in-progress when `cancel_run` is called. This gap matters once execution becomes truly concurrent (e.g., parallel steps or background task).

**Impact:** LOW for current scaffold; will need addressing when async/parallel dispatch lands.

### P4-002: `FileBackedGovernedExecutor` is a scaffold, not real governed execution (INFO)

**File:** `commands/orchestrator.rs:33–62`
The executor writes placeholder markdown files — it doesn't invoke any actual agent. This is explicitly called out in the code comment as a Phase 4 scaffold. The trait interface (`GovernedExecutor`) is correctly designed for swappable implementations.

**Impact:** INFO — by design. Swapping in a real executor (035 governed execution + 042 provider registry) requires no changes to `dispatch_manifest()` or the Tauri command surface.

### P4-003: `RUN_STATE` is a process-global static (INFO)

**File:** `commands/orchestrator.rs:16–20`
`LazyLock<Mutex<InMemoryRunState>>` means all runs share one mutex, and state is lost on process restart. The spec explicitly says "run metadata lives for the duration of the session; cross-session persistence is a follow-on" (Out of scope), so this is correct for MVP. The `get_run_status` fallback to `summary.json` on disk partially mitigates this.

### P4-004: `orchestrate_manifest` blocks on synchronous YAML read (INFO)

**File:** `commands/orchestrator.rs:69–70`
`std::fs::read_to_string` is synchronous in an async Tauri command. For small manifest files this is negligible. If manifests grow large or live on network storage, this would need `tokio::fs`.

### P4-005: No `Cancelled` status in `StepStatus` display for `dispatch_manifest` (INFO)

**File:** `lib.rs:427–429`
The async dispatcher skips `Cancelled` steps (alongside `Skipped`), but nothing in the async dispatcher ever *sets* `Cancelled` — that's only done by `cancel_run`. This is correct: cancellation is an external action, not an orchestrator-internal one. The status enum includes `Cancelled` for the Tauri command surface.

### P4-006: Agent registry snapshot is point-in-time (INFO)

**File:** `commands/orchestrator.rs:74–85`
The `SnapshotRegistry` loads all agent names from SQLite at dispatch start and never refreshes. If an agent is registered mid-run, it won't be found. Given the linear dispatch model, this is acceptable — the set of agents needed is known at manifest load time.

## Prior findings status

| Finding | Status | Resolution |
|---------|--------|------------|
| P3-001 (output paths not in prompt) | **Resolved** | `lib.rs:204–213` — output paths now injected under "Output artifact paths (write your results here):" |
| P3-002 (Quick text missing "no sub-agent calls") | **Resolved** | `lib.rs:175` — text now reads "quick — single-pass, very concise (< 2k tokens), no sub-agent calls" |
| P3-003 (no zero-input test) | **Resolved** | `build_step_system_prompt_handles_no_inputs` test added |
| P3-004 (`to_string_lossy`) | Acknowledged | INFO — acceptable for OAP's use case |

## Architecture assessment

The trait-based design (`AgentRegistry` + `GovernedExecutor`) is the correct integration pattern:

- `dispatch_manifest()` is parameterized by traits, making it testable and swappable
- The Tauri layer provides concrete implementations (`SnapshotRegistry` from SQLite, `FileBackedGovernedExecutor` as scaffold)
- The `DispatchRequest` carries the full system prompt (built by `build_step_system_prompt`), input/output paths, effort level — everything a real executor needs
- Output artifact verification happens *after* dispatch, *before* marking success — agents that don't write declared outputs fail the step

The `dispatch_manifest_noop()` from Phase 2 is preserved for backward compatibility and testing; the async `dispatch_manifest()` is the real path forward.

## Summary

Phase 4 completes the orchestrator's dispatch surface. All 3 actionable findings from Phase 3 are resolved. The async `dispatch_manifest()` correctly integrates registry lookup, governed execution via traits, output artifact verification, and failure cascade. All 4 spec contract operations are exposed as Tauri commands. 14/14 tests pass. 6 findings — 1 low (cancel_run scope), 5 info.

The next step to bring 044 to `status: active` is wiring a real `GovernedExecutor` implementation that dispatches through the 042 provider registry and 035 governed execution layer, replacing `FileBackedGovernedExecutor`. No changes to `dispatch_manifest()` or the Tauri commands are needed for that swap.
