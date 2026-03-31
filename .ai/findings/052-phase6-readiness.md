# 052 Phase 6 Readiness Review — Pre-Implementation Assessment

**Reviewer:** claude
**Date:** 2026-03-31
**Base commit:** 2788d97
**Verdict:** Phases 1–5 approved and solid. Phase 6 ready for implementation with clear scope below.

## Current State (Phases 1–5 Complete)

| Phase | Module | Key Deliverable | Status |
|-------|--------|----------------|--------|
| 1 | `state.rs` | JSON `WorkflowState` with atomic write-to-temp-then-rename | ✅ Approved |
| 2 | `lib.rs` | `ResumePlan` + `compute_resume_plan_from_state` (FR-003) | ✅ Approved |
| 3 | `gates.rs` | `GateHandler` trait, checkpoint/approval evaluation (FR-004, FR-005) | ✅ Approved |
| 4 | `sqlite_state.rs` | `SqliteWorkflowStore` with WAL, 3-table schema, events table | ✅ Approved |
| 5 | `sse.rs` | `EventBroadcaster`, `subscribe_with_replay`, dedup pattern (FR-006, NF-002, SC-004) | ✅ Approved |

All 48 orchestrator tests pass. Zero clippy warnings.

## Phase 6 Scope — What Remains

Phase 6 per spec: *"Wire state persistence into existing orchestrator commands and verify end-to-end resume across crash scenarios."*

### 6A — HTTP SSE Endpoint

**What:** An HTTP endpoint (e.g., `GET /workflows/{id}/events?offset=0`) that wraps `EventBroadcaster::subscribe_with_replay` and frames output as `text/event-stream` (SSE).

**Implementation guidance:**
- Add `axum` (or `warp`) as an optional dependency — `axum` is more common in the Tokio ecosystem and has native SSE support (`axum::response::sse::Sse`).
- Endpoint should: parse `workflow_id` from path, parse `offset` from query (default 0), call `subscribe_with_replay`, stream `replay` events first, then `recv()` loop for live events, skip duplicates via `high_water_mark`.
- Response: `Content-Type: text/event-stream`, each event as `data: {json}\n\n` with optional `id: {event_id}\n`.
- Error handling: 404 if workflow_id not found, 500 on store errors.

**Dependencies needed in `Cargo.toml`:**
- `axum` (with `tokio` `rt-multi-thread` and `macros` features for production runtime)
- Possibly `tower` for middleware (not strictly required for Phase 6)

**Finding R-001 (INFO):** The current `tokio` dep only has `sync` and `time` features. Production HTTP serving requires `rt-multi-thread`. Dev-deps already have `macros` and `rt` but those won't be available at runtime. The implementer must add runtime features to the main `[dependencies]` section.

### 6B — Per-Workflow Broadcaster Registry

**What:** A shared `BroadcasterRegistry` mapping `workflow_id → EventBroadcaster` for use by the HTTP server and the orchestrator dispatch loop.

**Implementation guidance:**
- `Arc<DashMap<Uuid, EventBroadcaster>>` or `Arc<RwLock<HashMap<Uuid, EventBroadcaster>>>` — either works; DashMap avoids write-contention on independent workflows.
- The orchestrator creates/registers a broadcaster when a workflow starts, removes it when the workflow completes (or after a configurable TTL).
- `append_event` calls should be followed by `broadcaster.broadcast(event)` in the dispatch loop.

**Finding R-002 (LOW):** `SqliteWorkflowStore` currently requires `&mut self` for `append_event`. The HTTP server will hold a shared `Arc<Mutex<SqliteWorkflowStore>>` for writes. The broadcaster is already `Clone + Send + Sync` so it doesn't need wrapping. The implementer should verify that the `Mutex` contention on the store is acceptable — SQLite WAL allows concurrent reads so only writes need serialization.

### 6C — End-to-End Crash Resume Verification

**What:** Integration tests proving SC-001 (crashed workflow resumes from last completed step), SC-005 (state files survive process crashes), and the full stack (manifest → dispatch → state persist → crash → resume plan → skip completed → SSE replay).

**Implementation guidance:**
- Create `crates/orchestrator/tests/integration_052.rs` (or a `tests/` module).
- Test 1 — **Crash resume:** Create manifest with 3 steps. Run steps 1–2 (persist state after each). Simulate crash (drop everything). Re-open store, call `compute_resume_plan_from_state` → assert `first_non_completed_step_index == 2`.
- Test 2 — **SSE replay after crash:** Append events for steps 1–2. Drop broadcaster. Re-create broadcaster, call `subscribe_with_replay(store, wf_id, 0)` → assert replay contains all events. Simulate step 3 → assert live event received.
- Test 3 — **Full stack:** Manifest → `WorkflowState::new` → step dispatch with `append_event` + `broadcast` → checkpoint gate → resume plan → SSE client sees entire history.

**Finding R-003 (LOW):** There are no integration tests currently. All tests are `#[cfg(test)]` unit tests within each module. Phase 6 should introduce `tests/` directory-level integration tests that exercise cross-module paths.

### 6D — Wire Into Existing Orchestrator Commands

**What:** `dispatch_manifest_noop` (the existing no-op dispatcher in `lib.rs`) and any future real dispatch loop should create a `WorkflowState` at workflow start, call `write_workflow_state_atomic` (or `SqliteWorkflowStore::write_workflow_state`) after each step, and `append_event` + `broadcast` for each step transition.

**Implementation guidance:**
- The current `dispatch_manifest_noop` validates artifacts and records statuses but does NOT persist state. Phase 6 should add an optional `SqliteWorkflowStore` parameter (or make a new `dispatch_manifest_persisted` variant) that persists after each step.
- At workflow start: create `WorkflowState::new`, `write_workflow_state`, `append_event("workflow_started")`.
- At each step: `mark_step_started`, persist, `append_event("step_started")`, run step, `mark_step_finished`, persist, `append_event("step_completed"|"step_failed")`.
- On gate: `evaluate_gate` already calls persist callback — just wire it to the store.

## Carry-Forward Findings from Phases 1–5

| ID | Severity | Description | Phase 6 Impact |
|----|----------|-------------|----------------|
| P3-001 | LOW | `chrono_now_iso` / `sqlite_now_ts` outputs epoch seconds, not ISO-8601 | Cosmetic — SSE clients will see epoch timestamps |
| P4-001 | LOW | `current_step_index` not persisted (always `None` on load) | Resume logic uses step statuses, not index — no impact |
| P4-002 | LOW | `completed_at` always NULL on workflows table | Schema column exists but field not in `WorkflowState` — no impact |
| P4-003 | LOW | `timeout_ms` lost in gate round-trip | Approval gate timeout not persisted — acceptable if gates re-read from manifest on resume |
| P5-008 | INFO | Lagged subscriber handling | Consumers should handle `RecvError::Lagged` by re-querying SQLite |
| P5-009 | INFO | Per-workflow broadcaster registry needed | Addressed by 6B above |

## Recommendation

Phase 6 is the integration and verification phase. The library primitives are solid and well-tested. The implementer should:

1. Add `axum` + tokio runtime features to `Cargo.toml`
2. Create `src/http.rs` with SSE endpoint + broadcaster registry
3. Create `src/dispatch_persisted.rs` (or extend `lib.rs`) wiring state persistence into the dispatch loop
4. Create `tests/integration_052.rs` with crash-resume and SSE replay tests
5. Add `specs/052-state-persistence/execution/verification.md` documenting SC-001 through SC-006 evidence

**No blockers.** Hand to **cursor** for Phase 6 implementation.
