use crate::commands::agents::AgentDb;
use orchestrator::{
    dispatch_manifest, materialize_run_directory, AgentRegistry, ArtifactManager, DispatchRequest,
    DispatchResult, EffortLevel, GovernedExecutor, RunSummary, StepStatus, WorkflowManifest,
};
use rusqlite::params;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, LazyLock, Mutex};
use tauri::State;
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
}

struct RealGovernedExecutor {
    agents: HashMap<String, AgentExecutionProfile>,
    working_directory: String,
}

#[async_trait::async_trait]
impl GovernedExecutor for RealGovernedExecutor {
    async fn dispatch_step(&self, request: DispatchRequest) -> Result<DispatchResult, String> {
        let Some(profile) = self.agents.get(&request.agent_id) else {
            return Err(format!("agent execution profile not found: {}", request.agent_id));
        };

        let effort_hint = match request.effort {
            EffortLevel::Quick => "quick",
            EffortLevel::Investigate => "investigate",
            EffortLevel::Deep => "deep",
        };
        let full_prompt = format!(
            "{}\n\n{}\n\nExecution requirements:\n- Effort hint: {}\n- You MUST write all declared output artifacts to the exact absolute paths listed in the prompt.",
            profile.system_prompt.as_str(),
            request.system_prompt,
            effort_hint
        );

        if profile.model.contains(':') {
            self.dispatch_via_provider_registry(&request, profile, &full_prompt)
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
            "workingDirectory": &self.working_directory,
            "model": &profile.model,
            "permissionMode": "default"
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

        Ok(DispatchResult { tokens_used })
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
        if let Ok(axiom) = crate::governed_claude::bundled_axiomregent_binary_path() {
            if let Ok(mcp_config) =
                crate::governed_claude::axiomregent_mcp_config_json(&axiom, &grants_json)
            {
                args.push("--mcp-config".to_string());
                args.push(mcp_config);
                args.push("--permission-mode".to_string());
                args.push("default".to_string());
            } else {
                args.push("--dangerously-skip-permissions".to_string());
            }
        } else {
            args.push("--dangerously-skip-permissions".to_string());
        }

        let mut cmd = Command::new("claude");
        for arg in args {
            cmd.arg(arg);
        }
        cmd.current_dir(&self.working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

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
                        return Err(format!("step {} failed in governed execution", request.step_id));
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
                return Err(format!("governed claude exited with non-zero status: {status}"));
            }
            return Err(format!(
                "governed claude exited with non-zero status: {status}; stderr: {detail}"
            ));
        }

        Ok(DispatchResult { tokens_used })
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

    let agent_profiles = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT name, system_prompt, model, enable_file_read, enable_file_write, enable_network FROM agents")
            .map_err(|e| format!("prepare registry query failed: {e}"))?;
        let rows = stmt
            .query_map(params![], |row| {
                let name: String = row.get(0)?;
                let system_prompt: String = row.get(1)?;
                let model: String = row.get(2)?;
                let enable_file_read: bool = row.get::<_, bool>(3).unwrap_or(true);
                let enable_file_write: bool = row.get::<_, bool>(4).unwrap_or(true);
                let enable_network: bool = row.get::<_, bool>(5).unwrap_or(true);
                Ok((
                    name,
                    AgentExecutionProfile {
                        model,
                        system_prompt,
                        enable_file_read,
                        enable_file_write,
                        enable_network,
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

    let summary = dispatch_manifest(
        &artifact_base,
        run_id,
        &manifest,
        Arc::new(SnapshotRegistry {
            agents: agent_profiles.clone(),
        }),
        Arc::new(RealGovernedExecutor {
            agents: agent_profiles,
            working_directory: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        }),
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
