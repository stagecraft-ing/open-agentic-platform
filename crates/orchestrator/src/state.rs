// SPDX-License-Identifier: AGPL-3.0-or-later
// Feature 052: State persistence for resumable workflows (JSON backend, Phase 1).

use crate::artifact::ArtifactManager;
use crate::OrchestratorError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Top-level workflow status stored in `state.json` (FR-001, FR-003).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    Running,
    Completed,
    Failed,
    TimedOut,
    AwaitingCheckpoint,
}

/// Per-step execution status stored in `state.json` (FR-002).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StepExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// Optional gate information attached to a step.
///
/// This struct is intentionally minimal in Phase 1 – Phase 3 (checkpoints and
/// approvals) will populate these fields with richer configs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateInfo {
    #[serde(rename = "type")]
    pub gate_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<JsonValue>,
}

/// Per-step state in `state.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepState {
    pub id: String,
    pub name: String,
    pub status: StepExecutionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateInfo>,
}

/// JSON state file schema (FR-001, FR-002, FR-007, SC-006).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowState {
    pub version: u32,
    pub workflow_id: Uuid,
    pub workflow_name: String,
    pub started_at: String,
    pub status: WorkflowStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_step_index: Option<usize>,
    pub steps: Vec<StepState>,
    /// Free-form metadata (branch, trigger, etc.).
    #[serde(default)]
    pub metadata: serde_json::Map<String, JsonValue>,
}

impl WorkflowState {
    /// Constructs a new in-memory workflow state with all steps pending.
    pub fn new(
        workflow_id: Uuid,
        workflow_name: impl Into<String>,
        started_at: String,
        step_defs: impl IntoIterator<Item = (String, String)>,
        metadata: serde_json::Map<String, JsonValue>,
    ) -> Self {
        let steps: Vec<StepState> = step_defs
            .into_iter()
            .map(|(id, name)| StepState {
                id,
                name,
                status: StepExecutionStatus::Pending,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                output: None,
                gate: None,
            })
            .collect();

        Self {
            version: 1,
            workflow_id,
            workflow_name: workflow_name.into(),
            started_at,
            status: WorkflowStatus::Running,
            current_step_index: None,
            steps,
            metadata,
        }
    }

    /// Marks a step as started and updates `current_step_index`.
    pub fn mark_step_started(&mut self, step_id: &str, started_at: String) {
        if let Some((idx, step)) = self
            .steps
            .iter_mut()
            .enumerate()
            .find(|(_, s)| s.id == step_id)
        {
            step.status = StepExecutionStatus::Running;
            step.started_at = Some(started_at);
            self.current_step_index = Some(idx);
        }
    }

    /// Marks a step as completed/failed/skipped with timing and output summary.
    pub fn mark_step_finished(
        &mut self,
        step_id: &str,
        status: StepExecutionStatus,
        completed_at: String,
        duration_ms: Option<u64>,
        output_summary: Option<JsonValue>,
    ) {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id == step_id) {
            step.status = status;
            step.completed_at = Some(completed_at);
            step.duration_ms = duration_ms;
            step.output = output_summary;
        }
    }
}

/// Computes the canonical `state.json` path for a given run directory.
pub fn state_file_path_for_run_dir(run_dir: &Path) -> PathBuf {
    run_dir.join("state.json")
}

/// Computes the canonical `state.json` path for a workflow using the artifact manager.
pub fn state_file_path_for_run(artifact_base: &ArtifactManager, workflow_id: Uuid) -> PathBuf {
    let run_dir = artifact_base.run_dir(workflow_id);
    state_file_path_for_run_dir(&run_dir)
}

/// Atomically writes the workflow state to `path` as pretty-printed JSON (FR-008, NF-003).
pub fn write_workflow_state_atomic(
    path: &Path,
    state: &WorkflowState,
) -> Result<(), OrchestratorError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| OrchestratorError::StatePersistence {
            reason: format!("create state dir {}: {e}", parent.display()),
        })?;
    }

    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_vec_pretty(state).map_err(|e| OrchestratorError::StatePersistence {
        reason: format!("serialize workflow state: {e}"),
    })?;

    fs::write(&tmp_path, &json).map_err(|e| OrchestratorError::StatePersistence {
        reason: format!("write temp state file {}: {e}", tmp_path.display()),
    })?;

    fs::rename(&tmp_path, path).map_err(|e| OrchestratorError::StatePersistence {
        reason: format!(
            "rename temp state file {} -> {}: {e}",
            tmp_path.display(),
            path.display()
        ),
    })?;

    Ok(())
}

