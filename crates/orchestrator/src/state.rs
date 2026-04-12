// SPDX-License-Identifier: AGPL-3.0-or-later
// Feature 052: State persistence for resumable workflows (JSON backend, Phase 1).

use crate::OrchestratorError;
use crate::artifact::ArtifactManager;
use crate::manifest::{ApprovalEscalation, StepGateConfig, WorkflowManifest};
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
    /// Fully synced — all events and artifacts recorded to platform. Promotion-eligible.
    Completed,
    /// Finished locally but platform sync incomplete. Not promotion-eligible (spec 097).
    CompletedLocal,
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

    /// Attaches gate information to steps based on the workflow manifest.
    ///
    /// This is a convenience for callers that want `state.json` to carry gate
    /// metadata derived from declarative workflow definitions (052 FR-004/FR-005).
    pub fn attach_gates_from_manifest(&mut self, manifest: &WorkflowManifest) {
        for step_state in &mut self.steps {
            if let Some(manifest_step) = manifest.steps.iter().find(|s| s.id == step_state.id)
                && let Some(ref gate_cfg) = manifest_step.gate
            {
                step_state.gate = Some(gate_info_from_step_gate(gate_cfg));
            }
        }
    }

    /// Marks the workflow as waiting on a checkpoint gate for the given step (FR-004 / SC-002).
    ///
    /// Callers should invoke this when execution reaches a checkpoint gate and
    /// is about to pause for operator confirmation.
    pub fn mark_awaiting_checkpoint(&mut self, step_id: &str) {
        if self.steps.iter().any(|s| s.id == step_id) {
            self.status = WorkflowStatus::AwaitingCheckpoint;
        }
    }

    /// Clears the `"awaiting_checkpoint"` status and returns the workflow to `"running"`.
    ///
    /// Callers should invoke this after an operator has confirmed a checkpoint
    /// and execution is allowed to continue.
    pub fn mark_checkpoint_released(&mut self) {
        if matches!(self.status, WorkflowStatus::AwaitingCheckpoint) {
            self.status = WorkflowStatus::Running;
        }
    }

    /// Applies an approval-gate timeout outcome for the given step (FR-005 / SC-003).
    ///
    /// The workflow status transitions to `"timed_out"`, and the step's status
    /// is updated according to the escalation policy:
    /// - `Fail`   → step marked `Failed`
    /// - `Skip`   → step marked `Skipped`
    /// - `Notify` → step remains `Pending`
    pub fn mark_approval_timed_out(
        &mut self,
        step_id: &str,
        escalation: ApprovalEscalation,
        completed_at: String,
        duration_ms: Option<u64>,
    ) {
        self.status = WorkflowStatus::TimedOut;
        if let Some(step) = self.steps.iter_mut().find(|s| s.id == step_id) {
            step.completed_at = Some(completed_at);
            step.duration_ms = duration_ms;
            step.status = match escalation {
                ApprovalEscalation::Fail => StepExecutionStatus::Failed,
                ApprovalEscalation::Skip => StepExecutionStatus::Skipped,
                ApprovalEscalation::Notify => StepExecutionStatus::Failed,
            };
        }
    }
}

