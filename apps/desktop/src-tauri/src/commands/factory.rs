use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Serde types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartPipelineResponse {
    pub run_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineStatusResponse {
    pub run_id: String,
    pub phase: String, // "idle" | "process" | "scaffolding" | "complete" | "failed"
    pub stages: Vec<StageInfo>,
    pub scaffolding: Option<ScaffoldingInfo>,
    pub total_tokens: u64,
    pub audit_trail: Vec<AuditEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageInfo {
    pub id: String,
    pub name: String,
    pub status: String, // "pending" | "in_progress" | "completed" | "failed" | "awaiting_gate"
    pub token_spend: u64,
    pub artifacts: Vec<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScaffoldingInfo {
    pub categories: Vec<CategoryInfo>,
    pub active_step_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryInfo {
    pub category: String,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub in_progress: usize,
    pub steps: Vec<StepInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StepInfo {
    pub id: String,
    pub category: String,
    pub feature_name: String,
    pub status: String,
    pub retry_count: u32,
    pub max_retries: u32,
    pub last_error: Option<String>,
    pub token_spend: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub mime_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: String,
    pub stage_id: Option<String>,
    pub details: Option<String>,
    pub feedback: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PipelineRunSummary {
    pub run_id: String,
    pub adapter: String,
    pub project_path: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub phase: String,
    pub total_tokens: u64,
}

// ---------------------------------------------------------------------------
// In-memory state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct FactoryRunState {
    status: PipelineStatusResponse,
    project_path: String,
    adapter: String,
    started_at: String,
}

static FACTORY_RUNS: LazyLock<Mutex<HashMap<String, FactoryRunState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ---------------------------------------------------------------------------
// Process stage constants  (the 6 Factory pipeline stages)
// ---------------------------------------------------------------------------

const PROCESS_STAGES: &[(&str, &str)] = &[
    ("s0", "Pre-flight"),
    ("s1", "Business Requirements"),
    ("s2", "Service Requirements"),
    ("s3", "Data Model"),
    ("s4", "API Specification"),
    ("s5", "UI Specification"),
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn initial_stages() -> Vec<StageInfo> {
    PROCESS_STAGES
        .iter()
        .map(|(id, name)| StageInfo {
            id: id.to_string(),
            name: name.to_string(),
            status: "pending".to_string(),
            token_spend: 0,
            artifacts: vec![],
            started_at: None,
            completed_at: None,
        })
        .collect()
}

fn mime_from_ext(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "md" => "markdown",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        _ => "text",
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Start a new Factory pipeline run. Creates an in-memory run state and emits
/// `factory:workflow_started`.
#[tauri::command]
pub async fn start_factory_pipeline(
    app: AppHandle,
    project_path: String,
    adapter_name: String,
    business_doc_paths: Vec<String>,
) -> Result<StartPipelineResponse, String> {
    let run_id = Uuid::new_v4().to_string();
    let started_at = now_iso();

    let initial_audit = AuditEntry {
        timestamp: started_at.clone(),
        action: "pipeline_started".to_string(),
        stage_id: None,
        details: Some(format!(
            "adapter={} docs={}",
            adapter_name,
            business_doc_paths.join(",")
        )),
        feedback: None,
    };

    let status = PipelineStatusResponse {
        run_id: run_id.clone(),
        phase: "process".to_string(),
        stages: initial_stages(),
        scaffolding: None,
        total_tokens: 0,
        audit_trail: vec![initial_audit],
    };

    let run_state = FactoryRunState {
        status: status.clone(),
        project_path: project_path.clone(),
        adapter: adapter_name.clone(),
        started_at: started_at.clone(),
    };

    FACTORY_RUNS
        .lock()
        .map_err(|e| e.to_string())?
        .insert(run_id.clone(), run_state);

    app.emit(
        "factory:workflow_started",
        &serde_json::json!({
            "runId": run_id,
            "adapter": adapter_name,
            "projectPath": project_path,
            "startedAt": started_at,
        }),
    )
    .map_err(|e| format!("emit factory:workflow_started failed: {e}"))?;

    Ok(StartPipelineResponse { run_id })
}

/// Return the current status of a pipeline run from the in-memory map.
#[tauri::command]
pub async fn get_factory_pipeline_status(
    run_id: String,
) -> Result<PipelineStatusResponse, String> {
    let runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
    runs.get(&run_id)
        .map(|s| s.status.clone())
        .ok_or_else(|| format!("run not found: {run_id}"))
}

/// Confirm a gate stage. Advances the stage status and emits
/// `factory:step_started` for the next stage.
#[tauri::command]
pub async fn confirm_factory_stage(
    app: AppHandle,
    run_id: String,
    stage_id: String,
) -> Result<(), String> {
    let next_stage = {
        let mut runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
        let state = runs
            .get_mut(&run_id)
            .ok_or_else(|| format!("run not found: {run_id}"))?;

        let now = now_iso();

        // Mark the confirmed stage as completed.
        let mut next_idx: Option<usize> = None;
        for (idx, stage) in state.status.stages.iter_mut().enumerate() {
            if stage.id == stage_id {
                stage.status = "completed".to_string();
                stage.completed_at = Some(now.clone());
                next_idx = Some(idx + 1);
                break;
            }
        }

        // Advance the next stage to in_progress.
        let mut next_id: Option<String> = None;
        if let Some(next) = next_idx.and_then(|i| state.status.stages.get_mut(i)) {
            next.status = "in_progress".to_string();
            next.started_at = Some(now.clone());
            next_id = Some(next.id.clone());
        }

        // Append audit entry.
        state.status.audit_trail.push(AuditEntry {
            timestamp: now.clone(),
            action: "gate_confirmed".to_string(),
            stage_id: Some(stage_id.clone()),
            details: None,
            feedback: None,
        });

        // Update phase to complete when all stages are done.
        if state.status.stages.iter().all(|s| s.status == "completed") {
            state.status.phase = "complete".to_string();
        }

        next_id
    };

    if let Some(next_id) = next_stage {
        app.emit(
            "factory:step_started",
            &serde_json::json!({
                "runId": run_id,
                "stepId": next_id,
                "stepName": next_id,
            }),
        )
        .map_err(|e| format!("emit factory:step_started failed: {e}"))?;
    }

    Ok(())
}

/// Reject a gate stage. Records feedback and emits `factory:stage_rejected`.
#[tauri::command]
pub async fn reject_factory_stage(
    app: AppHandle,
    run_id: String,
    stage_id: String,
    feedback: String,
) -> Result<(), String> {
    {
        let mut runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
        let state = runs
            .get_mut(&run_id)
            .ok_or_else(|| format!("run not found: {run_id}"))?;

        let now = now_iso();

        // Rejection resets the stage to pending for re-run (FR-004).
        for stage in state.status.stages.iter_mut() {
            if stage.id == stage_id {
                stage.status = "pending".to_string();
                stage.completed_at = None;
                stage.started_at = None;
                break;
            }
        }

        // Phase stays "process" — rejection is not failure.

        state.status.audit_trail.push(AuditEntry {
            timestamp: now,
            action: "stage_rejected".to_string(),
            stage_id: Some(stage_id.clone()),
            details: None,
            feedback: Some(feedback.clone()),
        });
    }

    app.emit(
        "factory:stage_rejected",
        &serde_json::json!({
            "runId": run_id,
            "stageId": stage_id,
            "feedback": feedback,
        }),
    )
    .map_err(|e| format!("emit factory:stage_rejected failed: {e}"))?;

    Ok(())
}

/// List all Factory pipeline runs for a project by scanning
/// `<project_path>/.factory/runs/*/state.json`. Returns an empty vec when the
/// directory does not exist.
#[tauri::command]
pub async fn list_factory_runs(
    project_path: String,
) -> Result<Vec<PipelineRunSummary>, String> {
    let runs_dir = std::path::Path::new(&project_path)
        .join(".factory")
        .join("runs");

    if !runs_dir.exists() {
        return Ok(vec![]);
    }

    let entries = std::fs::read_dir(&runs_dir)
        .map_err(|e| format!("read .factory/runs failed: {e}"))?;

    let mut summaries = Vec::new();
    for entry in entries.flatten() {
        let state_path = entry.path().join("state.json");
        if !state_path.exists() {
            continue;
        }
        let text = match std::fs::read_to_string(&state_path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if let Ok(summary) = serde_json::from_str::<PipelineRunSummary>(&text) {
            summaries.push(summary);
        }
    }

    // Most-recently started first.
    summaries.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(summaries)
}

/// List artifact files for a given run/step combination.
/// Scans `<artifacts_root>/<run_id>/<step_id>/`.
#[tauri::command]
pub async fn get_factory_artifacts(
    run_id: String,
    step_id: String,
) -> Result<Vec<ArtifactInfo>, String> {
    // Resolve project path from in-memory state if available.
    let project_path = FACTORY_RUNS
        .lock()
        .map_err(|e| e.to_string())?
        .get(&run_id)
        .map(|s| s.project_path.clone());

    let base = if let Some(p) = project_path {
        std::path::PathBuf::from(p)
            .join(".factory")
            .join("runs")
            .join(&run_id)
            .join(&step_id)
    } else {
        // Fallback: try the OAP artifact base directory convention.
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        std::path::PathBuf::from(home)
            .join(".oap")
            .join("artifacts")
            .join(&run_id)
            .join(&step_id)
    };

    if !base.exists() {
        return Ok(vec![]);
    }

    let entries = std::fs::read_dir(&base)
        .map_err(|e| format!("read artifact dir failed: {e}"))?;

    let mut artifacts = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        let mime_type = mime_from_ext(&name).to_string();
        artifacts.push(ArtifactInfo {
            name,
            path: path.to_string_lossy().to_string(),
            size,
            mime_type,
        });
    }

    artifacts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(artifacts)
}

/// Mark a failed scaffold step as skipped and emit `factory:step_skipped`.
#[tauri::command]
pub async fn skip_factory_step(
    app: AppHandle,
    run_id: String,
    step_id: String,
) -> Result<(), String> {
    {
        let mut runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
        let state = runs
            .get_mut(&run_id)
            .ok_or_else(|| format!("run not found: {run_id}"))?;

        let now = now_iso();
        let mut found = false;

        // Look in scaffolding steps.
        if let Some(ref mut scaffolding) = state.status.scaffolding {
            'outer: for category in scaffolding.categories.iter_mut() {
                for step in category.steps.iter_mut() {
                    if step.id == step_id {
                        step.status = "skipped".to_string();
                        found = true;
                        break 'outer;
                    }
                }
            }
        }

        if !found {
            return Err(format!("step not found: {step_id}"));
        }

        state.status.audit_trail.push(AuditEntry {
            timestamp: now,
            action: "step_skipped".to_string(),
            stage_id: Some(step_id.clone()),
            details: None,
            feedback: None,
        });
    }

    app.emit(
        "factory:step_skipped",
        &serde_json::json!({
            "runId": run_id,
            "stepId": step_id,
        }),
    )
    .map_err(|e| format!("emit factory:step_skipped failed: {e}"))?;

    Ok(())
}
