//! Spec 172 — Live agent-session introspection (consumer surface).
//!
//! Aggregates two underlying surfaces into a single live-panel snapshot:
//!
//!   1. The process registry (running Claude sessions + per-session
//!      ActivityTracker — see [`crate::process::activity`]).
//!   2. The orchestrator's `list_active_workflows` consumer query
//!      (`SqliteWorkflowStore::list_active_workflows`).
//!
//! Both are read through their owners' typed APIs; this module does not
//! reach past those APIs to parse persisted state (spec 103, FR-008).

use crate::process::{ActivitySnapshot, ProcessRegistryState, ProcessType};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::State;

const SETTINGS_FILE: &str = "spec-172-thresholds.json";

/// One row in the Live Sessions panel (spec 172 §2.1 per-session row).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSessionRow {
    pub run_id: i64,
    pub session_id: String,
    pub project_path: String,
    pub pid: u32,
    pub started_at: String,
    pub model: String,
    pub task: String,
    /// Spec 172 §2.1 — "local-only" when no Rauthy scope is bound; the
    /// platform-issued JWT subject + tenant scope otherwise. Current OPC
    /// substrate does not bind scopes to local Claude sessions, so this is
    /// always "local-only" today. The field is permanent so future
    /// scope binding can fill it without a schema migration.
    pub scope: String,
    pub activity: ActivitySnapshot,
    pub status: SessionStatus,
}

/// Status indicator computed from event rate vs threshold (spec 172 §2.3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SessionStatus {
    Idle,
    Active,
    Warning,
    Critical,
}

/// One row in the Live Sessions panel (spec 172 §2.1 per-workflow row).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveWorkflowRow {
    pub workflow_id: String,
    pub workflow_name: String,
    pub status: String,
    pub started_at: String,
    pub project_id: Option<String>,
    pub current_step_name: Option<String>,
    pub current_step_index: Option<u32>,
    pub current_step_started_at: Option<String>,
    pub step_count: Option<u32>,
}

/// Configurable rate thresholds (spec 172 §2.3).
///
/// Defaults are intentionally conservative; per spec 036 a tenant-scoped
/// session has lower thresholds than substrate-scoped. The current OPC
/// substrate keeps a single profile until scope binding is wired
/// (see [`LiveSessionRow::scope`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSessionThresholds {
    pub warning_tool_calls_per_minute: u64,
    pub critical_tool_calls_per_minute: u64,
    pub warning_tokens_per_minute: u64,
    pub critical_tokens_per_minute: u64,
    /// Cumulative tool calls above which a session is forced into Critical
    /// regardless of recent rate.
    pub critical_cumulative_tool_calls: u64,
}

impl Default for LiveSessionThresholds {
    fn default() -> Self {
        Self {
            warning_tool_calls_per_minute: 30,
            critical_tool_calls_per_minute: 60,
            warning_tokens_per_minute: 20_000,
            critical_tokens_per_minute: 50_000,
            critical_cumulative_tool_calls: 5_000,
        }
    }
}

impl LiveSessionThresholds {
    /// Classify an activity snapshot against this threshold set.
    pub fn classify(&self, activity: &ActivitySnapshot) -> SessionStatus {
        if activity.cumulative_tool_calls >= self.critical_cumulative_tool_calls
            || activity.tool_calls_per_minute >= self.critical_tool_calls_per_minute
            || activity.tokens_per_minute >= self.critical_tokens_per_minute
        {
            return SessionStatus::Critical;
        }
        if activity.tool_calls_per_minute >= self.warning_tool_calls_per_minute
            || activity.tokens_per_minute >= self.warning_tokens_per_minute
        {
            return SessionStatus::Warning;
        }
        if activity.last_event_at.is_some() {
            SessionStatus::Active
        } else {
            SessionStatus::Idle
        }
    }
}

/// Aggregated snapshot returned to the panel (spec 172 FR-002).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSessionsSnapshot {
    pub sessions: Vec<LiveSessionRow>,
    pub workflows: Vec<LiveWorkflowRow>,
    pub thresholds: LiveSessionThresholds,
    /// ISO-8601 timestamp the snapshot was assembled.
    pub generated_at: String,
}

fn thresholds_settings_path() -> std::path::PathBuf {
    let data_dir = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    data_dir.join("opc").join(SETTINGS_FILE)
}

fn load_thresholds() -> LiveSessionThresholds {
    let path = thresholds_settings_path();
    load_thresholds_from(&path)
}

fn load_thresholds_from(path: &Path) -> LiveSessionThresholds {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => LiveSessionThresholds::default(),
    }
}

fn workflow_db_path() -> std::path::PathBuf {
    std::env::var("OPC_WORKFLOW_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let data_dir = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            data_dir.join("opc").join("workflows.db")
        })
}

