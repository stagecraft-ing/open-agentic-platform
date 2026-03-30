use crate::commands::agents::AgentDb;
use orchestrator::{
    dispatch_manifest, materialize_run_directory, AgentRegistry, ArtifactManager, DispatchRequest,
    DispatchResult, EffortLevel, GovernedExecutor, RunSummary, StepStatus, WorkflowManifest,
};
use rusqlite::params;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::State;
use uuid::Uuid;

struct InMemoryRunState {
    summaries: HashMap<Uuid, RunSummary>,
}

static RUN_STATE: LazyLock<Mutex<InMemoryRunState>> = LazyLock::new(|| {
    Mutex::new(InMemoryRunState {
        summaries: HashMap::new(),
    })
});

struct SnapshotRegistry {
    agents: HashSet<String>,
}

#[async_trait::async_trait]
impl AgentRegistry for SnapshotRegistry {
    async fn has_agent(&self, agent_id: &str) -> bool {
        self.agents.contains(agent_id)
    }
}

struct FileBackedGovernedExecutor;

#[async_trait::async_trait]
impl GovernedExecutor for FileBackedGovernedExecutor {
    async fn dispatch_step(&self, request: DispatchRequest) -> Result<DispatchResult, String> {
        // Phase 4 wiring: preserve artifact protocol and output-path contract.
        // Governed runtime transport can replace this executor without changing orchestrator semantics.
        let max_tokens_hint = match request.effort {
            EffortLevel::Quick => "quick",
            EffortLevel::Investigate => "investigate",
            EffortLevel::Deep => "deep",
        };

        for output in &request.output_artifacts {
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("create output directory failed: {e}"))?;
            }
            std::fs::write(
                output,
                format!(
                    "# Orchestrator Artifact\n\nstep_id: {}\nagent_id: {}\neffort: {}\n\nThis file was produced by the Phase 4 governed-dispatch scaffold.\n",
                    request.step_id, request.agent_id, max_tokens_hint
                ),
            )
            .map_err(|e| format!("write output artifact failed: {e}"))?;
        }
        Ok(DispatchResult { tokens_used: None })
    }
}

#[tauri::command]
pub async fn orchestrate_manifest(
    manifest_path: String,
    db: State<'_, AgentDb>,
) -> Result<RunSummary, String> {
    let manifest_text =
        std::fs::read_to_string(&manifest_path).map_err(|e| format!("read manifest failed: {e}"))?;
    let manifest: WorkflowManifest =
        serde_yaml::from_str(&manifest_text).map_err(|e| format!("parse manifest failed: {e}"))?;

    let agent_names = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT name FROM agents")
            .map_err(|e| format!("prepare registry query failed: {e}"))?;
        let rows = stmt
            .query_map(params![], |row| row.get::<_, String>(0))
            .map_err(|e| format!("query registry failed: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect registry failed: {e}"))?;
        rows.into_iter().collect::<HashSet<_>>()
    };

    let run_id = Uuid::new_v4();
    let artifact_base = ArtifactManager::from_env();
    materialize_run_directory(&artifact_base, run_id, &manifest)
        .map_err(|e| format!("materialize run dir failed: {e}"))?;

    let summary = dispatch_manifest(
        &artifact_base,
        run_id,
        &manifest,
        Arc::new(SnapshotRegistry { agents: agent_names }),
        Arc::new(FileBackedGovernedExecutor),
    )
    .await
    .map_err(|e| e.to_string())?;

    RUN_STATE
        .lock()
        .map_err(|e| e.to_string())?
        .summaries
        .insert(run_id, summary.clone());
    Ok(summary)
}

#[tauri::command]
pub async fn get_run_status(run_id: String) -> Result<RunSummary, String> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| format!("invalid run_id: {e}"))?;
    if let Some(summary) = RUN_STATE
        .lock()
        .map_err(|e| e.to_string())?
        .summaries
        .get(&run_uuid)
        .cloned()
    {
        return Ok(summary);
    }

    let artifact_base = ArtifactManager::from_env();
    let summary_path = artifact_base.run_dir(run_uuid).join("summary.json");
    let text =
        std::fs::read_to_string(summary_path).map_err(|e| format!("read summary failed: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("parse summary failed: {e}"))
}

#[tauri::command]
pub async fn cancel_run(run_id: String) -> Result<(), String> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| format!("invalid run_id: {e}"))?;
    let mut state = RUN_STATE.lock().map_err(|e| e.to_string())?;
    if let Some(summary) = state.summaries.get_mut(&run_uuid) {
        for step in &mut summary.steps {
            if matches!(step.status, StepStatus::Pending | StepStatus::Running) {
                step.status = StepStatus::Cancelled;
            }
        }
        let artifact_base = ArtifactManager::from_env();
        summary
            .write_to_disk(&artifact_base)
            .map_err(|e| format!("write cancelled summary failed: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn cleanup_artifacts(run_id: String) -> Result<(), String> {
    let run_uuid = Uuid::parse_str(&run_id).map_err(|e| format!("invalid run_id: {e}"))?;
    RUN_STATE
        .lock()
        .map_err(|e| e.to_string())?
        .summaries
        .remove(&run_uuid);
    ArtifactManager::from_env()
        .cleanup_run(run_uuid)
        .map_err(|e| format!("cleanup artifacts failed: {e}"))
}
