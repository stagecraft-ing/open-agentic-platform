use chrono::Utc;
use factory_engine::{
    FactoryAgentBridge, FactoryEngine, FactoryEngineConfig, FactoryPipelineState,
    record_scaffold_completion, record_scaffold_failure,
};
use orchestrator::{
    AgentPromptLookup, ArtifactManager, ClaudeCodeExecutor, DispatchOptions, GateHandler,
    StepEvent, StepEventHandler, detect_resume_plan_for_run, dispatch_manifest,
    materialize_run_directory, promotion::SyncTracker,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use super::keychain::clone_token_load;
use super::stagecraft_client::{StagecraftClient, StagecraftState};
use super::sync_client::{
    FnHandler, KnowledgeBundle as WireKnowledgeBundle, ServerEnvelopeWire, SyncClientState,
};

/// Spec 112 §6.4.5 — load the project's clone token from the OS
/// keychain and surface it as `{ GITHUB_TOKEN: <value> }` for the
/// factory engine subprocess. Empty map when:
///   * no `project_id` is supplied (local-only pipeline run, no
///     stagecraft binding),
///   * no clone token is stored (public repo, anonymous run), or
///   * the keychain read errors (logged once, run continues
///     anonymously — surfaces as 401 from GitHub if the run actually
///     needs auth, and the cockpit's refresh path takes over).
fn clone_token_env_for_project(project_id: Option<&str>) -> HashMap<String, String> {
    let Some(pid) = project_id else {
        return HashMap::new();
    };
    match clone_token_load(pid.to_string()) {
        Ok(Some(stored)) => {
            let mut env = HashMap::new();
            env.insert("GITHUB_TOKEN".to_string(), stored.value);
            env
        }
        Ok(None) => HashMap::new(),
        Err(err) => {
            eprintln!(
                "[factory] clone token load failed for project {pid}: {err}; \
                 starting run without GITHUB_TOKEN"
            );
            HashMap::new()
        }
    }
}

// ---------------------------------------------------------------------------
// Step event handler — bridges orchestrator's StepEvent stream to Tauri events
// so the desktop UI can render terminal-style live output during execution.
// Mirrors the existing `factory:*` event surface the React side already
// listens for in FactoryPipelineContext.
// ---------------------------------------------------------------------------

struct TauriStepEventHandler {
    app: AppHandle,
    run_id: String,
}

impl TauriStepEventHandler {
    fn new(app: AppHandle, run_id: String) -> Self {
        Self { app, run_id }
    }
}

impl StepEventHandler for TauriStepEventHandler {
    fn handle(&self, event: StepEvent) {
        match event {
            StepEvent::StepStarted { step_id } => {
                let _ = self.app.emit(
                    "factory:step_started",
                    &serde_json::json!({
                        "runId": self.run_id,
                        "stepId": step_id,
                    }),
                );
            }
            StepEvent::AgentOutput { step_id, line } => {
                let _ = self.app.emit(
                    "factory:agent_output",
                    &serde_json::json!({
                        "runId": self.run_id,
                        "stepId": step_id,
                        "line": line,
                    }),
                );
            }
            StepEvent::StepCompleted {
                step_id,
                tokens_used,
            } => {
                let _ = self.app.emit(
                    "factory:step_completed",
                    &serde_json::json!({
                        "runId": self.run_id,
                        "stepId": step_id,
                        "tokenSpend": tokens_used.unwrap_or(0),
                        "artifacts": Vec::<String>::new(),
                    }),
                );
            }
            StepEvent::StepFailed { step_id, error } => {
                let _ = self.app.emit(
                    "factory:step_failed",
                    &serde_json::json!({
                        "runId": self.run_id,
                        "stepId": step_id,
                        "error": error,
                    }),
                );
            }
        }
    }
}

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
    pub phase: String, // "idle" | "process" | "scaffolding" | "complete" | "failed" | "paused"
    pub stages: Vec<StageInfo>,
    pub scaffolding: Option<ScaffoldingInfo>,
    pub total_tokens: u64,
    pub audit_trail: Vec<AuditEntry>,
    /// Adapter recorded at run start (live runs) or read from `state.json`
    /// (disk fallback). Empty for legacy runs that pre-date state.json
    /// being written before terminal phases. The desktop UI uses this to
    /// resume the run without depending on a separate adapter prop.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
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
    /// Count of process stages with at least one output file on disk.
    /// Surfaced in the Pipeline History table so the user can see how far
    /// each run got without selecting it.
    #[serde(default)]
    pub stages_completed: u32,
    #[serde(default)]
    pub stages_total: u32,
    /// Display name of the highest-index completed stage; `None` when no
    /// stage produced an output yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_stage: Option<String>,
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
            tx.send(Ok(()))
                .map_err(|_| "gate channel closed".to_string())
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
        // The TS-side `FactoryGateReachedEvent` expects `stageId` /
        // `stageName` (matching the rest of the gate-confirm API surface).
        // When emitting `stepId` here, the React listener destructured
        // undefined, the dialog title rendered without a stage name, and
        // Confirm POSTed an empty `stage_id` so `gate_handler.approve("")`
        // never matched the pending oneshot. Emit the right field names
        // so the same step id round-trips back through `confirm_factory_stage`.
        self.app
            .emit(
                "factory:gate_reached",
                &serde_json::json!({
                    "stageId": step_id,
                    "stageName": label.unwrap_or(step_id),
                    "gateType": "checkpoint",
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
                    "stageId": step_id,
                    "stageName": step_id,
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
    /// When set, lifecycle events are dual-written to this Stagecraft project.
    stagecraft_project_id: Option<String>,
    /// The Stagecraft-assigned pipeline ID, captured from init_pipeline response.
    stagecraft_pipeline_id: Mutex<Option<String>>,
    /// Tab/execution session that owns this run (spec 110 §2.4). Minted on
    /// tab creation for stagecraft-triggered runs; a fresh UUID for OPC-direct
    /// runs. Surfaces back on `factory.run.ack` and on every `execution.status`
    /// the run emits.
    session_id: String,
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

/// Dedupe set for `factory.run.request` envelopes (spec 110 §2.1: exactly-once
/// intent per pipeline_id). The server's outbox is at-least-once; the first
/// envelope wins and subsequent retries for the same `pipeline_id` become
/// no-ops. Entries live for the life of the OPC process — a retry after a
/// process restart is treated as a fresh request (the prior run persisted its
/// state to disk and stagecraft correlates by pipeline_id regardless).
static FACTORY_RUN_REQUESTS_SEEN: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

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

/// Fire-and-forget: update pipeline status in Stagecraft.
fn sc_update_status(
    sc: &StagecraftClient,
    project_id: &str,
    pipeline_id: &str,
    status: &str,
    current_stage: Option<&str>,
    error: Option<&str>,
    phase: Option<&str>,
) {
    let sc = sc.clone();
    let project_id = project_id.to_string();
    let pipeline_id = pipeline_id.to_string();
    let status = status.to_string();
    let current_stage = current_stage.map(String::from);
    let error = error.map(String::from);
    let phase = phase.map(String::from);
    tokio::spawn(async move {
        if let Err(e) = sc
            .update_pipeline_status(
                &project_id,
                &pipeline_id,
                &status,
                current_stage.as_deref(),
                error.as_deref(),
                phase.as_deref(),
            )
            .await
        {
            log::warn!("Stagecraft status update failed ({status}): {e}");
        }
    });
}

/// Resolve the Stagecraft client + project_id + pipeline_id triple if dual-write is active.
fn resolve_sc_context(
    ctx: &FactoryRunContext,
    sc_client: &Option<StagecraftClient>,
) -> Option<(StagecraftClient, String, String)> {
    let sc = sc_client.as_ref()?;
    let project_id = ctx.stagecraft_project_id.as_ref()?;
    let pipeline_id = ctx.stagecraft_pipeline_id.lock().ok()?.clone()?;
    Some((sc.clone(), project_id.clone(), pipeline_id))
}

/// Fire-and-forget: post step-level events from a dispatch summary to Stagecraft.
/// When a `SyncTracker` is provided, records ack/fail for promotion eligibility (099 Slice 1).
fn sc_ingest_step_events(
    sc: &StagecraftClient,
    project_id: &str,
    pipeline_id: &str,
    summary: &orchestrator::RunSummary,
    phase: &str,
    sync_tracker: Option<&SyncTracker>,
) {
    let events: Vec<super::stagecraft_client::OrchestratorEventReport> = summary
        .steps
        .iter()
        .flat_map(|step| {
            let mut evts = vec![super::stagecraft_client::OrchestratorEventReport {
                event_type: "step_completed".into(),
                step_id: Some(step.step_id.clone()),
                timestamp: now_iso(),
                payload: Some(serde_json::json!({
                    "agent": step.agent,
                    "status": format!("{:?}", step.status),
                    "tokens_used": step.tokens_used,
                    "phase": phase,
                })),
            }];
            if matches!(
                step.status,
                orchestrator::StepStatus::Failure | orchestrator::StepStatus::VerificationFailed
            ) {
                evts[0].event_type = "step_failed".into();
            }
            evts
        })
        .collect();

    if events.is_empty() {
        return;
    }

    let event_count = events.len() as u32;
    let sc = sc.clone();
    let project_id = project_id.to_string();
    let pipeline_id = pipeline_id.to_string();
    let tracker = sync_tracker.cloned();
    tokio::spawn(async move {
        match sc.ingest_events(&project_id, &pipeline_id, &events).await {
            Ok(_) => {
                if let Some(ref t) = tracker {
                    t.record_events_ack(event_count);
                }
            }
            Err(e) => {
                log::warn!("Stagecraft event ingestion failed: {e}");
                if let Some(ref t) = tracker {
                    t.record_events_fail(e.to_string());
                }
            }
        }
    });
}

/// Record step output artifacts to Stagecraft for promotion eligibility (099).
fn sc_record_artifacts(
    sc: &StagecraftClient,
    project_id: &str,
    pipeline_id: &str,
    summary: &orchestrator::RunSummary,
    stage_id: &str,
    sync_tracker: Option<&SyncTracker>,
) {
    let mut artifacts = Vec::new();
    for step in &summary.steps {
        for (filename, hash) in &step.output_hashes {
            let size = step
                .output_artifacts
                .iter()
                .find(|p| {
                    p.file_name()
                        .map(|f| f.to_string_lossy() == filename.as_str())
                        .unwrap_or(false)
                })
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len())
                .unwrap_or(0);
            let ext = std::path::Path::new(filename)
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            artifacts.push(super::stagecraft_client::ArtifactRecord {
                artifact_type: ext,
                content_hash: hash.clone(),
                storage_path: filename.clone(),
                size_bytes: size,
            });
        }
    }
    if artifacts.is_empty() {
        // No artifacts to record — still count as ack so sync succeeds.
        if let Some(t) = sync_tracker {
            t.record_artifacts_ack(0);
        }
        return;
    }

    let sc = sc.clone();
    let pid = project_id.to_string();
    let plid = pipeline_id.to_string();
    let sid = stage_id.to_string();
    let tracker = sync_tracker.cloned();
    let count = artifacts.len() as u32;

    tokio::spawn(async move {
        match sc.record_artifacts(&pid, &plid, &sid, &artifacts).await {
            Ok(_) => {
                if let Some(ref t) = tracker {
                    t.record_artifacts_ack(count);
                }
            }
            Err(e) => {
                log::warn!("Stagecraft artifact recording failed: {e}");
                if let Some(ref t) = tracker {
                    t.record_artifacts_fail(e.to_string());
                }
            }
        }
    });
}

/// Infer the scaffold category from a step ID.
/// Step IDs follow patterns like "s6a-entity-*" (data), "s6b-api-*" (api), "s6c-ui-*" (ui), etc.
fn infer_scaffold_category(step_id: &str) -> String {
    if step_id.contains("entity") || step_id.starts_with("s6a") {
        "data".into()
    } else if step_id.contains("api") || step_id.starts_with("s6b") {
        "api".into()
    } else if step_id.contains("page") || step_id.contains("ui") || step_id.starts_with("s6c") {
        "ui".into()
    } else if step_id.contains("config") || step_id.starts_with("s6d") {
        "configure".into()
    } else if step_id.contains("trim") || step_id.starts_with("s6e") {
        "trim".into()
    } else if step_id.contains("valid") || step_id.starts_with("s6f") {
        "validate".into()
    } else {
        "data".into() // fallback
    }
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
        if let Ok(p) = candidate.canonicalize()
            && p.join("adapters").is_dir()
        {
            return Ok(p);
        }
    }
    Err("factory/ directory not found. Ensure the repository contains a factory/ directory with adapters/".into())
}

/// Persist the run-level summary (`state.json`) used by `list_factory_runs`
/// to surface the run in Pipeline History. Called at run start, on phase
/// transitions, and on terminal success/failure so a partial run does not
/// disappear from history. Errors are intentionally swallowed — disk
/// persistence is best-effort and must never poison the in-memory dispatch.
fn persist_run_state(ctx: &FactoryRunContext, phase: &str) {
    let started_at = ctx
        .audit_trail
        .lock()
        .ok()
        .and_then(|trail| trail.first().map(|e| e.timestamp.clone()))
        .unwrap_or_else(now_iso);
    let total_tokens = ctx
        .pipeline_state
        .lock()
        .map(|ps| ps.total_tokens)
        .unwrap_or(0);
    let completed_at = if phase == "complete" || phase == "failed" {
        Some(now_iso())
    } else {
        None
    };
    // Snapshot stage progress so the history view can render "N/6 — Last
    // stage" without re-walking disk. `stage_status` is the authoritative
    // tracker for completed stages while the run is live; the disk fallback
    // (`list_factory_runs` for runs without state.json) recomputes from
    // artifact presence.
    let (stages_completed, last_completed_stage) = {
        let trackers = ctx.stage_status.lock().ok();
        let mut completed = 0u32;
        let mut last: Option<String> = None;
        if let Some(trackers) = trackers {
            for (id, name) in PROCESS_STAGES {
                if let Some(t) = trackers.get(*id) {
                    if t.status == "completed" {
                        completed += 1;
                        last = Some(name.to_string());
                    }
                }
            }
        }
        (completed, last)
    };
    let summary = PipelineRunSummary {
        run_id: ctx.run_id.to_string(),
        adapter: ctx.adapter_name.clone(),
        project_path: ctx.project_path.to_string_lossy().into(),
        started_at,
        completed_at,
        phase: phase.to_string(),
        total_tokens,
        stages_completed,
        stages_total: PROCESS_STAGES.len() as u32,
        last_completed_stage,
    };
    let dir = ctx
        .project_path
        .join(".factory")
        .join("runs")
        .join(ctx.run_id.to_string());
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join("state.json");
    if let Ok(json) = serde_json::to_string_pretty(&summary) {
        let _ = std::fs::write(&path, json);
    }
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
                artifacts: tracker.map(|t| t.artifacts.clone()).unwrap_or_default(),
                started_at: tracker.and_then(|t| t.started_at.clone()),
                completed_at: tracker.and_then(|t| t.completed_at.clone()),
            }
        })
        .collect();

    // Build scaffolding info from pipeline state if in scaffolding phase.
    let scaffolding = pipeline_state.scaffolding.as_ref().map(|sp| {
        // Entities
        let categories = vec![
            CategoryInfo {
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
            },
            // Operations
            CategoryInfo {
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
            },
            // Pages
            CategoryInfo {
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
            },
        ];
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
        adapter: Some(ctx.adapter_name.clone()),
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
    stagecraft_project_id: Option<String>,
    session_id: Option<String>,
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

    // Get org/project id from StagecraftClient (set by set_active_workspace) — spec 092.
    let org_id: Option<String> = app
        .try_state::<StagecraftState>()
        .and_then(|s| s.current().map(|c| c.org_id()))
        .filter(|s| !s.is_empty());

    let start = engine
        .start_pipeline(&adapter_name, &doc_paths, org_id.clone())
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
        stagecraft_project_id: stagecraft_project_id.clone(),
        stagecraft_pipeline_id: Mutex::new(None),
        // Tab/execution session (spec 110 §2.4). Stagecraft-triggered runs
        // pass the minted id from the envelope handler; OPC-direct runs
        // generate a fresh one so the invariant "every run has a session"
        // holds uniformly.
        session_id: session_id
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
    });

    FACTORY_RUNS
        .lock()
        .map_err(|e| e.to_string())?
        .insert(run_id_str.clone(), ctx.clone());

    // Persist an initial state.json so the run shows up in Pipeline History
    // even before Phase 1 finishes. Updated on phase transitions and on
    // terminal success/failure inside the dispatch task.
    persist_run_state(&ctx, "process");

    // Dual-write: register pipeline with Stagecraft (fire-and-forget).
    if let Some(sc_project_id) = &stagecraft_project_id {
        let sc_opt: Option<StagecraftClient> =
            app.try_state::<StagecraftState>().and_then(|s| s.current());
        if let Some(sc) = sc_opt {
            let pid = sc_project_id.clone();
            let adapter = adapter_name.clone();
            let docs: Vec<_> = business_doc_paths
                .iter()
                .map(|p| super::stagecraft_client::BusinessDocRef {
                    name: PathBuf::from(p)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    storage_ref: p.clone(),
                })
                .collect();
            let ctx_init = ctx.clone();
            tokio::spawn(async move {
                match sc.init_pipeline(&pid, &adapter, &docs).await {
                    Ok(resp) => {
                        log::info!("Stagecraft pipeline registered: {}", resp.pipeline_id);
                        if let Ok(mut guard) = ctx_init.stagecraft_pipeline_id.lock() {
                            *guard = Some(resp.pipeline_id);
                        }
                    }
                    Err(e) => log::warn!("Stagecraft init_pipeline failed (local continues): {e}"),
                }
            });
        }
    }

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
    let project_id_for_spawn = stagecraft_project_id.clone();
    let sc_client: Option<StagecraftClient> = stagecraft_project_id
        .as_ref()
        .and_then(|_| app.try_state::<StagecraftState>())
        .and_then(|s| s.current());

    // Derive governance mode once for the entire pipeline (098 Slice 2).
    let grants_json = crate::governed_claude::grants_json_claude_default();
    let (gov_plan, bypass_reason) = crate::governed_claude::plan_governed_from_binary(&grants_json)
        .map_err(|e| format!("factory start: {e}"))?;
    if let Some(reason) = &bypass_reason {
        eprintln!(
            "[governance] factory start falling back to bypass: {}",
            reason
        );
    }
    let governance_mode_str = match &gov_plan {
        crate::governed_claude::GovernedPlan::Governed { .. } => "governed".to_string(),
        crate::governed_claude::GovernedPlan::Bypass => "bypass".to_string(),
    };
    // Single SyncTracker shared across both phases (099 Slice 2).
    let sync_tracker = SyncTracker::new();

    tokio::spawn(async move {
        // Dual-write: mark pipeline as "running" in Stagecraft.
        if let Some((sc, pid, plid)) = resolve_sc_context(&ctx_for_spawn, &sc_client) {
            sc_update_status(
                &sc,
                &pid,
                &plid,
                "running",
                Some("s0-preflight"),
                None,
                Some("process"),
            );
        }

        // Build executor with agent prompt lookup.
        // Spec 112 §6.4.5 — thread the project's clone token (if any) as
        // GITHUB_TOKEN into every spawned `claude` subprocess so axiomregent
        // authenticates against GitHub on the project's behalf without OPC's
        // own env being mutated.
        let lookup = Arc::new(BridgeLookup(bridge.clone()));
        let extra_env = clone_token_env_for_project(project_id_for_spawn.as_deref());
        let step_event_handler: Arc<dyn StepEventHandler> = Arc::new(TauriStepEventHandler::new(
            app_handle.clone(),
            run_id.to_string(),
        ));
        let executor = Arc::new(
            ClaudeCodeExecutor::new(project_path.clone())
                .with_prompt_lookup(lookup)
                .with_max_turns(25)
                .with_extra_env(extra_env)
                .with_step_event_handler(step_event_handler)
                // Default base is 300s, which scales to Quick=75s — empirically
                // too tight for s0-preflight (reads 30 docx + verification +
                // report write). 900s gives Deep=15m, Investigate=7.5m,
                // Quick=3.75m, which fits the observed Phase 1 stages.
                .with_step_timeout(900),
        );

        let options = DispatchOptions {
            gate_handler: Some(gate_handler.clone() as Arc<dyn GateHandler>),
            project_root: Some(project_path.clone()),
            skip_completed_steps: HashSet::new(),
            cas: None,
            artifact_metadata: None,
            governance_mode: Some(governance_mode_str.clone()),
            sync_tracker: Some(sync_tracker.clone()),
            on_gate_checkpoint: None,
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
                log::error!("factory phase 1 dispatch failed for run {run_id}: {e}");
                ctx_for_spawn.pipeline_state.lock().unwrap().mark_failed();
                persist_run_state(&ctx_for_spawn, "failed");
                if let Some((sc, pid, plid)) = resolve_sc_context(&ctx_for_spawn, &sc_client) {
                    sc_update_status(
                        &sc,
                        &pid,
                        &plid,
                        "failed",
                        None,
                        Some(&e.to_string()),
                        Some("process"),
                    );
                }
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
        persist_run_state(&ctx_for_spawn, "scaffolding");

        app_handle
            .emit(
                "factory:phase1_completed",
                &serde_json::json!({
                    "runId": run_id.to_string(),
                    "steps": summary1.steps.len(),
                }),
            )
            .ok();

        // Dual-write: report Phase 1 token spend per stage to Stagecraft.
        if let Some((sc, pid, plid)) = resolve_sc_context(&ctx_for_spawn, &sc_client) {
            let rid = run_id.to_string();
            for step in &summary1.steps {
                let tokens = step.tokens_used.unwrap_or(0);
                if tokens > 0 {
                    // Split evenly between prompt/completion as a rough estimate;
                    // the orchestrator doesn't track prompt vs completion separately.
                    let half = tokens / 2;
                    if let Err(e) = sc
                        .report_token_spend(
                            &pid,
                            &rid,
                            &step.step_id,
                            half,
                            tokens - half,
                            "claude-sonnet-4-20250514",
                        )
                        .await
                    {
                        log::warn!(
                            "Stagecraft token-spend report failed for {}: {e}",
                            step.step_id
                        );
                    }
                }
            }
            // Ingest step-level events for audit trail.
            sc_ingest_step_events(&sc, &pid, &plid, &summary1, "process", Some(&sync_tracker));
            sc_record_artifacts(&sc, &pid, &plid, &summary1, "process", Some(&sync_tracker));
        }

        // Phase transition: read frozen Build Spec, generate Phase 2 manifest.
        let build_spec_path =
            am.output_artifact_path(run_id, "s5-ui-specification", "build-spec.yaml");
        if !build_spec_path.exists() {
            ctx_for_spawn.pipeline_state.lock().unwrap().mark_failed();
            let err_msg = format!("Build Spec not found at {}", build_spec_path.display());
            log::error!("factory transition failed for run {run_id}: {err_msg}");
            if let Some((sc, pid, plid)) = resolve_sc_context(&ctx_for_spawn, &sc_client) {
                sc_update_status(
                    &sc,
                    &pid,
                    &plid,
                    "failed",
                    None,
                    Some(&err_msg),
                    Some("transition"),
                );
            }
            app_handle
                .emit(
                    "factory:workflow_failed",
                    &serde_json::json!({
                        "runId": run_id.to_string(),
                        "error": err_msg,
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
                org_id.clone(),
            ) {
                Ok(t) => t,
                Err(e) => {
                    log::error!("factory transition_to_scaffolding failed for run {run_id}: {e}");
                    ps.mark_failed();
                    drop(ps);
                    persist_run_state(&ctx_for_spawn, "failed");
                    if let Some((sc, pid, plid)) = resolve_sc_context(&ctx_for_spawn, &sc_client) {
                        sc_update_status(
                            &sc,
                            &pid,
                            &plid,
                            "failed",
                            None,
                            Some(&e.to_string()),
                            Some("transition"),
                        );
                    }
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

        // Dual-write: transition to scaffolding phase in Stagecraft.
        if let Some((sc, pid, plid)) = resolve_sc_context(&ctx_for_spawn, &sc_client) {
            sc_update_status(
                &sc,
                &pid,
                &plid,
                "running",
                Some("s6-scaffolding"),
                None,
                Some("scaffold"),
            );
        }

        // Materialize Phase 2 run directory.
        if let Err(e) = materialize_run_directory(&am, run_id, &transition.manifest) {
            ctx_for_spawn.pipeline_state.lock().unwrap().mark_failed();
            let err_msg = format!("materialize phase 2 failed: {e}");
            log::error!("factory phase 2 materialize failed for run {run_id}: {err_msg}");
            if let Some((sc, pid, plid)) = resolve_sc_context(&ctx_for_spawn, &sc_client) {
                sc_update_status(
                    &sc,
                    &pid,
                    &plid,
                    "failed",
                    None,
                    Some(&err_msg),
                    Some("scaffolding"),
                );
            }
            app_handle
                .emit(
                    "factory:workflow_failed",
                    &serde_json::json!({
                        "runId": run_id.to_string(),
                        "error": err_msg,
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
            cas: None,
            artifact_metadata: None,
            governance_mode: Some(governance_mode_str),
            sync_tracker: Some(sync_tracker.clone()),
            on_gate_checkpoint: None,
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
                log::error!("factory phase 2 dispatch failed for run {run_id}: {e}");
                ctx_for_spawn.pipeline_state.lock().unwrap().mark_failed();
                persist_run_state(&ctx_for_spawn, "failed");
                if let Some((sc, pid, plid)) = resolve_sc_context(&ctx_for_spawn, &sc_client) {
                    sc_update_status(
                        &sc,
                        &pid,
                        &plid,
                        "failed",
                        None,
                        Some(&e.to_string()),
                        Some("scaffolding"),
                    );
                }
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

        // Refresh state.json so list_factory_runs reports the terminal phase
        // and final token total. `persist_run_state` reads phase + tokens
        // straight off the run context (uses the started_at preserved in the
        // audit trail), so the helper deliberately ignores `adapter_for_spawn`.
        let _ = adapter_for_spawn;
        persist_run_state(&ctx_for_spawn, "complete");

        // Dual-write: report Phase 2 (scaffolding) token spend and scaffold progress to Stagecraft.
        if let Some((sc, pid, plid)) = resolve_sc_context(&ctx_for_spawn, &sc_client) {
            let rid = run_id.to_string();
            let mut scaffold_total: u64 = 0;
            for step in &summary2.steps {
                let tokens = step.tokens_used.unwrap_or(0);
                scaffold_total += tokens;
            }
            if scaffold_total > 0 {
                let half = scaffold_total / 2;
                if let Err(e) = sc
                    .report_token_spend(
                        &pid,
                        &rid,
                        "s6-scaffolding",
                        half,
                        scaffold_total - half,
                        "claude-sonnet-4-20250514",
                    )
                    .await
                {
                    log::warn!("Stagecraft token-spend report failed for s6-scaffolding: {e}");
                }
            }

            // Report scaffold feature progress.
            let features: Vec<super::stagecraft_client::ScaffoldFeatureReport> = summary2
                .steps
                .iter()
                .map(|step| {
                    let tokens = step.tokens_used.unwrap_or(0);
                    let half = tokens / 2;
                    super::stagecraft_client::ScaffoldFeatureReport {
                        feature_id: step.step_id.clone(),
                        category: infer_scaffold_category(&step.step_id),
                        status: match step.status {
                            orchestrator::StepStatus::Success => "completed".into(),
                            orchestrator::StepStatus::Failure => "failed".into(),
                            _ => "completed".into(),
                        },
                        retry_count: None,
                        last_error: None,
                        files_created: None,
                        prompt_tokens: Some(half),
                        completion_tokens: Some(tokens - half),
                    }
                })
                .collect();
            if !features.is_empty()
                && let Err(e) = sc.report_scaffold_progress(&pid, &plid, &features).await
            {
                log::warn!("Stagecraft scaffold-progress report failed: {e}");
            }

            // Ingest step-level events for audit trail.
            sc_ingest_step_events(&sc, &pid, &plid, &summary2, "scaffold", Some(&sync_tracker));
            sc_record_artifacts(&sc, &pid, &plid, &summary2, "scaffold", Some(&sync_tracker));

            // Mark pipeline as completed in Stagecraft.
            sc_update_status(&sc, &pid, &plid, "completed", None, None, None);
        }

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
///
/// Resolution order:
/// 1. In-memory `FACTORY_RUNS` (live or just-completed run).
/// 2. Disk fallback — when `project_path` is supplied, reconstruct a status
///    from `<project_path>/.factory/runs/<run_id>/` so Pipeline History can
///    hydrate runs that aren't loaded in memory (after restart, or runs
///    started by a different process).
#[tauri::command]
pub async fn get_factory_pipeline_status(
    run_id: String,
    project_path: Option<String>,
) -> Result<PipelineStatusResponse, String> {
    {
        let runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
        if let Some(ctx) = runs.get(&run_id) {
            return Ok(build_status_response(ctx));
        }
    }

    if let Some(pp) = project_path {
        if let Some(resp) = build_status_response_from_disk(&run_id, &pp) {
            return Ok(resp);
        }
    }

    Err(format!("run not found: {run_id}"))
}

/// Reconstruct a PipelineStatusResponse from on-disk artifacts. Used when
/// the run isn't in `FACTORY_RUNS` — e.g. after an OPC restart, or for runs
/// authored by a different process. Returns `None` when there isn't enough
/// state on disk to identify the run.
fn build_status_response_from_disk(
    run_id: &str,
    project_path: &str,
) -> Option<PipelineStatusResponse> {
    let run_dir = std::path::Path::new(project_path)
        .join(".factory")
        .join("runs")
        .join(run_id);
    if !run_dir.exists() {
        return None;
    }

    // Read state.json if available — gives us run-level phase + total tokens.
    let state_summary: Option<PipelineRunSummary> = std::fs::read_to_string(run_dir.join("state.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok());

    // Read summary.json (orchestrator's RunSummary) for per-step token counts.
    // Optional — falls back to 0 when missing or malformed.
    let step_tokens: HashMap<String, u64> = std::fs::read_to_string(run_dir.join("summary.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("steps").cloned())
        .and_then(|steps| steps.as_array().cloned())
        .map(|arr| {
            arr.into_iter()
                .filter_map(|s| {
                    let id = s.get("step_id")?.as_str()?.to_string();
                    let tokens = s.get("tokens_used").and_then(|t| t.as_u64()).unwrap_or(0);
                    Some((id, tokens))
                })
                .collect()
        })
        .unwrap_or_default();

    let stages: Vec<StageInfo> = PROCESS_STAGES
        .iter()
        .map(|(id, name)| {
            let stage_dir = run_dir.join(id);
            let mut artifacts: Vec<String> = Vec::new();
            let mut completed = false;
            if stage_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&stage_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_file() {
                            if let Some(name) = entry.file_name().to_str() {
                                artifacts.push(name.to_string());
                            }
                        }
                    }
                }
                completed = !artifacts.is_empty();
            }
            StageInfo {
                id: id.to_string(),
                name: name.to_string(),
                status: if completed { "completed".into() } else { "pending".into() },
                token_spend: step_tokens.get(*id).copied().unwrap_or(0),
                artifacts,
                started_at: None,
                completed_at: None,
            }
        })
        .collect();

    let all_done = stages.iter().all(|s| s.status == "completed");
    let phase = state_summary
        .as_ref()
        .map(|s| {
            // A `state.json` saying `process` for a run that's not in
            // FACTORY_RUNS is stale — the dispatch task already exited.
            // Promote those to `paused` so the UI offers Resume rather than
            // claiming the run is still ticking.
            if s.phase == "process" || s.phase == "scaffolding" {
                "paused".to_string()
            } else {
                s.phase.clone()
            }
        })
        .unwrap_or_else(|| {
            // Infer phase from stage completeness when state.json is absent
            // (legacy runs predating the early-write helper).
            if all_done {
                "complete".to_string()
            } else {
                "paused".to_string()
            }
        });
    let total_tokens = state_summary
        .as_ref()
        .map(|s| s.total_tokens)
        .filter(|t| *t > 0)
        .unwrap_or_else(|| stages.iter().map(|s| s.token_spend).sum());

    let adapter = state_summary.as_ref().and_then(|s| {
        if s.adapter.is_empty() {
            None
        } else {
            Some(s.adapter.clone())
        }
    });

    Some(PipelineStatusResponse {
        run_id: run_id.to_string(),
        phase,
        stages,
        scaffolding: None,
        total_tokens,
        audit_trail: vec![],
        adapter,
    })
}

/// Confirm a gate stage. Resolves the pending oneshot in TauriGateHandler.
#[tauri::command]
pub async fn confirm_factory_stage(
    app: AppHandle,
    run_id: String,
    stage_id: String,
) -> Result<(), String> {
    let sc_project_id;
    {
        let runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
        let ctx = runs
            .get(&run_id)
            .ok_or_else(|| format!("run not found: {run_id}"))?;

        ctx.gate_handler.approve(&stage_id)?;

        // Record in audit trail.
        ctx.audit_trail.lock().unwrap().push(AuditEntry {
            timestamp: now_iso(),
            action: "gate_confirmed".into(),
            stage_id: Some(stage_id.clone()),
            details: None,
            feedback: None,
        });

        sc_project_id = ctx.stagecraft_project_id.clone();
    }

    // Dual-write: confirm stage in Stagecraft (fire-and-forget).
    if let Some(pid) = sc_project_id {
        let sc_opt: Option<StagecraftClient> =
            app.try_state::<StagecraftState>().and_then(|s| s.current());
        if let Some(sc) = sc_opt {
            let sid = stage_id;
            tokio::spawn(async move {
                if let Err(e) = sc.confirm_stage(&pid, &sid, None).await {
                    log::warn!("Stagecraft confirm_stage failed for {sid}: {e}");
                }
            });
        }
    }

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
    let sc_project_id;
    {
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

        sc_project_id = ctx.stagecraft_project_id.clone();
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

    // Dual-write: reject stage in Stagecraft (fire-and-forget).
    if let Some(pid) = sc_project_id {
        let sc_opt: Option<StagecraftClient> =
            app.try_state::<StagecraftState>().and_then(|s| s.current());
        if let Some(sc) = sc_opt {
            let sid = stage_id;
            let fb = feedback;
            tokio::spawn(async move {
                if let Err(e) = sc.reject_stage(&pid, &sid, &fb).await {
                    log::warn!("Stagecraft reject_stage failed for {sid}: {e}");
                }
            });
        }
    }

    Ok(())
}

/// Cancel a running Factory pipeline.
#[tauri::command]
pub async fn cancel_factory_pipeline(
    app: AppHandle,
    run_id: String,
    reason: String,
) -> Result<(), String> {
    let sc_project_id = {
        let runs = FACTORY_RUNS.lock().map_err(|e| e.to_string())?;
        let ctx = runs
            .get(&run_id)
            .ok_or_else(|| format!("run not found: {run_id}"))?;

        // Mark local pipeline as failed/cancelled
        ctx.pipeline_state.lock().unwrap().mark_failed();

        ctx.audit_trail.lock().unwrap().push(AuditEntry {
            timestamp: now_iso(),
            action: "pipeline_cancelled".into(),
            stage_id: None,
            details: Some(reason.clone()),
            feedback: None,
        });

        persist_run_state(ctx, "failed");

        ctx.stagecraft_project_id.clone()
    };

    app.emit(
        "factory:workflow_cancelled",
        &serde_json::json!({
            "runId": run_id,
            "reason": reason,
        }),
    )
    .map_err(|e| format!("emit factory:workflow_cancelled failed: {e}"))?;

    // Dual-write: cancel in Stagecraft
    if let Some(pid) = sc_project_id {
        let sc_opt: Option<StagecraftClient> =
            app.try_state::<StagecraftState>().and_then(|s| s.current());
        if let Some(sc) = sc_opt {
            tokio::spawn(async move {
                if let Err(e) = sc.cancel_pipeline(&pid, &reason).await {
                    log::warn!("Stagecraft cancel_pipeline failed: {e}");
                }
            });
        }
    }

    Ok(())
}

/// Scan a single run directory for completed-stage progress. A stage is
/// considered complete when its sub-directory holds at least one regular
/// file. Matches the heuristic used by `build_status_response_from_disk`
/// and the orchestrator's resume detector.
fn scan_disk_progress(run_dir: &std::path::Path) -> (u32, Option<String>) {
    let mut completed = 0u32;
    let mut last: Option<String> = None;
    for (id, name) in PROCESS_STAGES {
        let sd = run_dir.join(id);
        if !sd.is_dir() {
            continue;
        }
        let has_output = std::fs::read_dir(&sd)
            .map(|it| it.flatten().any(|e| e.path().is_file()))
            .unwrap_or(false);
        if has_output {
            completed += 1;
            last = Some(name.to_string());
        }
    }
    (completed, last)
}

/// List all Factory pipeline runs under `<project_path>/.factory/runs/`.
///
/// Prefers `state.json` (written by the desktop dispatch loop), and falls
/// back to a `manifest.yaml`-only synthesis so legacy runs and runs that
/// died before reaching the first state-write still appear in history and
/// can be selected for resume.
#[tauri::command]
pub async fn list_factory_runs(project_path: String) -> Result<Vec<PipelineRunSummary>, String> {
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
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let run_id = match dir.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };

        // Walk stage subdirs once — used both to refine state.json (which
        // may be stale between phase transitions) and to synthesise the
        // legacy fallback path below.
        let (stages_completed, last_completed_stage) = scan_disk_progress(&dir);

        // Primary: state.json from the desktop dispatcher.
        let state_path = dir.join("state.json");
        if state_path.exists() {
            if let Ok(text) = std::fs::read_to_string(&state_path) {
                if let Ok(mut summary) = serde_json::from_str::<PipelineRunSummary>(&text) {
                    // Disk truth wins over the snapshot — `state.json` is
                    // refreshed on phase boundaries, but stage outputs land
                    // continuously. Fix-ups also paper over old state.json
                    // files that pre-date the progress fields.
                    summary.stages_total = PROCESS_STAGES.len() as u32;
                    if stages_completed > summary.stages_completed {
                        summary.stages_completed = stages_completed;
                    }
                    if summary.last_completed_stage.is_none() {
                        summary.last_completed_stage = last_completed_stage.clone();
                    }
                    summaries.push(summary);
                    continue;
                }
            }
        }

        // Fallback: manifest.yaml-only run (no state.json yet, or written by
        // a non-desktop tool). Build a minimal summary from disk so the run
        // is still selectable and resumable from Pipeline History.
        let manifest_path = dir.join("manifest.yaml");
        if !manifest_path.exists() {
            continue;
        }

        let started_at = std::fs::metadata(&manifest_path)
            .and_then(|m| m.created().or_else(|_| m.modified()))
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| {
                chrono::DateTime::<chrono::Utc>::from_timestamp(d.as_secs() as i64, 0)
                    .unwrap_or_else(chrono::Utc::now)
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string()
            })
            .unwrap_or_else(now_iso);

        let stages_total = PROCESS_STAGES.len() as u32;
        let all_stages_done = stages_completed == stages_total;

        summaries.push(PipelineRunSummary {
            run_id,
            adapter: String::new(), // unknown for legacy runs
            project_path: project_path.clone(),
            started_at,
            completed_at: None,
            // Disk-only runs are not running anywhere — surface as `paused`
            // so the UI offers Resume and stops claiming the run is active.
            phase: if all_stages_done {
                "complete".into()
            } else {
                "paused".into()
            },
            total_tokens: 0,
            stages_completed,
            stages_total,
            last_completed_stage,
        });
    }

    summaries.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(summaries)
}

/// List artifact files for a given run/step combination.
///
/// Resolution order for the artifact directory:
/// 1. Live `FACTORY_RUNS` entry's project path.
/// 2. Caller-supplied `project_path` (used by Pipeline History when the run
///    is hydrated from disk and isn't in `FACTORY_RUNS`).
/// 3. Legacy `~/.oap/artifacts/<run_id>/<step_id>` cache.
#[tauri::command]
pub async fn get_factory_artifacts(
    run_id: String,
    step_id: String,
    project_path: Option<String>,
) -> Result<Vec<ArtifactInfo>, String> {
    // Resolve project path from live context if available.
    let live_path = FACTORY_RUNS
        .lock()
        .map_err(|e| e.to_string())?
        .get(&run_id)
        .map(|ctx| ctx.project_path.clone());

    let base = if let Some(p) = live_path {
        p.join(".factory").join("runs").join(&run_id).join(&step_id)
    } else if let Some(p) = project_path.as_ref().filter(|s| !s.is_empty()) {
        PathBuf::from(p)
            .join(".factory")
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

    let entries = std::fs::read_dir(&base).map_err(|e| format!("read artifact dir failed: {e}"))?;

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
    stagecraft_project_id: Option<String>,
) -> Result<(), String> {
    let factory_root = resolve_factory_root()?;
    let project_path = PathBuf::from(&project_path)
        .canonicalize()
        .map_err(|e| format!("resolve project path failed: {e}"))?;
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| format!("invalid run_id: {e}"))?;

    let config = FactoryEngineConfig {
        factory_root: factory_root.clone(),
        project_path: project_path.clone(),
        concurrency_limit: 4,
        max_total_tokens: None,
    };
    let engine = FactoryEngine::new(config).map_err(|e| e.to_string())?;

    // Get org/project id from StagecraftClient for resumed pipelines (spec 092).
    let org_id: Option<String> = app
        .try_state::<StagecraftState>()
        .and_then(|s| s.current().map(|c| c.org_id()))
        .filter(|s| !s.is_empty());

    let start = engine
        .start_pipeline(&adapter_name, &[], org_id)
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
    // Spec 112 §6.4.5 — thread the project's clone token through resumed runs
    // too, so re-entry after a pause does not silently downgrade to anon.
    let extra_env = clone_token_env_for_project(stagecraft_project_id.as_deref());
    let step_event_handler: Arc<dyn StepEventHandler> = Arc::new(TauriStepEventHandler::new(
        app.clone(),
        run_uuid.to_string(),
    ));
    let executor = Arc::new(
        ClaudeCodeExecutor::new(project_path.clone())
            .with_prompt_lookup(lookup)
            .with_max_turns(25)
            .with_extra_env(extra_env)
            .with_step_event_handler(step_event_handler)
            // Match start_factory_pipeline's bumped budget (default 300 → 900).
            .with_step_timeout(900),
    );

    // Derive governance mode for resumed pipeline (098 Slice 2).
    let grants_json = crate::governed_claude::grants_json_claude_default();
    let (gov_plan, bypass_reason) = crate::governed_claude::plan_governed_from_binary(&grants_json)
        .map_err(|e| format!("factory resume: {e}"))?;
    if let Some(reason) = &bypass_reason {
        eprintln!(
            "[governance] factory resume falling back to bypass: {}",
            reason
        );
    }
    let governance_mode_str = match &gov_plan {
        crate::governed_claude::GovernedPlan::Governed { .. } => "governed".to_string(),
        crate::governed_claude::GovernedPlan::Bypass => "bypass".to_string(),
    };
    // Fresh SyncTracker for the resumed run (099 Slice 2).
    let sync_tracker = SyncTracker::new();

    let options = DispatchOptions {
        gate_handler: Some(gate_handler as Arc<dyn GateHandler>),
        project_root: Some(project_path),
        skip_completed_steps: skip_steps,
        cas: None,
        artifact_metadata: None,
        governance_mode: Some(governance_mode_str),
        sync_tracker: Some(sync_tracker),
        on_gate_checkpoint: None,
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
                log::error!("factory dispatch failed for run {run_id}: {e}");
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

// ---------------------------------------------------------------------------
// Knowledge bundle materialisation (spec 110 §2.3)
// ---------------------------------------------------------------------------

/// Reasons a knowledge-bundle materialisation can fail. Each variant maps to
/// a distinct decline_reason on `factory.run.ack` so stagecraft can record
/// why the run never started.
#[derive(Debug, thiserror::Error)]
pub enum KnowledgeMaterializationError {
    #[error("knowledge_hash_mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("download failed for {object_id}: {source}")]
    Download {
        object_id: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("cache I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid content hash {0}: must be lowercase hex sha-256")]
    InvalidHash(String),
}

/// Resolve the OPC knowledge cache directory. Honours `OPC_CACHE_DIR` for
/// tests and sandboxes; falls back to the platform cache dir + `/opc`.
fn opc_cache_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("OPC_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("opc")
}

/// Content-addressable cache root for materialised knowledge blobs.
pub fn knowledge_cache_dir() -> PathBuf {
    opc_cache_dir().join("knowledge")
}

fn is_lowercase_hex_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn hash_file(path: &std::path::Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Materialise a single knowledge bundle to a local path. Enforces the
/// content-hash trust boundary (§2.3): a mismatch fails the run.
///
/// Cache layout: `<cache_dir>/knowledge/<sha256>/<filename>`. Nesting under
/// the hash lets different filenames coexist for the same content while still
/// deduping by bytes.
pub async fn materialize_knowledge_bundle(
    bundle: &WireKnowledgeBundle,
) -> Result<PathBuf, KnowledgeMaterializationError> {
    materialize_knowledge_bundle_in(bundle, &knowledge_cache_dir()).await
}

/// Test-friendly variant: pins the cache root explicitly so unit tests can
/// isolate themselves without mutating process-global env state.
pub async fn materialize_knowledge_bundle_in(
    bundle: &WireKnowledgeBundle,
    cache_root: &std::path::Path,
) -> Result<PathBuf, KnowledgeMaterializationError> {
    if !is_lowercase_hex_sha256(&bundle.content_hash) {
        return Err(KnowledgeMaterializationError::InvalidHash(
            bundle.content_hash.clone(),
        ));
    }

    let hash_dir = cache_root.join(&bundle.content_hash);
    std::fs::create_dir_all(&hash_dir)?;
    let cache_path = hash_dir.join(&bundle.filename);

    // Cache hit — verify bytes haven't been tampered with since last write
    // before we hand the path to the engine.
    if cache_path.exists() {
        let observed = hash_file(&cache_path)?;
        if observed == bundle.content_hash {
            return Ok(cache_path);
        }
        // Corrupted cache entry — remove and fall through to re-download.
        let _ = std::fs::remove_file(&cache_path);
    }

    // Cache miss — download, stream-hash, write atomically.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| KnowledgeMaterializationError::Download {
            object_id: bundle.object_id.clone(),
            source: e,
        })?;

    let bytes = client
        .get(&bundle.download_url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| KnowledgeMaterializationError::Download {
            object_id: bundle.object_id.clone(),
            source: e,
        })?
        .bytes()
        .await
        .map_err(|e| KnowledgeMaterializationError::Download {
            object_id: bundle.object_id.clone(),
            source: e,
        })?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let observed = hex::encode(hasher.finalize());
    if observed != bundle.content_hash {
        return Err(KnowledgeMaterializationError::HashMismatch {
            expected: bundle.content_hash.clone(),
            actual: observed,
        });
    }

    // Atomic write: land to a sibling temp file in the same directory, then
    // rename. Avoids a half-written cache entry on crash.
    let tmp = hash_dir.join(format!(".{}.partial", bundle.filename));
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &cache_path)?;

    Ok(cache_path)
}

// ---------------------------------------------------------------------------
// factory.run.request dispatch handler (spec 110 §2.1 + §8 Phase 4)
// ---------------------------------------------------------------------------

/// Minimal shape we extract from a `factory.run.request` envelope before
/// dispatching it to the engine. Mirrors [`ServerEnvelopeWire`] but narrowed
/// to the fields this handler actually reads.
struct InboundFactoryRun {
    pipeline_id: String,
    project_id: String,
    adapter: String,
    knowledge: Vec<WireKnowledgeBundle>,
}

fn extract_factory_run(envelope: &ServerEnvelopeWire) -> Option<InboundFactoryRun> {
    Some(InboundFactoryRun {
        pipeline_id: envelope.pipeline_id.clone()?,
        project_id: envelope.project_id.clone()?,
        adapter: envelope.adapter.clone()?,
        knowledge: envelope.knowledge.clone().unwrap_or_default(),
    })
}

/// Register a `factory.run.request` handler on the given sync consumer
/// dispatch table. Wires the desktop end of spec 110 §8 Phase 4.
///
/// The handler:
///   - Dedupes by `pipeline_id` (§2.1 exactly-once intent).
///   - Materialises attached knowledge bundles into the content-addressable
///     cache and verifies each sha-256 (§2.3 trust boundary).
///   - Mints a `session_id` per accepted request (§2.4 tab session).
///   - Starts the local factory pipeline via `start_factory_pipeline` and
///     passes the minted session id through so progress frames can carry it.
///   - Emits `factory.run.ack` on the outbound channel — `accepted: true` on
///     the happy path, `accepted: false` with a structured decline_reason on
///     materialisation or engine start failures.
pub fn register_factory_run_handler(app: AppHandle, opc_instance_id: String) {
    let sync_state_present = app.try_state::<SyncClientState>().is_some();
    if !sync_state_present {
        log::warn!("register_factory_run_handler: SyncClientState not managed — skipping");
        return;
    }
    let dispatch = app.state::<SyncClientState>().dispatch_table();

    let app_for_handler = app.clone();
    let handler = FnHandler(move |envelope: &ServerEnvelopeWire| {
        let Some(run) = extract_factory_run(envelope) else {
            log::warn!(
                "factory.run.request missing required fields (pipeline_id/project_id/adapter) — ignoring"
            );
            return;
        };

        // Exactly-once dedupe by pipeline_id. A returning `true` from `insert`
        // means this is the first time we've seen this pipeline_id.
        let first_time = match FACTORY_RUN_REQUESTS_SEEN.lock() {
            Ok(mut g) => g.insert(run.pipeline_id.clone()),
            Err(_) => false,
        };
        if !first_time {
            log::info!(
                "factory.run.request duplicate for pipeline_id={} — ignored",
                run.pipeline_id
            );
            return;
        }

        let session_id = Uuid::new_v4().to_string();
        let app = app_for_handler.clone();
        let opc_id = opc_instance_id.clone();
        tauri::async_runtime::spawn(async move {
            handle_factory_run_request(app, opc_id, session_id, run).await;
        });
    });

    dispatch.register("factory.run.request", Arc::new(handler));
    log::info!("sync_client: factory.run.request dispatch handler registered");
}

async fn handle_factory_run_request(
    app: AppHandle,
    opc_instance_id: String,
    session_id: String,
    run: InboundFactoryRun,
) {
    // Step 1: materialise knowledge bundles. Hash mismatch is a trust-boundary
    // failure — decline the run and let stagecraft mark it failed.
    let mut doc_paths = Vec::with_capacity(run.knowledge.len());
    for bundle in &run.knowledge {
        match materialize_knowledge_bundle(bundle).await {
            Ok(p) => doc_paths.push(p.to_string_lossy().into_owned()),
            Err(e) => {
                let decline = match &e {
                    KnowledgeMaterializationError::HashMismatch { .. } => {
                        "knowledge_hash_mismatch".to_string()
                    }
                    _ => format!("knowledge_materialization_failed: {e}"),
                };
                log::warn!(
                    "factory.run.request pipeline_id={} bundle={} failed: {e}",
                    run.pipeline_id,
                    bundle.object_id
                );
                send_factory_run_ack(
                    &app,
                    &run.pipeline_id,
                    &session_id,
                    &opc_instance_id,
                    false,
                    Some(decline),
                );
                return;
            }
        }
    }

    // Step 2: resolve a project_path. For the Phase 4 landing we use the
    // OPC workspace-scoped scratch path `$OPC_CACHE_DIR/projects/<project_id>`
    // when the frontend has no active project path. This keeps the envelope
    // flow self-contained while a future phase wires project paths through
    // the org catalog (spec 111).
    let project_path = opc_cache_dir()
        .join("projects")
        .join(&run.project_id)
        .to_string_lossy()
        .into_owned();

    // Step 3: start the pipeline with the minted session id.
    let result = start_factory_pipeline(
        app.clone(),
        project_path,
        run.adapter.clone(),
        doc_paths,
        Some(run.project_id.clone()),
        Some(session_id.clone()),
    )
    .await;

    match result {
        Ok(_) => {
            send_factory_run_ack(
                &app,
                &run.pipeline_id,
                &session_id,
                &opc_instance_id,
                true,
                None,
            );
        }
        Err(e) => {
            log::warn!(
                "factory.run.request pipeline_id={} engine start failed: {e}",
                run.pipeline_id
            );
            send_factory_run_ack(
                &app,
                &run.pipeline_id,
                &session_id,
                &opc_instance_id,
                false,
                Some(format!("engine_start_failed: {e}")),
            );
        }
    }
}

/// Fire a `factory.run.ack` on the outbound channel. No-op when the duplex
/// stream is not currently connected — the handler still executed so the
/// dedupe marker persists and a reconnect won't re-trigger the same run.
fn send_factory_run_ack(
    app: &AppHandle,
    pipeline_id: &str,
    session_id: &str,
    opc_instance_id: &str,
    accepted: bool,
    decline_reason: Option<String>,
) {
    let Some(sync) = app.try_state::<SyncClientState>() else {
        log::warn!("send_factory_run_ack: SyncClientState not managed");
        return;
    };
    let pipeline_id = pipeline_id.to_string();
    let session_id = session_id.to_string();
    let opc_instance_id = opc_instance_id.to_string();
    let sync_handle = sync.handle();
    tauri::async_runtime::spawn(async move {
        let sent = sync_handle
            .send_factory_run_ack(
                &pipeline_id,
                &session_id,
                &opc_instance_id,
                accepted,
                decline_reason.clone(),
            )
            .await;
        if !sent {
            log::warn!(
                "factory.run.ack not delivered (duplex disconnected) pipeline_id={}",
                pipeline_id
            );
        }
    });
}

// ---------------------------------------------------------------------------
// Tests (spec 110 Phase 4)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, extract::State, http::StatusCode, response::Response, routing::get};
    use std::net::SocketAddr;
    use tokio::sync::oneshot;

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    #[test]
    fn is_lowercase_hex_sha256_accepts_valid_hex() {
        let ok = "a".repeat(64);
        assert!(is_lowercase_hex_sha256(&ok));
        assert!(is_lowercase_hex_sha256(&sha256_hex(b"hello")));
    }

    #[test]
    fn is_lowercase_hex_sha256_rejects_bad_inputs() {
        assert!(!is_lowercase_hex_sha256(""));
        assert!(!is_lowercase_hex_sha256(&"a".repeat(63)));
        assert!(!is_lowercase_hex_sha256(&"A".repeat(64)), "uppercase rejected");
        assert!(!is_lowercase_hex_sha256(&"g".repeat(64)), "non-hex rejected");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn materialize_uses_cached_file_when_hash_matches() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_root = tmp.path().join("knowledge");

        let payload = b"canonical bytes";
        let hash = sha256_hex(payload);
        let cache_path = cache_root.join(&hash).join("doc.md");
        std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
        std::fs::write(&cache_path, payload).unwrap();

        let bundle = WireKnowledgeBundle {
            object_id: "k1".into(),
            filename: "doc.md".into(),
            content_hash: hash.clone(),
            // URL deliberately unreachable — a cache hit must not dial out.
            download_url: "http://127.0.0.1:1/does-not-exist".into(),
        };
        let got = materialize_knowledge_bundle_in(&bundle, &cache_root)
            .await
            .unwrap();
        assert_eq!(got, cache_path);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn materialize_rejects_invalid_hash() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = WireKnowledgeBundle {
            object_id: "k1".into(),
            filename: "doc.md".into(),
            content_hash: "not-a-sha256".into(),
            download_url: "http://127.0.0.1:1".into(),
        };
        let err = materialize_knowledge_bundle_in(&bundle, tmp.path())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            KnowledgeMaterializationError::InvalidHash(_)
        ));
    }

    // Spin up a minimal axum server that serves a fixed byte body at /blob.
    // Returns the bound address plus a shutdown signal; drop the signal to
    // stop the server.
    async fn spawn_blob_server(body: Vec<u8>) -> (SocketAddr, oneshot::Sender<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = oneshot::channel::<()>();

        async fn serve(State(body): State<Arc<Vec<u8>>>) -> Response {
            Response::builder()
                .status(StatusCode::OK)
                .body(Body::from(body.as_ref().clone()))
                .unwrap()
        }

        let app = Router::new()
            .route("/blob", get(serve))
            .with_state(Arc::new(body));

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = rx.await;
                })
                .await
                .unwrap();
        });
        (addr, tx)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn materialize_downloads_and_caches_on_miss() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_root = tmp.path().join("knowledge");

        let payload = b"streamed body".to_vec();
        let hash = sha256_hex(&payload);
        let (addr, _stop) = spawn_blob_server(payload.clone()).await;

        let bundle = WireKnowledgeBundle {
            object_id: "k1".into(),
            filename: "doc.md".into(),
            content_hash: hash.clone(),
            download_url: format!("http://{addr}/blob"),
        };

        let got = materialize_knowledge_bundle_in(&bundle, &cache_root)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&got).unwrap(), payload);

        // Second call must be a cache hit — the partial/temp file should be
        // gone, only the canonical filename remains under the hash dir.
        let second = materialize_knowledge_bundle_in(&bundle, &cache_root)
            .await
            .unwrap();
        assert_eq!(second, got);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn materialize_fails_on_hash_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_root = tmp.path().join("knowledge");

        let payload = b"actual bytes".to_vec();
        let (addr, _stop) = spawn_blob_server(payload).await;
        // Claim a different (valid-shape) hash so the downloaded bytes fail
        // verification. This is the trust-boundary case from spec 110 §2.3.
        let bogus_hash = "0".repeat(64);
        let bundle = WireKnowledgeBundle {
            object_id: "k1".into(),
            filename: "doc.md".into(),
            content_hash: bogus_hash,
            download_url: format!("http://{addr}/blob"),
        };
        let err = materialize_knowledge_bundle_in(&bundle, &cache_root)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            KnowledgeMaterializationError::HashMismatch { .. }
        ));
    }

    #[test]
    fn extract_factory_run_requires_pipeline_project_adapter() {
        // Missing pipeline_id — handler must reject (a malformed envelope
        // slipped past the schema guard).
        let env = ServerEnvelopeWire {
            kind: "factory.run.request".into(),
            meta: crate::commands::sync_client::ServerMeta {
                v: 1,
                event_id: "e".into(),
                sent_at: "2026-04-21T00:00:00Z".into(),
                correlation_id: None,
                causation_id: None,
                org_cursor: "c".into(),
                org_id: "org-1".into(),
            },
            policy_bundle_id: None,
            summary: None,
            user_id: None,
            change: None,
            details: None,
            project_id: Some("p".into()),
            environment_id: None,
            status: None,
            detail: None,
            pipeline_id: None,
            event_type: None,
            stage_id: None,
            actor: None,
            client_event_id: None,
            reason: None,
            session_id: None,
            server_started_at: None,
            cursor_gap: None,
            adapter: Some("rest".into()),
            actor_user_id: None,
            knowledge: None,
            business_docs: None,
            requested_at: None,
            deadline_at: None,
            agent_id: None,
            name: None,
            version: None,
            content_hash: None,
            frontmatter: None,
            body_markdown: None,
            updated_at: None,
            entries: None,
            generated_at: None,
            slug: None,
            description: None,
            org_id: None,
            factory_adapter_id: None,
            detection_level: None,
            repo: None,
            opc_deep_link: None,
            tombstone: None,
        };
        assert!(extract_factory_run(&env).is_none());
    }

    #[test]
    fn extract_factory_run_populates_on_complete_envelope() {
        let env = ServerEnvelopeWire {
            kind: "factory.run.request".into(),
            meta: crate::commands::sync_client::ServerMeta {
                v: 1,
                event_id: "e".into(),
                sent_at: "2026-04-21T00:00:00Z".into(),
                correlation_id: None,
                causation_id: None,
                org_cursor: "c".into(),
                org_id: "org-1".into(),
            },
            policy_bundle_id: None,
            summary: None,
            user_id: None,
            change: None,
            details: None,
            project_id: Some("proj-1".into()),
            environment_id: None,
            status: None,
            detail: None,
            pipeline_id: Some("pl-1".into()),
            event_type: None,
            stage_id: None,
            actor: None,
            client_event_id: None,
            reason: None,
            session_id: None,
            server_started_at: None,
            cursor_gap: None,
            adapter: Some("rest".into()),
            actor_user_id: None,
            knowledge: Some(vec![WireKnowledgeBundle {
                object_id: "k1".into(),
                filename: "d.md".into(),
                content_hash: "a".repeat(64),
                download_url: "http://x/k1".into(),
            }]),
            business_docs: None,
            requested_at: None,
            deadline_at: None,
            agent_id: None,
            name: None,
            version: None,
            content_hash: None,
            frontmatter: None,
            body_markdown: None,
            updated_at: None,
            entries: None,
            generated_at: None,
            slug: None,
            description: None,
            org_id: None,
            factory_adapter_id: None,
            detection_level: None,
            repo: None,
            opc_deep_link: None,
            tombstone: None,
        };
        let run = extract_factory_run(&env).unwrap();
        assert_eq!(run.pipeline_id, "pl-1");
        assert_eq!(run.project_id, "proj-1");
        assert_eq!(run.adapter, "rest");
        assert_eq!(run.knowledge.len(), 1);
    }
}
