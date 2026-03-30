// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/044-multi-agent-orchestration/spec.md

//! Multi-agent orchestration with file-based artifact passing (Feature 044).
//!
//! Phase 1: manifest parsing, DAG validation, artifact path helpers, effort classification.
//! Agent dispatch and Tauri wiring are follow-on slices.

pub mod artifact;
pub mod effort;
pub mod manifest;

pub use artifact::{ArtifactManager, DEFAULT_ARTIFACT_DIR};
pub use effort::{classify_from_task, EffortLevel};
pub use manifest::{split_input_ref, WorkflowManifest, WorkflowStep};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

/// Orchestrator-facing errors (044 contract + load-time validation).
#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("cycle detected: {message}")]
    CycleDetected { message: String },
    #[error("invalid manifest: {reason}")]
    InvalidManifest { reason: String },
    #[error("dependency missing at step {step_id}: {artifact_path}")]
    DependencyMissing {
        step_id: String,
        artifact_path: PathBuf,
    },
    #[error("step failed: {step_id} — {reason}")]
    StepFailed { step_id: String, reason: String },
    #[error("agent not found: {agent_id}")]
    AgentNotFound { agent_id: String },
}

/// Per-step status after or during a run (044 FR-006, FR-008).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    Running,
    Success,
    Failure,
    Skipped,
    Cancelled,
}

/// Serializable run summary (044 FR-006).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunSummary {
    pub run_id: Uuid,
    pub steps: Vec<StepSummaryEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StepSummaryEntry {
    pub step_id: String,
    pub agent: String,
    pub status: StepStatus,
    pub input_artifacts: Vec<PathBuf>,
    pub output_artifacts: Vec<PathBuf>,
    pub tokens_used: Option<u64>,
}

impl RunSummary {
    /// Persist `summary.json` under the run directory for this summary's `run_id`.
    pub fn write_to_disk(&self, artifact_base: &ArtifactManager) -> Result<(), OrchestratorError> {
        let run_dir = artifact_base.run_dir(self.run_id);
        let sj = serde_json::to_string_pretty(self).map_err(|e| OrchestratorError::InvalidManifest {
            reason: format!("serialize summary: {e}"),
        })?;
        std::fs::write(run_dir.join("summary.json"), sj).map_err(|e| OrchestratorError::InvalidManifest {
            reason: format!("write summary.json: {e}"),
        })?;
        Ok(())
    }
}

/// Writes frozen `manifest.yaml` and placeholder `summary.json` under the run directory.
pub fn materialize_run_directory(
    artifact_base: &ArtifactManager,
    run_id: Uuid,
    manifest: &WorkflowManifest,
) -> Result<PathBuf, OrchestratorError> {
    let run_dir = artifact_base.run_dir(run_id);
    std::fs::create_dir_all(&run_dir).map_err(|e| OrchestratorError::InvalidManifest {
        reason: format!("create run dir: {e}"),
    })?;
    let yaml = serde_yaml::to_string(manifest).map_err(|e| OrchestratorError::InvalidManifest {
        reason: format!("serialize manifest: {e}"),
    })?;
    std::fs::write(run_dir.join("manifest.yaml"), yaml).map_err(|e| {
        OrchestratorError::InvalidManifest {
            reason: format!("write manifest.yaml: {e}"),
        }
    })?;
    let summary = RunSummary {
        run_id,
        steps: vec![],
    };
    let sj = serde_json::to_string_pretty(&summary).map_err(|e| {
        OrchestratorError::InvalidManifest {
            reason: format!("serialize summary: {e}"),
        }
    })?;
    std::fs::write(run_dir.join("summary.json"), sj).map_err(|e| {
        OrchestratorError::InvalidManifest {
            reason: format!("write summary.json: {e}"),
        }
    })?;
    Ok(run_dir)
}

/// Resolves absolute artifact paths for a step's declared inputs (044).
pub fn resolve_input_paths(
    artifact_base: &ArtifactManager,
    run_id: Uuid,
    step: &WorkflowStep,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for input in &step.inputs {
        if let Some((producer_id, file)) = split_input_ref(input) {
            out.push(artifact_base.output_artifact_path(run_id, producer_id, file));
        } else {
            out.push(PathBuf::from(input));
        }
    }
    out
}

