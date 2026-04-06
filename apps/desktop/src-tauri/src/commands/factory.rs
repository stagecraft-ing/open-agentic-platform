use chrono::Utc;
use factory_engine::{
    record_scaffold_completion, record_scaffold_failure, FactoryAgentBridge, FactoryEngine,
    FactoryEngineConfig, FactoryPipelineState,
};
use orchestrator::{
    detect_resume_plan_for_run, dispatch_manifest, materialize_run_directory, AgentPromptLookup,
    ArtifactManager, ClaudeCodeExecutor, DispatchOptions, GateHandler,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Serde types — frontend-facing shapes (preserved for React compatibility)
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
// TauriGateHandler — bridges orchestrator gates to Tauri events + oneshots
// ---------------------------------------------------------------------------

struct TauriGateHandler {
    app: AppHandle,
    pending: Mutex<HashMap<String, tokio::sync::oneshot::Sender<Result<(), String>>>>,
}

impl TauriGateHandler {
    fn new(app: AppHandle) -> Self {
        Self {
            app,
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Resolve a pending gate as approved.
    fn approve(&self, step_id: &str) -> Result<(), String> {
        let mut pending = self.pending.lock().map_err(|e| e.to_string())?;
        if let Some(tx) = pending.remove(step_id) {
            tx.send(Ok(())).map_err(|_| "gate channel closed".to_string())
        } else {
            Err(format!("no pending gate for step {step_id}"))
        }
    }

    /// Resolve a pending gate as rejected.
    fn reject(&self, step_id: &str, feedback: &str) -> Result<(), String> {
        let mut pending = self.pending.lock().map_err(|e| e.to_string())?;
        if let Some(tx) = pending.remove(step_id) {
            tx.send(Err(format!("rejected: {feedback}")))
                .map_err(|_| "gate channel closed".to_string())
        } else {
            Err(format!("no pending gate for step {step_id}"))
        }
    }
}

#[async_trait::async_trait]
impl GateHandler for TauriGateHandler {
    async fn await_checkpoint(&self, step_id: &str, label: Option<&str>) -> Result<(), String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending.lock().map_err(|e| e.to_string())?;
            pending.insert(step_id.to_string(), tx);
        }
        self.app
            .emit(
                "factory:gate_reached",
                &serde_json::json!({
                    "stepId": step_id,
                    "gateType": "checkpoint",
                    "label": label,
                }),
            )
            .map_err(|e| format!("emit gate_reached failed: {e}"))?;

        rx.await.map_err(|_| "gate channel closed".to_string())?
    }

    async fn await_approval(&self, step_id: &str, timeout_ms: u64) -> Result<(), String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending.lock().map_err(|e| e.to_string())?;
            pending.insert(step_id.to_string(), tx);
        }
        self.app
            .emit(
                "factory:gate_reached",
                &serde_json::json!({
                    "stepId": step_id,
                    "gateType": "approval",
                    "timeoutMs": timeout_ms,
                }),
            )
            .map_err(|e| format!("emit gate_reached failed: {e}"))?;

        // The orchestrator's evaluate_gate wraps this call in tokio::time::timeout,
        // so we just wait for the UI to resolve the oneshot.
        rx.await.map_err(|_| "gate channel closed".to_string())?
    }
}

// ---------------------------------------------------------------------------
// BridgeLookup — adapts FactoryAgentBridge to AgentPromptLookup
// ---------------------------------------------------------------------------

struct BridgeLookup(Arc<FactoryAgentBridge>);

impl AgentPromptLookup for BridgeLookup {
    fn get_prompt(&self, agent_id: &str) -> Option<String> {
        self.0.get_prompt(agent_id).map(String::from)
    }
}

// ---------------------------------------------------------------------------
// Run context — replaces the fake in-memory state machine
// ---------------------------------------------------------------------------

struct FactoryRunContext {
    run_id: Uuid,
    gate_handler: Arc<TauriGateHandler>,
    pipeline_state: Mutex<FactoryPipelineState>,
    project_path: PathBuf,
    adapter_name: String,
    audit_trail: Mutex<Vec<AuditEntry>>,
    stage_status: Mutex<HashMap<String, StageTracker>>,
}

#[derive(Clone, Debug)]
struct StageTracker {
    status: String,
    token_spend: u64,
    artifacts: Vec<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
}

