# 052 Phase 3 Review: Checkpoint and Approval Gate Execution

**Reviewer:** claude
**Date:** 2026-03-31
**Verdict:** Phase 3 approved. FR-004, FR-005, SC-002, SC-003 satisfied.

## Scope

Phase 3 adds runtime gate execution mechanics on top of the gate declarations (manifest.rs, Phase 1) and state transitions (state.rs, Phase 1). New file: `crates/orchestrator/src/gates.rs`.

## Requirement Traceability

| Req | Status | Evidence |
|-----|--------|----------|
| FR-004 (checkpoint gates pause, write `"awaiting_checkpoint"`, resume on confirmation) | **Satisfied** | `evaluate_gate` checkpoint arm: calls `mark_awaiting_checkpoint` → persist → `await_checkpoint` → `mark_checkpoint_released` → persist. Test `checkpoint_pauses_and_resumes_on_confirm` verifies status transitions and 2 persist calls. |
| FR-005 (approval gates with configurable timeout and escalation) | **Satisfied** | `evaluate_gate` approval arm: `tokio::time::timeout(duration, handler.await_approval(...))`. On `Err(_elapsed)`: applies escalation via `mark_approval_timed_out`. Default escalation is `Fail` when `None`. Tests cover all three escalation policies + default. |
| FR-008 (atomic writes via persist callback) | **Satisfied** | `persist` callback invoked after every state transition (2 calls on happy path for both gate types, 2 calls on timeout path). Persist errors propagate as `GateError::HandlerError`. |
| SC-002 (checkpoint persists `"awaiting_checkpoint"`, resumes only after confirmation) | **Satisfied** | `checkpoint_pauses_and_resumes_on_confirm` test: asserts `WorkflowStatus::Running` after confirmation, persist count = 2. State must pass through `AwaitingCheckpoint` to reach the `await_checkpoint` call. |
| SC-003 (approval timeout → configured escalation outcome) | **Satisfied** | Three tests (`approval_timeout_escalation_fail`, `_skip`, `_notify`) each verify `GateOutcome::TimedOut { escalation }`, `WorkflowStatus::TimedOut`, and correct per-step status (`Failed`/`Skipped`/`Pending`). |

## Architecture Assessment

### GateHandler trait (lines 46-64)

Clean async trait with two methods: `await_checkpoint` and `await_approval`. `Send + Sync` bounds correct for use with `&dyn GateHandler` across `.await` points. The `timeout_ms` parameter on `await_approval` is informational (the caller manages the actual timeout) -- this is documented clearly in the doc comment and is the right design: it lets the handler display the remaining time to the operator without duplicating timeout enforcement.

### evaluate_gate (lines 82-153)

Two-arm match over `StepGateConfig`. Both arms follow the same pattern: mark awaiting → persist → block → outcome transition → persist. Clean separation of concerns: the function owns state transitions and persistence timing; the handler owns the operator interaction.

The approval timeout path correctly uses `tokio::time::timeout` wrapping the handler future. When the future is dropped on timeout, any resources held by the handler are cleaned up via normal Rust drop semantics.

### evaluate_gate_if_present (lines 159-175)

Thin convenience wrapper. Correctly delegates to `evaluate_gate` when a gate is present, returns `Ok(None)` otherwise. Prevents callers from needing to match on `Option<StepGateConfig>` themselves.

### chrono_now_iso (lines 178-192)

Records epoch seconds with millisecond precision as `"{secs}.{millis:03}"`. This is **not** ISO-8601 format despite the function name -- it produces `"1711843200.000"` rather than `"2026-03-31T00:00:00Z"`. See P3-001. The comment acknowledges this and recommends injecting a real clock in production.

### Error handling

`GateError::HandlerError(String)` covers both handler failures and persist failures. Implements `Display` and `Error`. Handler errors and persist errors are both mapped through the same variant -- acceptable for Phase 3 since callers get a descriptive message either way.

## Test Coverage

14 new async tests (all `#[tokio::test]`), 36/36 total orchestrator tests pass.

| Test | What it validates |
|------|-------------------|
| `checkpoint_pauses_and_resumes_on_confirm` | FR-004/SC-002: status transitions + persist count |
| `checkpoint_without_label_works` | Label=None path |
| `checkpoint_handler_error_propagates` | Error path for checkpoint |
| `approval_approved_within_timeout` | FR-005 happy path: approved before timeout |
| `approval_timeout_escalation_fail` | SC-003: timeout → Fail → step Failed |
| `approval_timeout_escalation_skip` | SC-003: timeout → Skip → step Skipped |
| `approval_timeout_escalation_notify` | SC-003: timeout → Notify → step Pending |
| `approval_timeout_defaults_to_fail_when_no_escalation` | Default escalation behavior |
| `approval_handler_error_propagates` | Error path for approval |
| `no_gate_returns_none` | Convenience wrapper passthrough |
| `gate_present_delegates_to_evaluate_gate` | Convenience wrapper delegation |
| `persist_error_propagates_from_checkpoint` | Persist failure path |