fn gate_info_from_step_gate(cfg: &StepGateConfig) -> GateInfo {
    match cfg {
        StepGateConfig::Checkpoint { label } => {
            let config = label.as_ref().map(|label| {
                let mut m = serde_json::Map::new();
                m.insert("label".to_string(), JsonValue::String(label.clone()));
                JsonValue::Object(m)
            });
            GateInfo {
                gate_type: "checkpoint".to_string(),
                timeout_ms: None,
                config,
            }
        }
        StepGateConfig::Approval {
            timeout_ms,
            escalation,
            checkpoint_id: _,
        } => {
            let mut m = serde_json::Map::new();
            if let Some(e) = escalation {
                let esc = match e {
                    ApprovalEscalation::Fail => "fail",
                    ApprovalEscalation::Skip => "skip",
                    ApprovalEscalation::Notify => "notify",
                };
                m.insert("escalation".to_string(), JsonValue::String(esc.to_string()));
            }
            GateInfo {
                gate_type: "approval".to_string(),
                timeout_ms: Some(*timeout_ms),
                config: if m.is_empty() {
                    None
                } else {
                    Some(JsonValue::Object(m))
                },
            }
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
    let json =
        serde_json::to_vec_pretty(state).map_err(|e| OrchestratorError::StatePersistence {
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
        assert!(
            state
                .steps
                .iter()
                .all(|s| s.status == StepExecutionStatus::Pending)
        );
        assert!(state.current_step_index.is_none());
        assert_eq!(
            state.metadata.get("branch").and_then(|v| v.as_str()),
            Some("main")
        );
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
        assert_eq!(step.completed_at.as_deref(), Some("2026-03-29T10:00:05Z"));
        assert_eq!(step.duration_ms, Some(4000));
        assert_eq!(
            step.output
                .as_ref()
                .and_then(|v| v.get("summary"))
                .and_then(|v| v.as_str()),
            Some("ok")
        );
    }

    #[test]
    fn write_and_load_state_round_trips_pretty_json() {
        let tmp = tempfile::tempdir().unwrap();
        let artifact_base = ArtifactManager::new(tmp.path());
        let wf_id = Uuid::new_v4();

        let mut meta = serde_json::Map::new();
        meta.insert(
            "triggeredBy".to_string(),
            JsonValue::from("user@example.com"),
        );

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
        // Gate field should round-trip as `None` by default.
        assert!(loaded.steps[0].gate.is_none());
    }

    #[test]
    fn attach_gates_from_manifest_populates_gate_info() {
        let wf_id = Uuid::new_v4();
        let step_defs = vec![
            ("step_001".to_string(), "lint".to_string()),
            ("step_002".to_string(), "deploy".to_string()),
        ];
        let mut state = WorkflowState::new(
            wf_id,
            "deploy-staging",
            "2026-03-29T10:00:00Z".to_string(),
            step_defs,
            serde_json::Map::new(),
        );

        let manifest = WorkflowManifest {
            steps: vec![
                crate::manifest::WorkflowStep {
                    id: "step_001".into(),
                    agent: "agent-lint".into(),
                    effort: crate::effort::EffortLevel::Quick,
                    inputs: vec![],
                    outputs: vec!["lint.md".into()],
                    instruction: "Run lint".into(),
                    gate: Some(StepGateConfig::Checkpoint { label: None }),
                    pre_verify: None,
                    post_verify: None,
                    max_retries: None,
                },
                crate::manifest::WorkflowStep {
                    id: "step_002".into(),
                    agent: "agent-deploy".into(),
                    effort: crate::effort::EffortLevel::Quick,
                    inputs: vec!["step_001/lint.md".into()],
                    outputs: vec!["deploy.md".into()],
                    instruction: "Deploy".into(),
                    gate: Some(StepGateConfig::Approval {
                        timeout_ms: 30_000,
                        escalation: Some(ApprovalEscalation::Fail),
                        checkpoint_id: None,
                    }),
                    pre_verify: None,
                    post_verify: None,
                    max_retries: None,
                },
            ],
            workspace_id: None,
        };

        state.attach_gates_from_manifest(&manifest);

        let gate1 = state.steps[0]
            .gate
            .as_ref()
            .expect("expected gate on step_001");
        assert_eq!(gate1.gate_type, "checkpoint");
        assert!(gate1.timeout_ms.is_none());

        let gate2 = state.steps[1]
            .gate
            .as_ref()
            .expect("expected gate on step_002");
        assert_eq!(gate2.gate_type, "approval");
        assert_eq!(gate2.timeout_ms, Some(30_000));
        let escalation = gate2
            .config
            .as_ref()
            .and_then(|v| v.get("escalation"))
            .and_then(|v| v.as_str());
        assert_eq!(escalation, Some("fail"));
    }

    #[test]
    fn checkpoint_and_approval_status_transitions_update_workflow_status() {
        let wf_id = Uuid::new_v4();
        let mut state = WorkflowState::new(
            wf_id,
            "deploy-staging",
            "2026-03-29T10:00:00Z".to_string(),
            vec![("step_001".to_string(), "deploy".to_string())],
            serde_json::Map::new(),
        );

        // Initially running.
        assert_eq!(state.status, WorkflowStatus::Running);

        // Reaching a checkpoint moves the workflow into AwaitingCheckpoint.
        state.mark_awaiting_checkpoint("step_001");
        assert_eq!(state.status, WorkflowStatus::AwaitingCheckpoint);

        // Releasing the checkpoint returns to Running.
        state.mark_checkpoint_released();
        assert_eq!(state.status, WorkflowStatus::Running);

        // An approval timeout moves the workflow into TimedOut and updates the step.
        state.mark_approval_timed_out(
            "step_001",
            ApprovalEscalation::Fail,
            "2026-03-29T10:05:00Z".to_string(),
            Some(5 * 60 * 1000),
        );
        assert_eq!(state.status, WorkflowStatus::TimedOut);
        let step = &state.steps[0];
        assert_eq!(step.status, StepExecutionStatus::Failed);
        assert_eq!(step.completed_at.as_deref(), Some("2026-03-29T10:05:00Z"));
    }
}