static FACTORY_RUNS: LazyLock<Mutex<HashMap<String, Arc<FactoryRunContext>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ---------------------------------------------------------------------------
// Process stage constants (the 6 Factory pipeline stages)
// ---------------------------------------------------------------------------

const PROCESS_STAGES: &[(&str, &str)] = &[
    ("s0-preflight", "Pre-flight"),
    ("s1-business-requirements", "Business Requirements"),
    ("s2-service-requirements", "Service Requirements"),
    ("s3-data-model", "Data Model"),
    ("s4-api-specification", "API Specification"),
    ("s5-ui-specification", "UI Specification"),
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
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

/// Locate the factory/ directory by walking up from the project path or CWD.
fn resolve_factory_root() -> Result<PathBuf, String> {
    // First try relative to the repo root (common case).
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join("factory"),
        PathBuf::from("factory"),
    ];
    for candidate in &candidates {
        if let Ok(p) = candidate.canonicalize() {
            if p.join("adapters").is_dir() {
                return Ok(p);
            }
        }
    }
    Err("factory/ directory not found. Ensure the repository contains a factory/ directory with adapters/".into())
}

/// Build a PipelineStatusResponse from the live run context.
fn build_status_response(ctx: &FactoryRunContext) -> PipelineStatusResponse {
    let pipeline_state = ctx.pipeline_state.lock().unwrap();
    let stage_status = ctx.stage_status.lock().unwrap();
    let audit_trail = ctx.audit_trail.lock().unwrap();

    let phase = match pipeline_state.phase {
        factory_engine::FactoryPhase::Process => "process",
        factory_engine::FactoryPhase::Scaffolding => "scaffolding",
        factory_engine::FactoryPhase::Complete => "complete",
        factory_engine::FactoryPhase::Failed => "failed",
    };

    let stages: Vec<StageInfo> = PROCESS_STAGES
        .iter()
        .map(|(id, name)| {
            let tracker = stage_status.get(*id);
            StageInfo {
                id: id.to_string(),
                name: name.to_string(),
                status: tracker
                    .map(|t| t.status.clone())
                    .unwrap_or_else(|| "pending".into()),
                token_spend: tracker.map(|t| t.token_spend).unwrap_or(0),
                artifacts: tracker
                    .map(|t| t.artifacts.clone())
                    .unwrap_or_default(),
                started_at: tracker.and_then(|t| t.started_at.clone()),
                completed_at: tracker.and_then(|t| t.completed_at.clone()),
            }
        })
        .collect();

    // Build scaffolding info from pipeline state if in scaffolding phase.
    let scaffolding = pipeline_state.scaffolding.as_ref().map(|sp| {
        let mut categories = Vec::new();
        // Entities
        categories.push(CategoryInfo {
            category: "data".into(),
            total: sp.entities_completed.len() + sp.entities_failed.len(),
            completed: sp.entities_completed.len(),
            failed: sp.entities_failed.len(),
            in_progress: 0,
            steps: sp
                .entities_failed
                .iter()
                .map(|f| StepInfo {
                    id: f.step_id.clone(),
                    category: "data".into(),
                    feature_name: f.name.clone(),
                    status: "failed".into(),
                    retry_count: f.retries,
                    max_retries: 3,
                    last_error: Some(f.last_error.clone()),
                    token_spend: 0,
                })
                .collect(),
        });
        // Operations
        categories.push(CategoryInfo {
            category: "api".into(),
            total: sp.operations_completed.len() + sp.operations_failed.len(),
            completed: sp.operations_completed.len(),
            failed: sp.operations_failed.len(),
            in_progress: 0,
            steps: sp
                .operations_failed
                .iter()
                .map(|f| StepInfo {
                    id: f.step_id.clone(),
                    category: "api".into(),
                    feature_name: f.name.clone(),
                    status: "failed".into(),
                    retry_count: f.retries,
                    max_retries: 3,
                    last_error: Some(f.last_error.clone()),
                    token_spend: 0,
                })
                .collect(),
        });
        // Pages
        categories.push(CategoryInfo {
            category: "ui".into(),
            total: sp.pages_completed.len() + sp.pages_failed.len(),
            completed: sp.pages_completed.len(),
            failed: sp.pages_failed.len(),
            in_progress: 0,
            steps: sp
                .pages_failed
                .iter()
                .map(|f| StepInfo {
                    id: f.step_id.clone(),
                    category: "ui".into(),
                    feature_name: f.name.clone(),
                    status: "failed".into(),
                    retry_count: f.retries,
                    max_retries: 3,
                    last_error: Some(f.last_error.clone()),
                    token_spend: 0,
                })
                .collect(),
        });
        ScaffoldingInfo {
            categories,
            active_step_id: None,
        }
    });

    PipelineStatusResponse {
        run_id: ctx.run_id.to_string(),
        phase: phase.to_string(),
        stages,
        scaffolding,
        total_tokens: pipeline_state.total_tokens,
        audit_trail: audit_trail.clone(),
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Start a new Factory pipeline run.
///
/// Creates a real `FactoryEngine`, generates a Phase 1 manifest, and dispatches
/// it via the orchestrator's `dispatch_manifest`. Gates are bridged to Tauri
/// events so the React UI can confirm/reject them.
#[tauri::command]
pub async fn start_factory_pipeline(
    app: AppHandle,
    project_path: String,
    adapter_name: String,
    business_doc_paths: Vec<String>,
) -> Result<StartPipelineResponse, String> {
    let factory_root = resolve_factory_root()?;
    let project_path = PathBuf::from(&project_path);
    let doc_paths: Vec<PathBuf> = business_doc_paths.iter().map(PathBuf::from).collect();

    // Ensure project directory exists.
    std::fs::create_dir_all(&project_path)
        .map_err(|e| format!("create project dir failed: {e}"))?;
    let project_path = project_path
        .canonicalize()
        .map_err(|e| format!("resolve project path failed: {e}"))?;

    // Build engine and start pipeline.
    let config = FactoryEngineConfig {
        factory_root: factory_root.clone(),
        project_path: project_path.clone(),
        concurrency_limit: 4,
        max_total_tokens: None,
    };
    let engine = FactoryEngine::new(config).map_err(|e| e.to_string())?;
    let start = engine
        .start_pipeline(&adapter_name, &doc_paths)
        .map_err(|e| e.to_string())?;

    let run_id = start.run_id;
    let run_id_str = run_id.to_string();

    // Set up artifact manager under the project directory.
    let artifact_dir = project_path.join(".factory").join("runs");
    let am = ArtifactManager::new(&artifact_dir);
    materialize_run_directory(&am, run_id, &start.manifest)
        .map_err(|e| format!("materialize run dir failed: {e}"))?;

    // Create gate handler and run context.
    let gate_handler = Arc::new(TauriGateHandler::new(app.clone()));
    let bridge = Arc::new(start.agent_bridge);

    let initial_audit = AuditEntry {
        timestamp: now_iso(),
        action: "pipeline_started".into(),
        stage_id: None,
        details: Some(format!(
            "adapter={} docs={}",
            adapter_name,
            business_doc_paths.join(",")
        )),
        feedback: None,
    };

    let mut initial_stages = HashMap::new();
    for (id, _name) in PROCESS_STAGES {
        initial_stages.insert(
            id.to_string(),
            StageTracker {
                status: "pending".into(),
                token_spend: 0,
                artifacts: vec![],
                started_at: None,
                completed_at: None,
            },
        );
    }

    let ctx = Arc::new(FactoryRunContext {
        run_id,
        gate_handler: gate_handler.clone(),
        pipeline_state: Mutex::new(start.pipeline_state),
        project_path: project_path.clone(),
        adapter_name: adapter_name.clone(),
        audit_trail: Mutex::new(vec![initial_audit]),
        stage_status: Mutex::new(initial_stages),
    });

    FACTORY_RUNS
        .lock()
        .map_err(|e| e.to_string())?
        .insert(run_id_str.clone(), ctx.clone());

    // Emit start event immediately.
    app.emit(
        "factory:workflow_started",
        &serde_json::json!({
            "runId": run_id_str,
            "adapter": adapter_name,
            "projectPath": project_path.to_string_lossy(),
            "startedAt": now_iso(),
        }),
    )
    .map_err(|e| format!("emit factory:workflow_started failed: {e}"))?;

    // Spawn the dispatch in the background so the command returns immediately.
    let app_handle = app.clone();
    let manifest = start.manifest;
    let adapter_for_spawn = adapter_name.clone();
    let ctx_for_spawn = ctx.clone();

    tokio::spawn(async move {
        // Build executor with agent prompt lookup.
        let lookup = Arc::new(BridgeLookup(bridge.clone()));
        let executor = Arc::new(
            ClaudeCodeExecutor::new(project_path.clone())
                .with_prompt_lookup(lookup)
                .with_max_turns(25),
        );

        let options = DispatchOptions {
            gate_handler: Some(gate_handler.clone() as Arc<dyn GateHandler>),
            project_root: Some(project_path.clone()),
            skip_completed_steps: HashSet::new(),
        };

        // Dispatch Phase 1 (s0–s5).
        let summary1 = match dispatch_manifest(
            &am,
            run_id,
            &manifest,
            bridge.clone(),
            executor.clone(),
            &options,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                ctx_for_spawn.pipeline_state.lock().unwrap().mark_failed();
                app_handle
                    .emit(
                        "factory:workflow_failed",
                        &serde_json::json!({
                            "runId": run_id.to_string(),
                            "error": e.to_string(),
                            "phase": "process",
                        }),
                    )
                    .ok();
                return;
            }
        };

        // Update stage tracking from summary.
        {
            let mut stages = ctx_for_spawn.stage_status.lock().unwrap();
            for step in &summary1.steps {
                if let Some(tracker) = stages.get_mut(&step.step_id) {
                    tracker.status = match step.status {
                        orchestrator::StepStatus::Success => "completed",
                        orchestrator::StepStatus::Failure => "failed",
                        orchestrator::StepStatus::Skipped => "skipped",
                        orchestrator::StepStatus::Cancelled => "cancelled",
                        _ => "completed",
                    }
                    .into();
                    tracker.token_spend = step.tokens_used.unwrap_or(0);
                    tracker.completed_at = Some(now_iso());
                }
            }
            let mut ps = ctx_for_spawn.pipeline_state.lock().unwrap();
            let phase1_tokens: u64 = summary1.steps.iter().filter_map(|s| s.tokens_used).sum();
            ps.add_tokens(phase1_tokens);
        }

        app_handle
            .emit(
                "factory:phase1_completed",
                &serde_json::json!({
                    "runId": run_id.to_string(),
                    "steps": summary1.steps.len(),
                }),
            )
            .ok();

        // Phase transition: read frozen Build Spec, generate Phase 2 manifest.
        let build_spec_path =
            am.output_artifact_path(run_id, "s5-ui-specification", "build-spec.yaml");
        if !build_spec_path.exists() {
            ctx_for_spawn.pipeline_state.lock().unwrap().mark_failed();
            app_handle
                .emit(
                    "factory:workflow_failed",
                    &serde_json::json!({
                        "runId": run_id.to_string(),
                        "error": format!("Build Spec not found at {}", build_spec_path.display()),
                        "phase": "transition",
                    }),
                )
                .ok();
            return;
        }

        let transition = {
            let mut ps = ctx_for_spawn.pipeline_state.lock().unwrap();
            match engine.transition_to_scaffolding(
                &adapter_for_spawn,
                &build_spec_path,
                &mut ps,
                None, // org_override
            ) {
                Ok(t) => t,
                Err(e) => {
                    ps.mark_failed();
                    app_handle
                        .emit(
                            "factory:workflow_failed",
                            &serde_json::json!({
                                "runId": run_id.to_string(),
                                "error": e.to_string(),
                                "phase": "transition",
                            }),
                        )
                        .ok();
                    return;
                }
            }
        };

        app_handle
            .emit(
                "factory:phase_transition",
                &serde_json::json!({
                    "runId": run_id.to_string(),
                    "phase": "scaffolding",
                    "totalScaffoldSteps": transition.manifest.steps.len(),
                }),
            )
            .ok();

        // Materialize Phase 2 run directory.
        if let Err(e) = materialize_run_directory(&am, run_id, &transition.manifest) {
            ctx_for_spawn.pipeline_state.lock().unwrap().mark_failed();
            app_handle
                .emit(
                    "factory:workflow_failed",
                    &serde_json::json!({
                        "runId": run_id.to_string(),
                        "error": format!("materialize phase 2 failed: {e}"),
                        "phase": "scaffolding",
                    }),
                )
                .ok();
            return;
        }

        // Dispatch Phase 2 (s6a–s6g).
        let phase2_options = DispatchOptions {
            gate_handler: Some(gate_handler as Arc<dyn GateHandler>),
            project_root: Some(project_path.clone()),
            skip_completed_steps: HashSet::new(),
        };

        let summary2 = match dispatch_manifest(
            &am,
            run_id,
            &transition.manifest,
            bridge,
            executor,
            &phase2_options,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                ctx_for_spawn.pipeline_state.lock().unwrap().mark_failed();
                app_handle
                    .emit(
                        "factory:workflow_failed",
                        &serde_json::json!({
                            "runId": run_id.to_string(),
                            "error": e.to_string(),
                            "phase": "scaffolding",
                        }),
                    )
                    .ok();
                return;
            }
        };

        // Update pipeline state from Phase 2 results.
        {
            let mut ps = ctx_for_spawn.pipeline_state.lock().unwrap();
            for step in &summary2.steps {
                let tokens = step.tokens_used.unwrap_or(0);
                match step.status {
                    orchestrator::StepStatus::Success => {
                        record_scaffold_completion(&mut ps, &step.step_id, tokens);
                    }
                    orchestrator::StepStatus::Failure => {
                        record_scaffold_failure(
                            &mut ps,
                            &step.step_id,
                            3, // max retries
                            "step failed after retries",
                        );
                    }
                    _ => {}
                }
            }
            ps.mark_complete();
        }

        // Persist final state to disk.
        let state_path = project_path
            .join(".factory")
            .join("runs")
            .join(run_id.to_string())
            .join("pipeline-state.json");
        if let Ok(ps) = ctx_for_spawn.pipeline_state.lock() {
            let _ = ps.save_to_file(&state_path);
        }

        // Also write a state.json for list_factory_runs discovery.
        let summary_path = project_path
            .join(".factory")
            .join("runs")
            .join(run_id.to_string())
            .join("state.json");
        let run_summary = PipelineRunSummary {
            run_id: run_id.to_string(),
            adapter: adapter_for_spawn,
            project_path: project_path.to_string_lossy().into(),
            started_at: now_iso(),
            completed_at: Some(now_iso()),
            phase: "complete".into(),
            total_tokens: ctx_for_spawn
                .pipeline_state
                .lock()
                .map(|ps| ps.total_tokens)
                .unwrap_or(0),
        };
        let _ = std::fs::write(
            &summary_path,
            serde_json::to_string_pretty(&run_summary).unwrap_or_default(),
        );

        app_handle
            .emit(
                "factory:workflow_completed",
                &serde_json::json!({
                    "runId": run_id.to_string(),
                    "totalSteps": summary1.steps.len() + summary2.steps.len(),
                    "totalTokens": ctx_for_spawn.pipeline_state.lock().map(|ps| ps.total_tokens).unwrap_or(0),
                }),
            )
            .ok();
    });

    Ok(StartPipelineResponse { run_id: run_id_str })
}