/// Dispatches a manifest using a no-op executor:
/// - checks that all required input artifacts exist before "running" a step (FR-002)
/// - marks steps as [`StepStatus::Success`] without invoking any agents
/// - on missing input, marks the step as [`StepStatus::Failure`], cascades [`StepStatus::Skipped`]
///   to dependents (FR-008), writes `summary.json`, and returns [`OrchestratorError::DependencyMissing`].
///
/// This is a Phase 2 scaffolding dispatcher — higher layers can later swap the
/// no-op execution for real governed agent runs without changing summary semantics.
pub fn dispatch_manifest_noop(
    artifact_base: &ArtifactManager,
    run_id: Uuid,
    manifest: &WorkflowManifest,
) -> Result<RunSummary, OrchestratorError> {
    let order = manifest.validate_and_order()?;
    let steps = &manifest.steps;

    // Pre-compute dependency relationships: which steps depend on a given producer step.
    let mut dependents: HashMap<&str, Vec<usize>> = HashMap::new();
    for (idx, step) in steps.iter().enumerate() {
        for input in &step.inputs {
            if let Some((producer_id, _file)) = split_input_ref(input) {
                dependents.entry(producer_id).or_default().push(idx);
            }
        }
    }

    // Track per-step status; default to Pending until processed.
    let mut statuses: Vec<StepStatus> = vec![StepStatus::Pending; steps.len()];

    // Helper: does this step depend on any failed or skipped step?
    let step_depends_on_failed_or_skipped = |idx: usize, statuses: &[StepStatus]| -> bool {
        let step = &steps[idx];
        for input in &step.inputs {
            if let Some((producer_id, _file)) = split_input_ref(input) {
                if let Some(prod_idx) = steps.iter().position(|s| s.id == producer_id) {
                    match statuses[prod_idx] {
                        StepStatus::Failure | StepStatus::Skipped | StepStatus::Cancelled => {
                            return true;
                        }
                        _ => {}
                    }
                }
            }
        }
        false
    };

    // Helper: resolve input paths for a specific step.
    let resolve_inputs_for_step =
        |step: &WorkflowStep| -> Vec<PathBuf> { resolve_input_paths(artifact_base, run_id, step) };

    // Process steps in topological order.
    for &idx in &order {
        // Already marked skipped from an upstream failure: respect that and continue.
        if matches!(statuses[idx], StepStatus::Skipped) {
            continue;
        }

        // If any of this step's producers failed or were skipped, mark as skipped (FR-008).
        if step_depends_on_failed_or_skipped(idx, &statuses) {
            statuses[idx] = StepStatus::Skipped;
            continue;
        }

        let step = &steps[idx];
        let input_paths = resolve_inputs_for_step(step);

        // Enforce FR-002: a step is not dispatched until all inputs exist.
        if let Some((missing_idx, missing_path)) = input_paths
            .iter()
            .enumerate()
            .find(|(_, p)| !p.exists())
            .map(|(i, p)| (i, p.clone()))
        {
            // Mark this step as failed.
            statuses[idx] = StepStatus::Failure;

            // Cascade skipped to direct dependents; they in turn will be skipped when processed
            // if they depend on a failed or skipped step.
            if let Some(dep_idxs) = dependents.get(step.id.as_str()) {
                for &d in dep_idxs {
                    if matches!(statuses[d], StepStatus::Pending | StepStatus::Running) {
                        statuses[d] = StepStatus::Skipped;
                    }
                }
            }

            // Build and persist summary before returning the error (FR-006 + FR-008).
            let mut summary_entries = Vec::with_capacity(steps.len());
            for (i, s) in steps.iter().enumerate() {
                let resolved_inputs = resolve_inputs_for_step(s);
                let output_paths: Vec<PathBuf> = s
                    .outputs
                    .iter()
                    .map(|o| artifact_base.output_artifact_path(run_id, &s.id, o))
                    .collect();
                summary_entries.push(StepSummaryEntry {
                    step_id: s.id.clone(),
                    agent: s.agent.clone(),
                    status: statuses[i].clone(),
                    input_artifacts: resolved_inputs,
                    output_artifacts: output_paths,
                    tokens_used: None,
                });
            }

            let summary = RunSummary {
                run_id,
                steps: summary_entries,
            };
            summary.write_to_disk(artifact_base)?;

            // Report which artifact was missing for this step.
            let failing_input = &step.inputs[missing_idx];
            let artifact_path = missing_path;
            return Err(OrchestratorError::DependencyMissing {
                step_id: step.id.clone(),
                artifact_path,
            });
        }

        // All inputs present: in the no-op dispatcher we just mark success.
        statuses[idx] = StepStatus::Success;
    }

    // Build final run summary with resolved artifact paths.
    let mut summary_entries = Vec::with_capacity(steps.len());
    for (i, step) in steps.iter().enumerate() {
        let input_paths = resolve_inputs_for_step(step);
        let output_paths: Vec<PathBuf> = step
            .outputs
            .iter()
            .map(|o| artifact_base.output_artifact_path(run_id, &step.id, o))
            .collect();
        summary_entries.push(StepSummaryEntry {
            step_id: step.id.clone(),
            agent: step.agent.clone(),
            status: statuses[i].clone(),
            input_artifacts: input_paths,
            output_artifacts: output_paths,
            tokens_used: None,
        });
    }

    let summary = RunSummary {
        run_id,
        steps: summary_entries,
    };
    summary.write_to_disk(artifact_base)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effort::EffortLevel;

    #[test]
    fn materialize_run_writes_files() {
        let tmp = tempfile::tempdir().unwrap();
        let am = ArtifactManager::new(tmp.path());
        let run_id = Uuid::nil();
        let m = WorkflowManifest {
            steps: vec![WorkflowStep {
                id: "s1".into(),
                agent: "test-agent".into(),
                effort: EffortLevel::Quick,
                inputs: vec![],
                outputs: vec!["out.md".into()],
                instruction: "do".into(),
            }],
        };
        let rd = materialize_run_directory(&am, run_id, &m).unwrap();
        assert!(rd.join("manifest.yaml").exists());
        assert!(rd.join("summary.json").exists());
    }

    #[test]
    fn dispatch_noop_marks_all_steps_success_when_inputs_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let am = ArtifactManager::new(tmp.path());
        let run_id = Uuid::new_v4();

        let manifest = WorkflowManifest {
            steps: vec![
                WorkflowStep {
                    id: "step-01".into(),
                    agent: "agent-a".into(),
                    effort: EffortLevel::Quick,
                    inputs: vec![],
                    outputs: vec!["out.md".into()],
                    instruction: "do a".into(),
                },
                WorkflowStep {
                    id: "step-02".into(),
                    agent: "agent-b".into(),
                    effort: EffortLevel::Investigate,
                    inputs: vec!["step-01/out.md".into()],
                    outputs: vec!["out.md".into()],
                    instruction: "do b".into(),
                },
            ],
        };

        // Materialize run dir and create the expected artifact for step-01.
        let run_dir = materialize_run_directory(&am, run_id, &manifest).unwrap();
        let step1_out = am.output_artifact_path(run_id, "step-01", "out.md");
        std::fs::create_dir_all(step1_out.parent().unwrap()).unwrap();
        std::fs::write(&step1_out, "ok").unwrap();

        let summary = dispatch_manifest_noop(&am, run_id, &manifest).unwrap();
        assert_eq!(summary.steps.len(), 2);
        assert!(summary
            .steps
            .iter()
            .all(|s| matches!(s.status, StepStatus::Success)));

        // Summary.json should have been updated.
        let summary_path = run_dir.join("summary.json");
        let contents = std::fs::read_to_string(summary_path).unwrap();
        assert!(contents.contains("\"step_id\": \"step-01\""));
        assert!(contents.contains("\"step_id\": \"step-02\""));
    }

    #[test]
    fn dispatch_noop_sets_failure_and_skipped_on_missing_input() {
        let tmp = tempfile::tempdir().unwrap();
        let am = ArtifactManager::new(tmp.path());
        let run_id = Uuid::new_v4();

        let manifest = WorkflowManifest {
            steps: vec![
                WorkflowStep {
                    id: "step-01".into(),
                    agent: "agent-a".into(),
                    effort: EffortLevel::Quick,
                    inputs: vec![],
                    outputs: vec!["out.md".into()],
                    instruction: "do a".into(),
                },
                WorkflowStep {
                    id: "step-02".into(),
                    agent: "agent-b".into(),
                    effort: EffortLevel::Investigate,
                    inputs: vec!["step-01/out.md".into()],
                    outputs: vec!["out2.md".into()],
                    instruction: "do b".into(),
                },
            ],
        };

        let run_dir = materialize_run_directory(&am, run_id, &manifest).unwrap();

        let err = dispatch_manifest_noop(&am, run_id, &manifest).unwrap_err();
        match err {
            OrchestratorError::DependencyMissing { step_id, artifact_path } => {
                assert_eq!(step_id, "step-02");
                assert!(artifact_path
                    .to_string_lossy()
                    .contains("step-01"));
            }
            other => panic!("expected DependencyMissing, got {other:?}"),
        }

        // Summary should reflect Failure for step-02 and Success or Pending for step-01.
        let contents = std::fs::read_to_string(run_dir.join("summary.json")).unwrap();
        assert!(contents.contains("\"step_id\": \"step-02\""));
        assert!(contents.contains("\"failure\"") || contents.contains("\"skipped\""));
    }
}
