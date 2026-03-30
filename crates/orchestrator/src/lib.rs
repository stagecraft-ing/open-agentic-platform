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
}
