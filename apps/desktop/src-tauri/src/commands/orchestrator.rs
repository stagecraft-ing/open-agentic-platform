use crate::commands::agents::AgentDb;
use orchestrator::{
    AgentRegistry, ArtifactManager, DispatchOptions, DispatchRequest, DispatchResult, EffortLevel,
    GovernedExecutor, RunSummary, StepStatus, WorkflowManifest, dispatch_manifest,
    materialize_run_directory,
};
use rusqlite::params;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{Emitter, State};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
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
    agents: HashMap<String, AgentExecutionProfile>,
}

#[async_trait::async_trait]
impl AgentRegistry for SnapshotRegistry {
    async fn has_agent(&self, agent_id: &str) -> bool {
        self.agents.contains_key(agent_id)
    }
}

#[derive(Clone, Debug)]
struct AgentExecutionProfile {
    model: String,
    system_prompt: String,
    enable_file_read: bool,
    enable_file_write: bool,
    enable_network: bool,
    allowed_tools: Option<Vec<String>>,
}

struct RealGovernedExecutor {
    agents: HashMap<String, AgentExecutionProfile>,
    working_directory: String,
}

#[async_trait::async_trait]
impl GovernedExecutor for RealGovernedExecutor {
    async fn dispatch_step(&self, request: DispatchRequest) -> Result<DispatchResult, String> {
        let Some(profile) = self.agents.get(&request.agent_id) else {
            return Err(format!(
                "agent execution profile not found: {}",
                request.agent_id
            ));
        };

        let user_prompt = build_user_prompt_with_requirements(&request);
        let full_prompt = format!(
            "{}\n\n{}",
            profile.system_prompt.as_str(),
            user_prompt.as_str()
        );

        if profile.model.contains(':') {
            self.dispatch_via_provider_registry(&request, profile, &user_prompt)
                .await
        } else {
            self.dispatch_via_governed_claude(&request, profile, &full_prompt)
                .await
        }
    }
}