Test stubs are well-designed: `ImmediateApproveHandler` (instant confirm), `NeverRespondHandler` (blocks forever via `std::future::pending()`), `ErrorHandler` (returns configurable error). `counting_persist` with `AtomicU32` verifies persist call counts.

## Findings

### P3-001: `chrono_now_iso` is not ISO-8601 (LOW)

**File:** `gates.rs:178-192`
**Issue:** Function name says "ISO" but output is epoch seconds (`"1711843200.000"`), not ISO-8601 (`"2026-03-31T00:00:00Z"`). All other timestamps in the codebase (Phase 1 state tests, spec schema) use ISO-8601.
**Impact:** Low. The timestamp is recorded in `completed_at` and `duration_ms` fields. `duration_ms` carries the meaningful timing data. The `completed_at` string is human-readable enough for debugging. Production callers should inject a clock, as the comment notes.
**Recommendation:** Rename to `epoch_timestamp_string()` or add the `chrono` crate for real ISO formatting. Not blocking.

### P3-002: No test for checkpoint with unknown step_id (LOW)

**File:** `gates.rs` + `state.rs:170-173`
**Issue:** `mark_awaiting_checkpoint` silently no-ops if `step_id` doesn't match any step (the `if self.steps.iter().any(...)` guard). `evaluate_gate` doesn't verify the step_id exists before proceeding. If called with a typo'd step_id, the workflow status stays `Running` but the handler still blocks for confirmation.
**Impact:** Low. Callers are expected to pass valid step IDs from the manifest. A typo would produce a confusing state (handler blocks but status never changes to `AwaitingCheckpoint`), but this is a caller bug, not a gate-module bug.
**Recommendation:** Consider returning a `bool` from `mark_awaiting_checkpoint` and erroring in `evaluate_gate` if the step wasn't found. Not blocking.

### P3-003: Handler error on approval gate doesn't transition state (LOW)

**File:** `gates.rs:129-131`
**Issue:** When `handler.await_approval` returns `Err`, the function returns `Err(GateError::HandlerError(...))` but doesn't transition the workflow out of `AwaitingCheckpoint`. The state file was already persisted with `AwaitingCheckpoint` status. If the process restarts, the state shows `AwaitingCheckpoint` with no indication of the handler error.
**Impact:** Low. A handler error is an exceptional condition (connection loss, etc.). The `AwaitingCheckpoint` status is truthful -- the gate was never resolved. On resume, the gate would be re-evaluated. The caller could also handle the error and transition state explicitly.
**Recommendation:** Consider adding a `mark_gate_error` transition or documenting that callers should handle this. Not blocking.

### P3-004: `GateError` doesn't distinguish persist errors from handler errors (INFO)

**File:** `gates.rs:24-28`
**Issue:** Both persist callback failures and handler failures map to `GateError::HandlerError`. The variant name is misleading for persist errors.
**Impact:** Informational. The error message is descriptive enough to distinguish the source. Could add a `PersistError` variant in a future phase.

### P3-005: `timeout_ms` on approval handler is informational only (INFO)

**File:** `gates.rs:64`
**Issue:** The `await_approval` method receives `timeout_ms` as a parameter, but the actual timeout is enforced by the caller via `tokio::time::timeout`. The handler could ignore this value entirely.
**Impact:** Informational. This is documented and is the correct design -- it avoids dual timeout enforcement. The parameter lets handlers display remaining time.

### P3-006: No integration test with `write_workflow_state_atomic` as persist callback (INFO)

**File:** `gates.rs` tests
**Issue:** All tests use no-op or counting persist callbacks. No test verifies that `evaluate_gate` works end-to-end with the real `write_workflow_state_atomic` persist function.
**Impact:** Informational. Phase 6 (integration) is the designated place for end-to-end wiring tests. The unit-level persist callback pattern is clean and sufficient for Phase 3.

## Summary

Phase 3 delivers a clean, well-tested gate execution module. `GateHandler` is appropriately abstract -- CLI, API, and test implementations can be injected without coupling. Timeout mechanics use `tokio::time::timeout` correctly. All three escalation policies are tested. State persistence is invoked after every transition. 6 findings (3 LOW, 3 INFO), none blocking. **Phase 3 approved -- proceed to Phase 4 (SQLite backend).**
