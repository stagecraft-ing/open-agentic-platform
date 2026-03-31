# 052 Phase 6C Review — Integration Tests + P6-001/P6-003 Fixes

**Reviewer:** claude
**Date:** 2026-03-31
**Base commit:** aaa9071
**Verdict:** 6C (integration tests) approved. P6-001 and P6-003 fixes approved. 6D (dispatch loop wiring) not yet implemented.

## What Was Delivered

Commit `aaa9071` delivers three items:

### P6-001 Fix: **APPROVED**

`http.rs:68–70` — Changed `let mut guard = state.store.lock().await` → `let guard = ...` and `&mut *guard` → `&guard`. Correct — `subscribe_with_replay` takes `&SqliteWorkflowStore`, not `&mut`. Both clippy warnings (`unnecessary_mut_passed`, `explicit_auto_deref`) eliminated. Verified: `cargo clippy` shows 7 warnings total, all pre-existing from earlier phases, zero from `http.rs`.

### P6-003 Fix: **APPROVED**

`http.rs:112–114` and `http.rs:128` — Replaced `.json_data(json).unwrap_or_else(...)` with `.data(json)` in both replay and live event paths. This is the correct fix: `serde_json::to_string` already produces a JSON string, and `.data()` sends it verbatim as SSE `data:` field. Using `.json_data()` would have double-serialized (wrapping the JSON string in quotes). Both replay and live paths are now consistent.

### 6C — Integration Tests: **APPROVED**

`tests/integration_052.rs` adds 2 cross-module integration tests:

**Test 1 — `integration_052_crash_resume_from_state_file`:**
- Creates a 3-step `WorkflowManifest` with linear dependencies.
- Simulates partial execution: steps 1–2 marked completed via `mark_step_started` + `mark_step_finished`.
- Persists via `write_workflow_state_atomic` (JSON backend).
- Simulates crash by reloading from disk via `load_workflow_state`.
- Calls `compute_resume_plan_from_state` and asserts `completed_step_ids == ["step-1", "step-2"]`, `first_non_completed_step_index == 2`.
- **Satisfies SC-001** (crashed workflow resumes from last completed step) and **SC-005** (state file survives process crash) at the JSON layer.

**Test 2 — `integration_052_sse_replay_round_trip`:**
- Creates SQLite store, seeds a workflow row (FK constraint).
- Appends 3 events via `store.append_event` with distinct event types.
- Creates `EventBroadcaster`, calls `subscribe_with_replay(&store, wf_id, 0)`.
- Asserts: `replay.len() == 3`, correct event types, monotonic event IDs, `high_water_mark == e3`.
- **Satisfies FR-006** (replay from any offset) at the library layer. Cross-module: exercises `SqliteWorkflowStore` → `EventBroadcaster` → `ReplaySubscription` integration.

## Build and Test Results

- **Build:** `cargo build` succeeds, zero errors.
- **Tests:** 48 unit + 2 integration = 50/50 pass.
- **Clippy:** 7 warnings total, all pre-existing. Zero new warnings.

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| P6C-001 | INFO | No live-event integration test — test 2 only exercises replay. A full integration test would broadcast after subscribing and verify `recv()` returns the live event. Acceptable since the unit test `replay_then_live_deduplication_pattern` in `sse.rs` covers this path. |
| P6C-002 | INFO | No HTTP-level integration test — no test exercises the Axum endpoint directly (e.g., via `axum::test` or `tower::ServiceExt`). Acceptable for Phase 6 scope; P6-002 remains open. |
| P6C-003 | INFO | Test 1 uses JSON backend, not SQLite. Spec allows both, and the round-trip pattern is the same, but SQLite crash-resume is untested at integration level. |
| P6C-004 | LOW | No "full stack" test per readiness guidance (Test 3: manifest → dispatch → persist → crash → resume → SSE replay). This is blocked on 6D (dispatch loop wiring), which hasn't been implemented. |

## Phase 6 Completion Status

| Sub-task | Status | Notes |
|----------|--------|-------|
| 6A — HTTP SSE endpoint | **Done** | Approved in prior review |
| 6B — Broadcaster registry | **Done** | Approved in prior review |
| 6C — Integration tests | **Done** | 2 tests, approved in this review |
| 6D — Dispatch loop wiring | **Not started** | Neither `dispatch_manifest_noop` nor `dispatch_manifest` persist state or emit events |

## 6D Assessment

`dispatch_manifest_noop` (lib.rs:328) and `dispatch_manifest` (lib.rs:503) both dispatch steps and track statuses in-memory, but do NOT:
- Create a `WorkflowState` at workflow start
- Call `write_workflow_state_atomic` or `SqliteWorkflowStore::write_workflow_state` after each step
- Call `append_event` or `broadcaster.broadcast` for step transitions
- Wire gate evaluation with a store-backed persist callback

This is the final remaining gap for 052 feature-completeness. Without 6D, state persistence is a library-only capability that no orchestrator command actually uses.

## Recommendation

6C is solid and exercises the critical cross-module paths. P6-001 and P6-003 are correctly fixed. The implementer should proceed with:

1. **6D:** Wire state persistence into `dispatch_manifest` (the async variant is the natural target — it already has step lifecycle tracking). Add optional `SqliteWorkflowStore` + `EventBroadcaster` parameters. At workflow start: `WorkflowState::new` + persist + `append_event("workflow_started")`. At each step: `mark_step_started` + persist + `append_event("step_started")` → execute → `mark_step_finished` + persist + `append_event("step_completed"|"step_failed")`.
2. After 6D: add a full-stack integration test (P6C-004) and `verification.md`.

**No blockers.** Hand to **cursor** for 6D implementation.
