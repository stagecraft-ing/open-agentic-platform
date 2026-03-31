# 052 State Persistence — Phase 1 Review

**Reviewer:** claude
**Date:** 2026-03-31
**Verdict:** Phase 1 approved

## Scope

Phase 1 covers the JSON state file core: `WorkflowState` schema, in-memory lifecycle helpers, atomic write/load, and the state query path. Implementation lives in `crates/orchestrator/src/state.rs` with re-exports from `crates/orchestrator/src/lib.rs`.

## Requirement coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| **FR-001** (state file created with workflow id, start time, steps, status `"running"`) | **Satisfied** | `WorkflowState::new()` sets `version: 1`, `workflow_id`, `started_at`, steps from `step_defs`, `status: Running`. Test `new_initializes_pending_steps_and_running_status` confirms. |
| **FR-002** (atomic step update with status, output, duration, timestamp) | **Satisfied** | `mark_step_finished()` writes `status`, `completed_at`, `duration_ms`, `output`. Test `mark_step_started_and_finished_updates_state` confirms all fields populated. |
| **FR-007** (state query API returns full workflow state) | **Satisfied** | `load_workflow_state()` deserializes full `WorkflowState` including all step statuses, current position, metadata. Test `write_and_load_state_round_trips_pretty_json` confirms round-trip fidelity. |
| **FR-008** (atomic writes via temp+rename) | **Satisfied** | `write_workflow_state_atomic()` writes to `state.json.tmp`, then `fs::rename` to `state.json`. Test confirms temp file does not persist after write. |
| **NF-003** (human-readable JSON) | **Satisfied** | Uses `serde_json::to_vec_pretty`. Test asserts `\n  "steps"` indentation in output. |
| **SC-006** (state query API returns accurate status at any point) | **Satisfied** | `load_workflow_state` reads and deserializes whatever was last atomically written. |

## Schema fidelity

Spec JSON schema fields vs implementation:

| Spec field | Implementation | Match |
|-----------|---------------|-------|
| `version` | `version: u32` | Yes |
| `workflowId` | `workflow_id: Uuid` with `#[serde(rename_all = "camelCase")]` → `workflowId` | Yes |
| `workflowName` | `workflow_name` → `workflowName` | Yes |
| `startedAt` | `started_at` → `startedAt` | Yes |
| `status` | `status: WorkflowStatus` (lowercase serde) | Yes |
| `currentStepIndex` | `current_step_index: Option<usize>` → `currentStepIndex` | Yes |
| `steps[].id` | `id: String` | Yes |
| `steps[].name` | `name: String` | Yes |
| `steps[].status` | `status: StepExecutionStatus` (lowercase serde) | Yes |
| `steps[].startedAt` | `started_at` → `startedAt` | Yes |
| `steps[].completedAt` | `completed_at` → `completedAt` | Yes |
| `steps[].durationMs` | `duration_ms` → `durationMs` | Yes |
| `steps[].output` | `output: Option<JsonValue>` | Yes |
| `steps[].gate` | `gate: Option<GateInfo>` with `type`/`timeoutMs` | Yes |
| `metadata` | `metadata: serde_json::Map<String, JsonValue>` | Yes |

All 15 spec schema fields present with correct camelCase serialization.

## Error handling

- `OrchestratorError::StatePersistence { reason }` added to error enum at `lib.rs:50`.
- All I/O paths in `write_workflow_state_atomic` and `load_workflow_state` map errors through this variant with descriptive messages including file paths.
- `create_dir_all` ensures parent directories exist before write — correct defensive behavior.

## Test coverage

3 tests covering:
1. Constructor initializes all steps as `Pending` with `Running` workflow status
2. `mark_step_started` + `mark_step_finished` update all expected fields
3. Write/load round-trip with pretty JSON, temp file cleanup verification

All 18 orchestrator tests pass (3 state-specific + 15 existing).

## Findings

| ID | Finding | Severity | Notes |
|----|---------|----------|-------|
| P1-001 | `WorkflowState::new` takes `Uuid` but spec shows string `"wf_abc123"` format for `workflowId` | **LOW** | Uuid is a stronger type; spec example is illustrative not prescriptive. Uuid serializes as string in JSON anyway. Acceptable deviation. |
| P1-002 | `mark_step_started` / `mark_step_finished` silently no-op if `step_id` not found | **LOW** | No error returned for unknown step IDs. Callers may miss typos. Phase 2+ may want to surface this, but acceptable for Phase 1 in-memory helpers. |
| P1-003 | No test for `load_workflow_state` on missing/corrupt file | **LOW** | Error paths are straightforward (`fs::read` → `StatePersistence`), but an explicit test would confirm the error variant. |
| P1-004 | `current_step_index` is `Option<usize>` (spec shows plain integer `2`) | **INFO** | `Option` is correct — `None` represents "no step started yet". Spec schema example just shows a mid-workflow snapshot. |
| P1-005 | `WorkflowStatus` includes `TimedOut` and `AwaitingCheckpoint` ahead of Phase 3 | **INFO** | Forward-looking enum variants. No harm; avoids breaking changes in Phase 3. |
| P1-006 | `state_file_path_for_run` depends on `ArtifactManager` from 044 | **INFO** | Clean integration — state files live alongside run artifacts. Consistent with handoff note that 052 is a natural follow-on to 044. |

## Verdict

**Phase 1 approved.** All Phase 1 requirements (FR-001, FR-002, FR-007, FR-008, NF-003, SC-006) satisfied. Schema matches spec exactly (camelCase, all fields present). Atomic write pattern correct. Tests pass. No blockers for Phase 2 (resume detection).
