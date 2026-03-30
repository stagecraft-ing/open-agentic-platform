# 044 Phase 1 Review — Manifest, DAG, Artifacts

**Reviewer:** claude
**Date:** 2026-03-30
**Scope:** `crates/orchestrator/` Phase 1 vs `specs/044-multi-agent-orchestration/spec.md`
**Verdict:** Phase 1 is spec-faithful. All Phase 1 deliverables are correctly implemented.

## FR coverage (Phase 1 scope only)

| FR | Status | Evidence |
|----|--------|----------|
| FR-001 | Partial (expected) | `WorkflowStep` has agent, inputs, outputs, effort, instruction. Natural-language decomposition is dispatch-phase work — not Phase 1. |
| FR-002 | **Done** | `validate_and_order()` returns topological indices. Dispatch gating on file existence is next phase. |
| FR-003 | **Done** | `ArtifactManager` resolves `$OAP_ARTIFACT_DIR/<run_id>/<step_id>/<filename>`. `DEFAULT_ARTIFACT_DIR` = `/tmp/oap-artifacts`. `from_env()` reads `OAP_ARTIFACT_DIR`. |
| FR-005 | **Done** | `EffortLevel` enum with `Quick`/`Investigate`/`Deep`. `classify_from_task()` covers all spec trigger phrases. `token_budget_hint()` returns `2_000`/`10_000`/`None`. Default is `Investigate`. |
| FR-006 | **Done** (scaffold) | `RunSummary` + `StepSummaryEntry` with all spec fields (step_id, agent, status, input/output artifacts, tokens_used). `materialize_run_directory` writes placeholder `summary.json`. Population deferred to dispatch. |
| FR-007 | **Done** | `WorkflowManifest::load_from_file()` loads YAML. `validate_and_order()` checks: acyclic, no duplicate outputs, every input references a valid producer output. |
| FR-008 | Partial (expected) | `StepStatus::Skipped` variant exists. Actual cascade logic is dispatch-phase. |

## Error variants vs spec contract

| Spec error | Crate variant | Match? |
|------------|---------------|--------|
| `CycleDetected { cycle }` | `CycleDetected { message }` | **Yes** — field name differs (`message` vs `cycle`) but semantically correct. The message is human-readable rather than a structured cycle path; acceptable for Phase 1. |
| `DependencyMissing { step_id, artifact_path }` | `DependencyMissing { step_id, artifact_path: PathBuf }` | **Exact match** |
| `StepFailed { step_id, reason }` | `StepFailed { step_id, reason }` | **Exact match** |
| `AgentNotFound { agent_id }` | `AgentNotFound { agent_id }` | **Exact match** |
| — | `InvalidManifest { reason }` | **Extension** — not in spec contract, but sensible for YAML load/validation errors. No conflict. |

## Validation rules vs spec

| Rule | Implemented? | Evidence |
|------|-------------|----------|
| Graph must be acyclic | **Yes** | `topological_sort()` via Kahn's algorithm, returns `CycleDetected` if `order.len() != n` |
| Every input must be output of prior step or pre-existing file | **Yes** | `validate_steps()` checks producer step exists and lists the file in outputs; absolute/UNC paths treated as external |
| No two steps may declare same output path | **Yes** | Full path `step_id/filename` checked via `HashSet`. Same filename in different steps is correctly allowed. |

## Artifact directory layout vs spec

Spec requires:
```
$OAP_ARTIFACT_DIR/<run_id>/
  manifest.yaml
  summary.json
  step-XX/artifact.md
```

`materialize_run_directory()` creates `manifest.yaml` (frozen YAML) and `summary.json` (empty placeholder). Step dirs created by `ensure_step_dir()`. **Matches spec layout.**

## Tests: 8/8 pass

| Test | Covers |
|------|--------|
| `paths_under_base` | `ArtifactManager` path resolution |
| `default_investigate_when_no_phrase` | FR-005 default effort |
| `quick_phrases` | FR-005 quick triggers |
| `deep_phrases` | FR-005 deep triggers |
| `three_step_linear_order` | FR-002/FR-007 topological sort |
| `cycle_rejected` | Cycle detection |
| `duplicate_output_rejected` | Duplicate output validation |
| `materialize_run_writes_files` | Run directory materialization |

## Findings

### P1-001: `CycleDetected` does not report the actual cycle path (LOW)

**File:** `manifest.rs:146`
The spec says `CycleDetected { cycle }` suggesting the cycle itself should be reported. Current implementation returns a generic message string. For Phase 1 this is fine — a user-facing message showing which step IDs form the cycle would be a nice follow-on.

### P1-002: `classify_from_task` has greedy match on "quick" (INFO)

**File:** `effort.rs:44`
The word "quick" appears in non-trigger contexts (e.g., "quickly investigate" should arguably be `Investigate`, not `Quick`). The spec lists "quick" as a standalone trigger phrase, so matching on substring `contains("quick")` is a defensible simplification. Worth revisiting if misclassification reports arise.

### P1-003: No `investigate` trigger phrases tested (INFO)

**File:** `effort.rs:63-88`
Tests cover `Quick`, `Deep`, and the default fallback. There's no explicit test for "investigate" / "look into" / "analyze" matching `Investigate`. The default test (`"fix the login bug"`) proves the fallback, but the explicit keyword paths are untested.

### P1-004: Empty-step manifest not tested (INFO)

**File:** `manifest.rs:44-47`
`validate_steps` correctly rejects empty manifests, but there's no test exercising this path.

## Summary

Phase 1 is solid. All spec-mandated Phase 1 deliverables — YAML manifest loading, DAG validation (cycle + duplicate output + input reference checks), topological ordering, artifact path management, effort classification, run directory materialization, and error types — are correctly implemented. The 4 findings are all LOW/INFO and do not block the next phase.

**Recommendation:** Proceed to Phase 2 — step dispatcher (check input artifacts exist before dispatch, populate `summary.json` after steps complete, wire `StepStatus` cascade on failure).