impl RealGovernedExecutor {
    async fn dispatch_via_provider_registry(
        &self,
        request: &DispatchRequest,
        profile: &AgentExecutionProfile,
        prompt: &str,
    ) -> Result<DispatchResult, String> {
        let sidecar_js = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join("packages/provider-registry/dist/node-sidecar.js");
        if !sidecar_js.exists() {
            return Err(format!(
                "bridge sidecar not found at {}. Build with: pnpm exec tsc -p packages/provider-registry/tsconfig.json",
                sidecar_js.display()
            ));
        }

        let mut cmd = Command::new("node");
        cmd.arg(sidecar_js.as_os_str())
            .current_dir(&self.working_directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(ref ws_id) = request.workspace_id {
            cmd.env("OPC_WORKSPACE_ID", ws_id);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("spawn provider-registry sidecar failed: {e}"))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "provider-registry sidecar stdin unavailable".to_string())?;
        let query = serde_json::json!({
            "type": "query",
            "prompt": prompt,
            "agentName": &request.agent_id,
            "systemPrompt": &profile.system_prompt,
            "workingDirectory": &self.working_directory,
            "model": &profile.model,
            "permissionMode": "default",
            "allowedTools": &profile.allowed_tools
        });
        let line = serde_json::to_string(&query).map_err(|e| e.to_string())?;
        stdin
            .write_all(format!("{line}\n").as_bytes())
            .await
            .map_err(|e| format!("write sidecar query failed: {e}"))?;
        stdin
            .flush()
            .await
            .map_err(|e| format!("flush sidecar query failed: {e}"))?;
        drop(stdin);

        let mut stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| "provider-registry sidecar stdout unavailable".to_string())?,
        )
        .lines();
        let mut stderr = BufReader::new(
            child
                .stderr
                .take()
                .ok_or_else(|| "provider-registry sidecar stderr unavailable".to_string())?,
        )
        .lines();

        let mut tokens_used: Option<u64> = None;
        while let Some(line) = stdout
            .next_line()
            .await
            .map_err(|e| format!("read sidecar stdout failed: {e}"))?
        {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                if v.get("done").and_then(|x| x.as_bool()) == Some(true) {
                    break;
                }
                if v.get("type").and_then(|x| x.as_str()) == Some("error") {
                    let msg = v
                        .get("error")
                        .and_then(|x| x.as_str())
                        .unwrap_or("provider-registry sidecar error");
                    return Err(format!("step {} failed: {}", request.step_id, msg));
                }
                if v.get("type").and_then(|x| x.as_str()) == Some("result") {
                    let input = v
                        .get("total_input_tokens")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0);
                    let output = v
                        .get("total_output_tokens")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0);
                    tokens_used = Some(input + output);
                }
            }
        }

        let mut stderr_buf = String::new();
        while let Some(line) = stderr
            .next_line()
            .await
            .map_err(|e| format!("read sidecar stderr failed: {e}"))?
        {
            stderr_buf.push_str(&line);
            stderr_buf.push('\n');
        }

        let status = child
            .wait()
            .await
            .map_err(|e| format!("wait sidecar process failed: {e}"))?;
        if !status.success() {
            let detail = stderr_buf.trim();
            if detail.is_empty() {
                return Err(format!(
                    "provider-registry sidecar exited with non-zero status: {status}"
                ));
            }
            return Err(format!(
                "provider-registry sidecar exited with non-zero status: {status}; stderr: {detail}"
            ));
        }

        Ok(DispatchResult {
            tokens_used,
            output_hashes: Default::default(),
            session_id: None,
            cost_usd: None,
            duration_ms: None,
            num_turns: None,
            governance_mode: None,
        })
    }

    async fn dispatch_via_governed_claude(
        &self,
        request: &DispatchRequest,
        profile: &AgentExecutionProfile,
        prompt: &str,
    ) -> Result<DispatchResult, String> {
        let grants_json = serde_json::json!({
            "enable_file_read": profile.enable_file_read,
            "enable_file_write": profile.enable_file_write,
            "enable_network": profile.enable_network,
            "max_tier": 3
        })
        .to_string();

        let mut args = vec![
            "-p".to_string(),
            prompt.to_string(),
            "--model".to_string(),
            profile.model.clone(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];
        if let Some(allowed_tools) = &profile.allowed_tools
            && !allowed_tools.is_empty()
        {
            args.push("--allowedTools".to_string());
            args.extend(allowed_tools.iter().cloned());
        }
        let (plan, bypass_reason) =
            crate::governed_claude::plan_governed_from_binary(&grants_json)?;
        let governance_mode_str = match &plan {
            crate::governed_claude::GovernedPlan::Governed { .. } => "governed",
            crate::governed_claude::GovernedPlan::Bypass => "bypass",
        };
        crate::governed_claude::append_claude_governance_args(&mut args, &plan);
        if let Some(reason) = &bypass_reason {
            eprintln!(
                "[governance] orchestrator step {} falling back to bypass: {}",
                request.step_id, reason
            );
        }

        let mut cmd = Command::new("claude");
        for arg in args {
            cmd.arg(arg);
        }
        cmd.current_dir(&self.working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(ref ws_id) = request.workspace_id {
            cmd.env("OPC_WORKSPACE_ID", ws_id);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("spawn governed claude process failed: {e}"))?;
        let mut stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| "governed claude stdout unavailable".to_string())?,
        )
        .lines();
        let mut stderr = BufReader::new(
            child
                .stderr
                .take()
                .ok_or_else(|| "governed claude stderr unavailable".to_string())?,
        )
        .lines();
        let mut tokens_used: Option<u64> = None;

        while let Some(line) = stdout
            .next_line()
            .await
            .map_err(|e| format!("read governed claude stdout failed: {e}"))?
        {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                if v.get("type").and_then(|x| x.as_str()) == Some("result") {
                    let input = v
                        .get("total_input_tokens")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0);
                    let output = v
                        .get("total_output_tokens")
                        .and_then(|x| x.as_u64())
                        .unwrap_or(0);
                    tokens_used = Some(input + output);
                    if v.get("subtype").and_then(|x| x.as_str()) == Some("error") {
                        return Err(format!(
                            "step {} failed in governed execution",
                            request.step_id
                        ));
                    }
                } else if v.get("type").and_then(|x| x.as_str()) == Some("error") {
                    let msg = v
                        .get("error")
                        .and_then(|x| x.as_str())
                        .unwrap_or("governed claude error");
                    return Err(format!("step {} failed: {}", request.step_id, msg));
                }
            }
        }

        let mut stderr_buf = String::new();
        while let Some(line) = stderr
            .next_line()
            .await
            .map_err(|e| format!("read governed claude stderr failed: {e}"))?
        {
            stderr_buf.push_str(&line);
            stderr_buf.push('\n');
        }

        let status = child
            .wait()
            .await
            .map_err(|e| format!("wait governed claude process failed: {e}"))?;
        if !status.success() {
            let detail = stderr_buf.trim();
            if detail.is_empty() {
                return Err(format!(
                    "governed claude exited with non-zero status: {status}"
                ));
            }
            return Err(format!(
                "governed claude exited with non-zero status: {status}; stderr: {detail}"
            ));
        }

        Ok(DispatchResult {
            tokens_used,
            output_hashes: Default::default(),
            session_id: None,
            cost_usd: None,
            duration_ms: None,
            num_turns: None,
            governance_mode: Some(governance_mode_str.to_string()),
        })
    }
}