/// Return the current status of a pipeline run.
#[tauri::command]
pub async fn get_factory_pipeline_status(
    run_id: String,
) -> Result<PipelineStatusResponse, String> {
    let runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
    if let Some(ctx) = runs.get(&run_id) {
        return Ok(build_status_response(ctx));
    }

    // Fallback: try loading persisted state from disk for completed runs.
    // Walk known artifact directories to find the pipeline-state.json.
    Err(format!("run not found: {run_id}"))
}

/// Confirm a gate stage. Resolves the pending oneshot in TauriGateHandler.
#[tauri::command]
pub async fn confirm_factory_stage(
    _app: AppHandle,
    run_id: String,
    stage_id: String,
) -> Result<(), String> {
    let runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
    let ctx = runs
        .get(&run_id)
        .ok_or_else(|| format!("run not found: {run_id}"))?;

    ctx.gate_handler.approve(&stage_id)?;

    // Record in audit trail.
    ctx.audit_trail.lock().unwrap().push(AuditEntry {
        timestamp: now_iso(),
        action: "gate_confirmed".into(),
        stage_id: Some(stage_id),
        details: None,
        feedback: None,
    });

    Ok(())
}

/// Reject a gate stage. Resolves the pending oneshot with an error.
#[tauri::command]
pub async fn reject_factory_stage(
    app: AppHandle,
    run_id: String,
    stage_id: String,
    feedback: String,
) -> Result<(), String> {
    let runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
    let ctx = runs
        .get(&run_id)
        .ok_or_else(|| format!("run not found: {run_id}"))?;

    ctx.gate_handler.reject(&stage_id, &feedback)?;

    ctx.audit_trail.lock().unwrap().push(AuditEntry {
        timestamp: now_iso(),
        action: "stage_rejected".into(),
        stage_id: Some(stage_id.clone()),
        details: None,
        feedback: Some(feedback.clone()),
    });

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

/// List all Factory pipeline runs by scanning `<project_path>/.factory/runs/*/state.json`.
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

    let entries =
        std::fs::read_dir(&runs_dir).map_err(|e| format!("read .factory/runs failed: {e}"))?;

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

    summaries.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(summaries)
}

