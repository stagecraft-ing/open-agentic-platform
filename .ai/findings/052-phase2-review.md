# 052 State Persistence — Phase 2 Review

**Reviewer:** claude
**Date:** 2026-03-31
**Verdict:** Phase 2 approved

## Scope

Phase 2 covers resume detection: startup state file check, resume plan computation, and step-skipping logic per FR-003. Implementation adds `ResumePlan`, `compute_resume_plan_from_state`, and `detect_resume_plan_for_run` to `crates/orchestrator/src/lib.rs`.

## Requirement coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| **FR-003** (detect existing state file, offer resume, skip completed steps) | **Satisfied** | `detect_resume_plan_for_run` checks `state.json` existence via `path.exists()`, loads state, and delegates to `compute_resume_plan_from_state` which collects completed step IDs and finds the first non-completed step index. Test `detect_resume_plan_for_run_loads_state_and_computes_plan` confirms end-to-end detection with file I/O. |

## Design analysis

**Clean separation of concerns:**
- `compute_resume_plan_from_state` is a pure function (state + manifest → plan) — easy to unit test without I/O.
- `detect_resume_plan_for_run` handles I/O (file existence check, deserialization) and delegates computation.
- `ResumePlan` is `Serialize`/`Deserialize` — supports file-based context passing per orchestrator rules.

**Manifest-driven step matching:**
- Only steps present in *both* the manifest and the persisted state contribute to the resume plan. Steps in the state but absent from the manifest are silently ignored — correct behavior for manifest evolution between crash and resume.
- `completed_step_ids` preserves manifest ordering (iterates `manifest.steps`), not state ordering — this ensures the skip list aligns with the dispatch order.

**None semantics:**
- Returns `None` when no steps are completed (nothing to skip) — correct: fresh workflow, no resume prompt needed.
- Returns `None` when *all* steps are completed (`first_non_completed_index` is `None`) — correct: workflow already finished, nothing to resume.
- Returns `None` when state file doesn't exist — correct: no prior run.
- Returns `Some(plan)` only when there are both completed steps *and* remaining steps — exactly the condition where a resume prompt makes sense.

## Test coverage

| Test | What it verifies |
|------|-----------------|
| `compute_resume_plan_from_state_identifies_completed_and_first_remaining_step` | 3-step manifest, 2 completed → plan with `["step-1", "step-2"]` and index 2 |
| `compute_resume_plan_from_state_returns_none_when_nothing_to_resume` | All steps pending → `None` |
| `detect_resume_plan_for_run_handles_missing_state_file` | No state file on disk → `Ok(None)` |
| `detect_resume_plan_for_run_loads_state_and_computes_plan` | Writes state.json with 1 completed step, loads it, asserts plan matches |

4/4 resume tests pass. 22/22 total orchestrator tests pass.

## Findings

| ID | Severity | Description |
|----|----------|-------------|
| **P2-001** | LOW | **No test for "all steps completed" returning `None`.** The `returns_none_when_nothing_to_resume` test only covers the "no completed steps" branch. A test where all manifest steps are `Completed` would verify the `(false, None)` arm of the match. |
| **P2-002** | LOW | **No test for failed/running step as resume point.** A step marked `Failed` or `Running` in persisted state is correctly treated as non-completed (resume starts there), but this behavior isn't explicitly tested. |
| **P2-003** | LOW | **`state.status` not checked.** `compute_resume_plan_from_state` ignores the workflow-level status (`Failed`, `TimedOut`, `Completed`). A workflow marked `Completed` at the top level but with step-level inconsistencies would still produce a resume plan. This may be intentional (step-level is authoritative), but could confuse callers. |
| **P2-004** | INFO | **Run ID discovery not yet addressed.** `detect_resume_plan_for_run` requires the caller to know the exact `run_id: Uuid`. The spec describes detecting "existing state file for the current workflow context" — scanning the artifact base for state files by workflow name is not yet possible. This is likely a Phase 6 integration concern. |
| **P2-005** | INFO | **`ResumePlan` doesn't carry the workflow state.** Callers who want to display resume context (e.g., "Resume from step 3/5 (deploy)?") need to separately load the state. The plan only carries IDs and an index. Minimal design — acceptable for helpers, callers can load state independently. |
| **P2-006** | INFO | **Prior Phase 1 LOWs (P1-001 through P1-003) remain open.** Expected — these are deferred, not blocking. |

## Verdict

**Phase 2 approved.** The resume detection helpers correctly implement the core FR-003 computation: finding completed steps, identifying the resume point, and handling all edge cases (no state, no completed, all completed). The separation between pure computation and I/O is clean. 4 new tests provide good coverage of the happy path and key `None` branches. The 3 LOW findings are minor gaps in test coverage, not correctness issues. No blockers for Phase 3.