fn build_user_prompt_with_requirements(request: &DispatchRequest) -> String {
    let effort_hint = match request.effort {
        EffortLevel::Quick => "quick",
        EffortLevel::Investigate => "investigate",
        EffortLevel::Deep => "deep",
    };
    format!(
        "{}\n\nExecution requirements:\n- Effort hint: {}\n- You MUST write all declared output artifacts to the exact absolute paths listed in the prompt.",
        request.system_prompt, effort_hint
    )
}

#[tauri::command]
pub async fn orchestrate_manifest(
    app: tauri::AppHandle,
    manifest_path: String,
    project_path: String,
    db: State<'_, AgentDb>,
) -> Result<RunSummary, String> {
    let manifest_text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read manifest failed: {e}"))?;
    let manifest: WorkflowManifest =
        serde_yaml::from_str(&manifest_text).map_err(|e| format!("parse manifest failed: {e}"))?;

    let agent_profiles = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT name, system_prompt, model, enable_file_read, enable_file_write, enable_network, tools FROM agents")
            .map_err(|e| format!("prepare registry query failed: {e}"))?;
        let rows = stmt
            .query_map(params![], |row| {
                let name: String = row.get(0)?;
                let system_prompt: String = row.get(1)?;
                let model: String = row.get(2)?;
                let enable_file_read: bool = row.get::<_, bool>(3).unwrap_or(true);
                let enable_file_write: bool = row.get::<_, bool>(4).unwrap_or(true);
                let enable_network: bool = row.get::<_, bool>(5).unwrap_or(true);
                let tools_raw: Option<String> = row.get(6)?;
                let allowed_tools = parse_allowed_tools(tools_raw.as_deref());
                Ok((
                    name,
                    AgentExecutionProfile {
                        model,
                        system_prompt,
                        enable_file_read,
                        enable_file_write,
                        enable_network,
                        allowed_tools,
                    },
                ))
            })
            .map_err(|e| format!("query registry failed: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect registry failed: {e}"))?;
        rows.into_iter().collect::<HashMap<_, _>>()
    };

    let run_id = Uuid::new_v4();
    let artifact_base = ArtifactManager::from_env();
    materialize_run_directory(&artifact_base, run_id, &manifest)
        .map_err(|e| format!("materialize run dir failed: {e}"))?;

    // Determine governance mode at launch time (098 Slice 2).
    let grants_json = crate::governed_claude::grants_json_claude_default();
    let (plan, bypass_reason) = crate::governed_claude::plan_governed_from_binary(&grants_json)
        .map_err(|e| format!("orchestrate_manifest: {e}"))?;
    if let Some(reason) = &bypass_reason {
        eprintln!(
            "[governance] orchestrate_manifest falling back to bypass: {}",
            reason
        );
    }
    let governance_mode = match &plan {
        crate::governed_claude::GovernedPlan::Governed { .. } => "governed",
        crate::governed_claude::GovernedPlan::Bypass => "bypass",
    };
    let _ = app.emit(
        "governance-mode",
        serde_json::json!({ "mode": governance_mode, "context": "orchestrate_manifest", "governance_bypass_reason": bypass_reason }),
    );

    let summary = dispatch_manifest(
        &artifact_base,
        run_id,
        &manifest,
        Arc::new(SnapshotRegistry {
            agents: agent_profiles.clone(),
        }),
        Arc::new(RealGovernedExecutor {
            agents: agent_profiles,
            working_directory: project_path,
        }),
        &DispatchOptions {
            governance_mode: Some(governance_mode.to_string()),
            ..Default::default()
        },
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

/// List workflow summaries for a given workspace (099 Slice 5).
#[tauri::command]
pub async fn list_workspace_workflows(
    workspace_id: String,
    limit: Option<u32>,
) -> Result<Vec<orchestrator::WorkflowStateSummary>, String> {
    // Use the default SQLite store location
    let store_path = std::env::var("OPC_WORKFLOW_DB").unwrap_or_else(|_| {
        let data_dir = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        data_dir
            .join("opc")
            .join("workflows.db")
            .to_string_lossy()
            .to_string()
    });
    let store =
        orchestrator::sqlite_state::SqliteWorkflowStore::open(std::path::Path::new(&store_path))
            .map_err(|e| format!("open workflow store: {e}"))?;
    store
        .list_workflows_by_workspace(&workspace_id, limit)
        .await
        .map_err(|e| e.to_string())
}

fn parse_allowed_tools(raw: Option<&str>) -> Option<Vec<String>> {
    let raw = raw?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('[')
        && let Ok(parsed) = serde_json::from_str::<Vec<String>>(trimmed)
    {
        let normalized: Vec<String> = parsed
            .into_iter()
            .map(|tool| tool.trim().to_string())
            .filter(|tool| !tool.is_empty())
            .collect();
        return if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        };
    }

    let normalized: Vec<String> = trimmed
        .split(',')
        .map(|tool| tool.trim().to_string())
        .filter(|tool| !tool.is_empty())
        .collect();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
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