/// Loads the current workflow state from `path` (FR-007 / SC-006).
pub fn load_workflow_state(path: &Path) -> Result<WorkflowState, OrchestratorError> {
    let bytes = fs::read(path).map_err(|e| OrchestratorError::StatePersistence {
        reason: format!("read state file {}: {e}", path.display()),
    })?;
    serde_json::from_slice(&bytes).map_err(|e| OrchestratorError::StatePersistence {
        reason: format!("decode state file {}: {e}", path.display()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ArtifactManager;

    #[test]
    fn new_initializes_pending_steps_and_running_status() {
        let mut meta = serde_json::Map::new();
        meta.insert("branch".to_string(), JsonValue::from("main"));
        let wf_id = Uuid::new_v4();

        let state = WorkflowState::new(
            wf_id,
            "deploy-staging",
            "2026-03-29T10:00:00Z".to_string(),
            vec![
                ("step_001".to_string(), "lint".to_string()),
                ("step_002".to_string(), "test".to_string()),
            ],
            meta,
        );

        assert_eq!(state.version, 1);
        assert_eq!(state.workflow_id, wf_id);
        assert_eq!(state.workflow_name, "deploy-staging");
        assert_eq!(state.status, WorkflowStatus::Running);
        assert_eq!(state.steps.len(), 2);
        assert!(state.steps.iter().all(|s| s.status == StepExecutionStatus::Pending));
        assert!(state.current_step_index.is_none());
        assert_eq!(state.metadata.get("branch").and_then(|v| v.as_str()), Some("main"));
    }

    #[test]
    fn mark_step_started_and_finished_updates_state() {
        let wf_id = Uuid::new_v4();
        let mut state = WorkflowState::new(
            wf_id,
            "deploy-staging",
            "2026-03-29T10:00:00Z".to_string(),
            vec![("step_001".to_string(), "lint".to_string())],
            serde_json::Map::new(),
        );

        state.mark_step_started("step_001", "2026-03-29T10:00:01Z".to_string());
        assert_eq!(state.current_step_index, Some(0));
        assert_eq!(state.steps[0].status, StepExecutionStatus::Running);
        assert_eq!(
            state.steps[0].started_at.as_deref(),
            Some("2026-03-29T10:00:01Z")
        );

        state.mark_step_finished(
            "step_001",
            StepExecutionStatus::Completed,
            "2026-03-29T10:00:05Z".to_string(),
            Some(4000),
            Some(serde_json::json!({"summary": "ok"})),
        );

        let step = &state.steps[0];
        assert_eq!(step.status, StepExecutionStatus::Completed);
        assert_eq!(
            step.completed_at.as_deref(),
            Some("2026-03-29T10:00:05Z")
        );
        assert_eq!(step.duration_ms, Some(4000));
        assert_eq!(
            step.output.as_ref().and_then(|v| v.get("summary")).and_then(|v| v.as_str()),
            Some("ok")
        );
    }

    #[test]
    fn write_and_load_state_round_trips_pretty_json() {
        let tmp = tempfile::tempdir().unwrap();
        let artifact_base = ArtifactManager::new(tmp.path());
        let wf_id = Uuid::new_v4();

        let mut meta = serde_json::Map::new();
        meta.insert("triggeredBy".to_string(), JsonValue::from("user@example.com"));

        let mut state = WorkflowState::new(
            wf_id,
            "deploy-staging",
            "2026-03-29T10:00:00Z".to_string(),
            vec![("step_001".to_string(), "lint".to_string())],
            meta,
        );
        state.mark_step_started("step_001", "2026-03-29T10:00:01Z".to_string());

        let path = state_file_path_for_run(&artifact_base, wf_id);
        write_workflow_state_atomic(&path, &state).unwrap();

        // Ensure we wrote to the canonical location with human-readable JSON.
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"workflowId\""));
        assert!(text.contains("\n  \"steps\""));

        // Ensure the temp file was cleaned up.
        assert!(!path.with_extension("json.tmp").exists());

        let loaded = load_workflow_state(&path).unwrap();
        assert_eq!(loaded.workflow_id, state.workflow_id);
        assert_eq!(loaded.workflow_name, state.workflow_name);
        assert_eq!(loaded.steps.len(), 1);
        assert_eq!(loaded.steps[0].id, "step_001");
    }
}

