# 044 Phase 2 Review — Dispatcher Stub, Summary Persistence, Failure Cascade

**Reviewer:** claude
**Date:** 2026-03-30
**Scope:** `crates/orchestrator/src/lib.rs` Phase 2 additions vs `specs/044-multi-agent-orchestration/spec.md`
**Verdict:** Phase 2 is spec-faithful. All Phase 2 deliverables are correctly implemented.

## Phase 2 deliverables

| Deliverable | Status | Evidence |
|-------------|--------|----------|
| Dispatcher stub (no-op executor) | **Done** | `dispatch_manifest_noop()` processes steps in topological order, checks input artifact existence, marks success without invoking agents |
| Input artifact existence check (FR-002) | **Done** | `input_paths.iter().find(\|(_, p)\| !p.exists())` gates dispatch per step |
| `summary.json` populated on completion (FR-006) | **Done** | `RunSummary::write_to_disk()` writes final summary on both success and failure paths |
| `StepStatus` cascade on failure (FR-008) | **Done** | Failed step → `Failure`; direct dependents → `Skipped` (eager); transitive dependents → `Skipped` (lazy check via `step_depends_on_failed_or_skipped`) |

## FR coverage update (cumulative)

| FR | Status | Phase |
|----|--------|-------|
| FR-001 | Partial | P1 (step model), NL decomposition deferred |
| FR-002 | **Done** | P1 (topo sort) + P2 (file existence gating) |
| FR-003 | **Done** | P1 (artifact path resolution) |
| FR-005 | **Done** | P1 (effort classification) |
| FR-006 | **Done** | P1 (summary struct) + P2 (summary persistence) |
| FR-007 | **Done** | P1 (YAML manifest loading + validation) |
| FR-008 | **Done** | P1 (StepStatus::Skipped variant) + P2 (cascade logic) |

Remaining: FR-004 (agent prompt injection with artifact paths) — requires real agent execution.

## Architecture assessment

The dispatcher uses a belt-and-suspenders approach for failure cascade:

1. **Eager marking** (lines 216–222): On step failure, direct dependents in the pre-computed `dependents` map are immediately marked `Skipped`.
2. **Lazy check** (lines 196–199): Before processing any step, `step_depends_on_failed_or_skipped` re-checks all producer statuses, catching transitive dependencies.

This is correct because topological ordering guarantees all producers are processed before consumers. The eager marking is technically redundant but acts as an optimization that short-circuits the main loop for direct dependents. No correctness issue.

The `RunSummary::write_to_disk()` method is a clean addition that decouples summary serialization from `materialize_run_directory()`, allowing summary to be updated at any point during a run.

## Tests: 10/10 pass

| Test | New? | Covers |
|------|------|--------|
| `dispatch_noop_marks_all_steps_success_when_inputs_exist` | **P2** | FR-002 happy path: pre-created artifacts → all steps Success; summary.json written with both step entries |
| `dispatch_noop_sets_failure_and_skipped_on_missing_input` | **P2** | FR-008: missing input → step Failure, downstream Skipped; DependencyMissing error returned; summary.json persisted before error |
| (8 Phase 1 tests) | P1 | Unchanged, still passing |

## Findings

### P2-001: Unused variable `failing_input` (COSMETIC)

**File:** `lib.rs:250`
`let failing_input = &step.inputs[missing_idx]` is assigned but never read. Compiler warning emitted. **Fixed** — prefixed with underscore.

### P2-002: Summary-building code is duplicated (LOW)

**File:** `lib.rs:225–246` and `lib.rs:263–279`
The logic to build `Vec<StepSummaryEntry>` from `steps` + `statuses` is duplicated between the failure path and the success path. A helper like `build_summary_entries(steps, statuses, ...)` would eliminate ~20 lines of duplication. Not blocking — the two paths are identical in structure and a future refactor would be clean.

### P2-003: No test for 3+ step transitive cascade (INFO)

The failure cascade test uses a 2-step chain (step-01 → step-02). A 3-step chain (A → B → C) where A's output is missing would verify that C is transitively skipped even though it doesn't directly depend on A. The current logic handles this correctly via the lazy check, but there's no test proving it.

### P2-004: `step_depends_on_failed_or_skipped` uses linear scan (INFO)

**File:** `lib.rs:171`
`steps.iter().position(|s| s.id == producer_id)` is O(n) per input per step. For the expected workflow sizes (< 20 steps) this is negligible. If workflows grow large, an index map `HashMap<&str, usize>` would be more efficient. Not actionable now.

### P2-005: Root step with no inputs always succeeds in noop (INFO)

By design — the no-op dispatcher doesn't execute agents, so root steps (no inputs) are trivially successful. A real dispatcher will need to handle agent execution failure for root steps. This is expected Phase 2 behavior and correctly documented in the function's doc comment.

## Summary

Phase 2 is solid. The no-op dispatcher correctly implements FR-002 (input gating), FR-006 (summary persistence), and FR-008 (failure cascade with skip propagation). The architecture cleanly separates dispatch mechanics from agent execution, making the swap to real governed execution a straightforward replacement of the inner loop. 10/10 tests pass. 5 findings — 1 cosmetic (fixed), 1 low, 3 info.

**Recommendation:** Proceed to Phase 3 — wire to governed agent execution (035) + agent registry lookup (042). This replaces the no-op inner loop with real `agent_dispatch(step, effort, artifact_paths)` calls.