/// Spec 172 FR-001 / FR-002 — return a single snapshot of live sessions +
/// workflows + thresholds for the panel to render.
#[tauri::command]
pub async fn list_live_sessions(
    registry: State<'_, ProcessRegistryState>,
) -> Result<LiveSessionsSnapshot, String> {
    let thresholds = load_thresholds();

    let registry = registry.0.clone();
    let session_rows = tokio::task::spawn_blocking({
        let thresholds = thresholds.clone();
        move || -> Result<Vec<LiveSessionRow>, String> {
            let processes = registry.get_running_claude_sessions()?;
            let mut rows = Vec::with_capacity(processes.len());
            for info in processes {
                let session_id = match &info.process_type {
                    ProcessType::ClaudeSession { session_id } => session_id.clone(),
                    _ => continue,
                };

                let activity = registry
                    .session_activity_snapshot(&session_id)?
                    .unwrap_or_else(|| ActivitySnapshot {
                        tool_calls_per_minute: 0,
                        tokens_per_minute: 0,
                        cumulative_tool_calls: 0,
                        cumulative_tokens: 0,
                        recent_tool_calls: vec![],
                        last_event_at: None,
                    });
                let status = thresholds.classify(&activity);

                rows.push(LiveSessionRow {
                    run_id: info.run_id,
                    session_id,
                    project_path: info.project_path,
                    pid: info.pid,
                    started_at: info.started_at.to_rfc3339(),
                    model: info.model,
                    task: info.task,
                    scope: "local-only".to_string(),
                    activity,
                    status,
                });
            }
            Ok(rows)
        }
    })
    .await
    .map_err(|e| format!("join error collecting sessions: {e}"))??;

    let workflows = collect_active_workflows().await?;

    Ok(LiveSessionsSnapshot {
        sessions: session_rows,
        workflows,
        thresholds,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

async fn collect_active_workflows() -> Result<Vec<LiveWorkflowRow>, String> {
    let store_path = workflow_db_path();
    // If the workflow store hasn't been initialised yet (fresh install / no
    // factory runs), an open() failure is not panel-fatal — show no workflows.
    let store = match orchestrator::sqlite_state::SqliteWorkflowStore::open(&store_path) {
        Ok(s) => s,
        Err(_) => return Ok(vec![]),
    };
    let summaries = store
        .list_active_workflows(None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(summaries
        .into_iter()
        .map(|s| LiveWorkflowRow {
            workflow_id: s.workflow_id,
            workflow_name: s.workflow_name,
            status: s.status,
            started_at: s.started_at,
            project_id: s.project_id,
            current_step_name: s.current_step_name,
            current_step_index: s.current_step_index,
            current_step_started_at: s.current_step_started_at,
            step_count: s.step_count,
        })
        .collect())
}

/// Spec 172 §2.3 / FR-006 — return the current threshold config.
#[tauri::command]
pub async fn get_live_session_thresholds() -> Result<LiveSessionThresholds, String> {
    Ok(load_thresholds())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn snapshot(rate: u64, tokens: u64, cumulative: u64) -> ActivitySnapshot {
        ActivitySnapshot {
            tool_calls_per_minute: rate,
            tokens_per_minute: tokens,
            cumulative_tool_calls: cumulative,
            cumulative_tokens: 0,
            recent_tool_calls: vec![],
            last_event_at: Some(Utc::now()),
        }
    }

    #[test]
    fn classify_idle_when_no_events() {
        let t = LiveSessionThresholds::default();
        let s = ActivitySnapshot {
            tool_calls_per_minute: 0,
            tokens_per_minute: 0,
            cumulative_tool_calls: 0,
            cumulative_tokens: 0,
            recent_tool_calls: vec![],
            last_event_at: None,
        };
        assert_eq!(t.classify(&s), SessionStatus::Idle);
    }

    #[test]
    fn classify_active_below_warning() {
        let t = LiveSessionThresholds::default();
        assert_eq!(t.classify(&snapshot(5, 100, 5)), SessionStatus::Active);
    }

    #[test]
    fn classify_warning_above_warning_rate() {
        let t = LiveSessionThresholds::default();
        assert_eq!(
            t.classify(&snapshot(t.warning_tool_calls_per_minute, 0, 50)),
            SessionStatus::Warning
        );
        assert_eq!(
            t.classify(&snapshot(0, t.warning_tokens_per_minute, 50)),
            SessionStatus::Warning
        );
    }

    #[test]
    fn classify_critical_above_critical_rate() {
        let t = LiveSessionThresholds::default();
        assert_eq!(
            t.classify(&snapshot(t.critical_tool_calls_per_minute, 0, 100)),
            SessionStatus::Critical
        );
        assert_eq!(
            t.classify(&snapshot(0, t.critical_tokens_per_minute, 100)),
            SessionStatus::Critical
        );
    }

    #[test]
    fn classify_critical_above_cumulative() {
        let t = LiveSessionThresholds::default();
        // Even at zero recent rate, cumulative tool calls push to Critical.
        let s = ActivitySnapshot {
            tool_calls_per_minute: 0,
            tokens_per_minute: 0,
            cumulative_tool_calls: t.critical_cumulative_tool_calls,
            cumulative_tokens: 0,
            recent_tool_calls: vec![],
            last_event_at: Some(Utc::now()),
        };
        assert_eq!(t.classify(&s), SessionStatus::Critical);
    }

    #[test]
    fn missing_thresholds_file_falls_back_to_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("does-not-exist.json");
        let loaded = load_thresholds_from(&path);
        assert_eq!(loaded.warning_tool_calls_per_minute, 30);
        assert_eq!(loaded.critical_tool_calls_per_minute, 60);
    }

    #[test]
    fn malformed_thresholds_file_falls_back_to_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("malformed.json");
        std::fs::write(&path, "{ not valid json").unwrap();
        let loaded = load_thresholds_from(&path);
        assert_eq!(
            loaded.warning_tool_calls_per_minute,
            LiveSessionThresholds::default().warning_tool_calls_per_minute
        );
    }

    #[test]
    fn well_formed_thresholds_file_overrides_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("ok.json");
        std::fs::write(
            &path,
            serde_json::to_string(&LiveSessionThresholds {
                warning_tool_calls_per_minute: 7,
                critical_tool_calls_per_minute: 14,
                warning_tokens_per_minute: 1_000,
                critical_tokens_per_minute: 2_000,
                critical_cumulative_tool_calls: 100,
            })
            .unwrap(),
        )
        .unwrap();
        let loaded = load_thresholds_from(&path);
        assert_eq!(loaded.warning_tool_calls_per_minute, 7);
        assert_eq!(loaded.critical_tokens_per_minute, 2_000);
    }
}