/// List artifact files for a given run/step combination.
#[tauri::command]
pub async fn get_factory_artifacts(
    run_id: String,
    step_id: String,
) -> Result<Vec<ArtifactInfo>, String> {
    // Resolve project path from live context if available.
    let project_path = FACTORY_RUNS
        .lock()
        .map_err(|e| e.to_string())?
        .get(&run_id)
        .map(|ctx| ctx.project_path.clone());

    let base = if let Some(p) = project_path {
        p.join(".factory")
            .join("runs")
            .join(&run_id)
            .join(&step_id)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        PathBuf::from(home)
            .join(".oap")
            .join("artifacts")
            .join(&run_id)
            .join(&step_id)
    };

    if !base.exists() {
        return Ok(vec![]);
    }

    let entries =
        std::fs::read_dir(&base).map_err(|e| format!("read artifact dir failed: {e}"))?;

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

/// Mark a failed scaffold step as skipped.
#[tauri::command]
pub async fn skip_factory_step(
    app: AppHandle,
    run_id: String,
    step_id: String,
) -> Result<(), String> {
    let runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
    let ctx = runs
        .get(&run_id)
        .ok_or_else(|| format!("run not found: {run_id}"))?;

    ctx.audit_trail.lock().unwrap().push(AuditEntry {
        timestamp: now_iso(),
        action: "step_skipped".into(),
        stage_id: Some(step_id.clone()),
        details: None,
        feedback: None,
    });

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

/// Resume a previously failed pipeline run.
#[tauri::command]
pub async fn resume_factory_pipeline(
    app: AppHandle,
    run_id: String,
    project_path: String,
    adapter_name: String,
) -> Result<(), String> {
    let factory_root = resolve_factory_root()?;
    let project_path = PathBuf::from(&project_path)
        .canonicalize()
        .map_err(|e| format!("resolve project path failed: {e}"))?;
    let run_uuid =
        Uuid::parse_str(&run_id).map_err(|e| format!("invalid run_id: {e}"))?;

    let config = FactoryEngineConfig {
        factory_root: factory_root.clone(),
        project_path: project_path.clone(),
        concurrency_limit: 4,
        max_total_tokens: None,
    };
    let engine = FactoryEngine::new(config).map_err(|e| e.to_string())?;
    let start = engine
        .start_pipeline(&adapter_name, &[])
        .map_err(|e| e.to_string())?;

    let artifact_dir = project_path.join(".factory").join("runs");
    let am = ArtifactManager::new(&artifact_dir);

    // Detect which steps are already completed.
    let skip_steps: HashSet<String> =
        match detect_resume_plan_for_run(&am, run_uuid, &start.manifest) {
            Ok(Some(plan)) => plan.completed_step_ids.into_iter().collect(),
            _ => HashSet::new(),
        };

    let gate_handler = Arc::new(TauriGateHandler::new(app.clone()));
    let bridge = Arc::new(start.agent_bridge);
    let lookup = Arc::new(BridgeLookup(bridge.clone()));
    let executor = Arc::new(
        ClaudeCodeExecutor::new(project_path.clone())
            .with_prompt_lookup(lookup)
            .with_max_turns(25),
    );

    let options = DispatchOptions {
        gate_handler: Some(gate_handler as Arc<dyn GateHandler>),
        project_root: Some(project_path),
        skip_completed_steps: skip_steps,
    };

    let app_handle = app.clone();
    tokio::spawn(async move {
        match dispatch_manifest(&am, run_uuid, &start.manifest, bridge, executor, &options).await {
            Ok(_summary) => {
                app_handle
                    .emit(
                        "factory:workflow_completed",
                        &serde_json::json!({ "runId": run_id }),
                    )
                    .ok();
            }
            Err(e) => {
                app_handle
                    .emit(
                        "factory:workflow_failed",
                        &serde_json::json!({
                            "runId": run_id,
                            "error": e.to_string(),
                        }),
                    )
                    .ok();
            }
        }
    });

    Ok(())
}
