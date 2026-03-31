// SPDX-License-Identifier: AGPL-3.0-or-later
// Feature 052 Phase 4: SQLite backend for workflow state and events.
//
// This module provides an alternative state backend backed by SQLite in WAL
// mode. It mirrors the JSON `WorkflowState` schema into relational tables and
// prepares an `events` table for append-only logging (used by Phase 5 SSE).

use crate::artifact::ArtifactManager;
use crate::state::{GateInfo, StepExecutionStatus, StepState, WorkflowState, WorkflowStatus};
use crate::OrchestratorError;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value as JsonValue;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Computes the canonical SQLite database path for a given run directory.
pub fn sqlite_db_path_for_run_dir(run_dir: &Path) -> PathBuf {
    run_dir.join("state.sqlite")
}

/// Computes the canonical SQLite database path for a workflow using the artifact manager.
pub fn sqlite_db_path_for_run(artifact_base: &ArtifactManager, workflow_id: Uuid) -> PathBuf {
    let run_dir = artifact_base.run_dir(workflow_id);
    sqlite_db_path_for_run_dir(&run_dir)
}

/// SQLite-backed store for workflow state and events (052 FR-001, FR-002, FR-007).
pub struct SqliteWorkflowStore {
    conn: Connection,
}

impl SqliteWorkflowStore {
    /// Opens (or creates) a SQLite database at `path`, enabling WAL mode and
    /// initializing the schema if needed.
    pub fn open(path: &Path) -> Result<Self, OrchestratorError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("create sqlite state dir {}: {e}", parent.display()),
            })?;
        }

        let mut conn =
            Connection::open(path).map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("open sqlite state db {}: {e}", path.display()),
            })?;

        // Enable WAL mode for better concurrent read performance (052 NF-001).
        conn.pragma_update(None, "journal_mode", &"WAL")
            .map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("enable WAL journal mode on {}: {e}", path.display()),
            })?;

        // Initialize schema if it does not exist yet. This mirrors the spec's
        // `workflows`, `steps`, and `events` tables.
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS workflows (
              workflow_id   TEXT PRIMARY KEY,
              workflow_name TEXT NOT NULL,
              status        TEXT NOT NULL,
              started_at    TEXT NOT NULL,
              completed_at  TEXT,
              metadata      TEXT
            );

            CREATE TABLE IF NOT EXISTS steps (
              step_id       TEXT PRIMARY KEY,
              workflow_id   TEXT NOT NULL REFERENCES workflows(workflow_id),
              step_index    INTEGER NOT NULL,
              name          TEXT NOT NULL,
              status        TEXT NOT NULL,
              started_at    TEXT,
              completed_at  TEXT,
              duration_ms   INTEGER,
              output        TEXT,
              gate_type     TEXT,
              gate_config   TEXT
            );

            CREATE TABLE IF NOT EXISTS events (
              event_id      INTEGER PRIMARY KEY AUTOINCREMENT,
              workflow_id   TEXT NOT NULL REFERENCES workflows(workflow_id),
              timestamp     TEXT NOT NULL,
              event_type    TEXT NOT NULL,
              payload       TEXT NOT NULL
            );
            "#,
        )
        .map_err(|e| OrchestratorError::StatePersistence {
            reason: format!("initialize sqlite schema in {}: {e}", path.display()),
        })?;

        Ok(SqliteWorkflowStore { conn })
    }

    /// Persists the given workflow state into the SQLite database inside a
    /// transaction. Existing step rows for the workflow are replaced.
    pub fn write_workflow_state(&mut self, state: &WorkflowState) -> Result<(), OrchestratorError> {
        let tx = self
            .conn
            .transaction()
            .map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("begin sqlite transaction: {e}"),
            })?;

        let wf_id_str = state.workflow_id.to_string();
        let status_str = workflow_status_to_str(&state.status);
        let metadata_json = if state.metadata.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&state.metadata).map_err(|e| {
                    OrchestratorError::StatePersistence {
                        reason: format!("serialize workflow metadata to json: {e}"),
                    }
                })?,
            )
        };

        tx.execute(
            r#"
            INSERT INTO workflows (
              workflow_id, workflow_name, status, started_at, completed_at, metadata
            )
            VALUES (?1, ?2, ?3, ?4, NULL, ?5)
            ON CONFLICT(workflow_id) DO UPDATE SET
              workflow_name = excluded.workflow_name,
              status        = excluded.status,
              started_at    = excluded.started_at,
              completed_at  = excluded.completed_at,
              metadata      = excluded.metadata
            "#,
            params![
                wf_id_str,
                state.workflow_name,
                status_str,
                state.started_at,
                metadata_json
            ],
        )
        .map_err(|e| OrchestratorError::StatePersistence {
            reason: format!("upsert workflow row in sqlite: {e}"),
        })?;

        // Replace all step rows for this workflow.
        tx.execute(
            "DELETE FROM steps WHERE workflow_id = ?1",
            params![wf_id_str],
        )
        .map_err(|e| OrchestratorError::StatePersistence {
            reason: format!("delete prior steps for workflow in sqlite: {e}"),
        })?;

        for (idx, step) in state.steps.iter().enumerate() {
            let status_str = step_status_to_str(&step.status);
            let output_json = step
                .output
                .as_ref()
                .map(|v| serde_json::to_string(v))
                .transpose()
                .map_err(|e| OrchestratorError::StatePersistence {
                    reason: format!("serialize step output json for {}: {e}", step.id),
                })?;

            let (gate_type, gate_config) = step
                .gate
                .as_ref()
                .map(gate_info_to_sql_columns)
                .unwrap_or((None, None));

            tx.execute(
                r#"
                INSERT INTO steps (
                  step_id,
                  workflow_id,
                  step_index,
                  name,
                  status,
                  started_at,
                  completed_at,
                  duration_ms,
                  output,
                  gate_type,
                  gate_config
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    step.id,
                    wf_id_str,
                    idx as i64,
                    step.name,
                    status_str,
                    step.started_at,
                    step.completed_at,
                    step.duration_ms.map(|d| d as i64),
                    output_json,
                    gate_type,
                    gate_config,
                ],
            )
            .map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("insert step row {} in sqlite: {e}", step.id),
            })?;
        }

        tx.commit()
            .map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("commit sqlite transaction: {e}"),
            })?;

        Ok(())
    }

    /// Loads workflow state for `workflow_id` from SQLite.
    ///
    /// Returns `Ok(None)` if the workflow row does not exist.
    pub fn load_workflow_state(
        &self,
        workflow_id: Uuid,
    ) -> Result<Option<WorkflowState>, OrchestratorError> {
        let wf_id_str = workflow_id.to_string();

        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT workflow_name, status, started_at, completed_at, metadata
                FROM workflows
                WHERE workflow_id = ?1
                "#,
            )
            .map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("prepare workflow select in sqlite: {e}"),
            })?;

        let wf_row = stmt
            .query_row([&wf_id_str], |row| {
                let name: String = row.get(0)?;
                let status_str: String = row.get(1)?;
                let started_at: String = row.get(2)?;
                let _completed_at: Option<String> = row.get(3)?;
                let metadata_text: Option<String> = row.get(4)?;
                Ok((name, status_str, started_at, metadata_text))
            })
            .optional()
            .map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("query workflow row in sqlite: {e}"),
            })?;

        let Some((workflow_name, status_str, started_at, metadata_text)) = wf_row else {
            return Ok(None);
        };

        let status = workflow_status_from_str(&status_str).map_err(|msg| {
            OrchestratorError::StatePersistence {
                reason: format!("invalid workflow status '{status_str}' in sqlite: {msg}"),
            }
        })?;

        let metadata = if let Some(text) = metadata_text {
            let value: JsonValue = serde_json::from_str(&text).map_err(|e| {
                OrchestratorError::StatePersistence {
                    reason: format!("decode workflow metadata json from sqlite: {e}"),
                }
            })?;
            match value {
                JsonValue::Object(map) => map,
                _ => serde_json::Map::new(),
            }
        } else {
            serde_json::Map::new()
        };

        // Load steps ordered by step_index.
        let mut stmt_steps = self
            .conn
            .prepare(
                r#"
                SELECT
                  step_id,
                  step_index,
                  name,
                  status,
                  started_at,
                  completed_at,
                  duration_ms,
                  output,
                  gate_type,
                  gate_config
                FROM steps
                WHERE workflow_id = ?1
                ORDER BY step_index ASC
                "#,
            )
            .map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("prepare steps select in sqlite: {e}"),
            })?;

        let mut steps: Vec<StepState> = Vec::new();

        let rows = stmt_steps
            .query_map([&wf_id_str], |row| {
                let step_id: String = row.get(0)?;
                let _index: i64 = row.get(1)?;
                let name: String = row.get(2)?;
                let status_str: String = row.get(3)?;
                let started_at: Option<String> = row.get(4)?;
                let completed_at: Option<String> = row.get(5)?;
                let duration_ms: Option<i64> = row.get(6)?;
                let output_text: Option<String> = row.get(7)?;
                let gate_type: Option<String> = row.get(8)?;
                let gate_config_text: Option<String> = row.get(9)?;

                Ok((
                    step_id,
                    name,
                    status_str,
                    started_at,
                    completed_at,
                    duration_ms,
                    output_text,
                    gate_type,
                    gate_config_text,
                ))
            })
            .map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("query steps for workflow in sqlite: {e}"),
            })?;

        for row in rows {
            let (
                step_id,
                name,
                status_str,
                started_at,
                completed_at,
                duration_ms,
                output_text,
                gate_type,
                gate_config_text,
            ) = row.map_err(|e| OrchestratorError::StatePersistence {
                reason: format!("iterate step rows for workflow in sqlite: {e}"),
            })?;

            let status = step_status_from_str(&status_str).map_err(|msg| {
                OrchestratorError::StatePersistence {
                    reason: format!("invalid step status '{status_str}' in sqlite: {msg}"),
                }
            })?;

            let output = if let Some(text) = output_text {
                let value: JsonValue = serde_json::from_str(&text).map_err(|e| {
                    OrchestratorError::StatePersistence {
                        reason: format!("decode step output json from sqlite: {e}"),
                    }
                })?;
                Some(value)
            } else {
                None
            };

            let gate = if let Some(gate_type_str) = gate_type {
                Some(sql_columns_to_gate_info(
                    &gate_type_str,
                    gate_config_text.as_deref(),
                )?)
            } else {
                None
            };

            steps.push(StepState {
                id: step_id,
                name,
                status,
                started_at,
                completed_at,
                duration_ms: duration_ms.map(|d| d as u64),
                output,
                gate,
            });
        }

        let state = WorkflowState {
            version: 1,
            workflow_id,
            workflow_name,
            started_at,
            status,
            current_step_index: None,
            steps,
            metadata,
        };

        Ok(Some(state))
    }
}

fn workflow_status_to_str(status: &WorkflowStatus) -> &'static str {
    match status {
        WorkflowStatus::Running => "running",
        WorkflowStatus::Completed => "completed",
        WorkflowStatus::Failed => "failed",
        WorkflowStatus::TimedOut => "timed_out",
        WorkflowStatus::AwaitingCheckpoint => "awaiting_checkpoint",
    }
}

fn workflow_status_from_str(s: &str) -> Result<WorkflowStatus, &'static str> {
    match s {
        "running" => Ok(WorkflowStatus::Running),
        "completed" => Ok(WorkflowStatus::Completed),
        "failed" => Ok(WorkflowStatus::Failed),
        "timed_out" => Ok(WorkflowStatus::TimedOut),
        "awaiting_checkpoint" => Ok(WorkflowStatus::AwaitingCheckpoint),
        _ => Err("unrecognized workflow status"),
    }
}

fn step_status_to_str(status: &StepExecutionStatus) -> &'static str {
    match status {
        StepExecutionStatus::Pending => "pending",
        StepExecutionStatus::Running => "running",
        StepExecutionStatus::Completed => "completed",
        StepExecutionStatus::Failed => "failed",
        StepExecutionStatus::Skipped => "skipped",
    }
}

fn step_status_from_str(s: &str) -> Result<StepExecutionStatus, &'static str> {
    match s {
        "pending" => Ok(StepExecutionStatus::Pending),
        "running" => Ok(StepExecutionStatus::Running),
        "completed" => Ok(StepExecutionStatus::Completed),
        "failed" => Ok(StepExecutionStatus::Failed),
        "skipped" => Ok(StepExecutionStatus::Skipped),
        _ => Err("unrecognized step status"),
    }
}

fn gate_info_to_sql_columns(gate: &GateInfo) -> (Option<String>, Option<String>) {
    let gate_type = Some(gate.gate_type.clone());
    let config_json = gate
        .config
        .as_ref()
        .map(|v| serde_json::to_string(v))
        .transpose()
        .ok()
        .flatten();
    (gate_type, config_json)
}

fn sql_columns_to_gate_info(
    gate_type: &str,
    gate_config_json: Option<&str>,
) -> Result<GateInfo, OrchestratorError> {
    let config = if let Some(text) = gate_config_json {
        let value: JsonValue = serde_json::from_str(text).map_err(|e| {
            OrchestratorError::StatePersistence {
                reason: format!("decode gate_config json from sqlite: {e}"),
            }
        })?;
        Some(value)
    } else {
        None
    };

    Ok(GateInfo {
        gate_type: gate_type.to_string(),
        timeout_ms: None,
        config,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_round_trip_matches_json_state_for_basic_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let artifact_base = ArtifactManager::new(tmp.path());
        let wf_id = Uuid::new_v4();

        let mut meta = serde_json::Map::new();
        meta.insert("branch".to_string(), JsonValue::from("main"));

        let mut state = WorkflowState::new(
            wf_id,
            "deploy-staging",
            "2026-03-29T10:00:00Z".to_string(),
            vec![("step_001".to_string(), "lint".to_string())],
            meta,
        );
        state.mark_step_started("step_001", "2026-03-29T10:00:01Z".to_string());
        state.mark_step_finished(
            "step_001",
            StepExecutionStatus::Completed,
            "2026-03-29T10:00:05Z".to_string(),
            Some(4000),
            Some(serde_json::json!({"summary": "ok"})),
        );

        let db_path = sqlite_db_path_for_run(&artifact_base, wf_id);
        let mut store = SqliteWorkflowStore::open(&db_path).expect("open sqlite store");
        store
            .write_workflow_state(&state)
            .expect("write sqlite state");

        let loaded = store
            .load_workflow_state(wf_id)
            .expect("load sqlite state")
            .expect("expected workflow row");

        assert_eq!(loaded.workflow_id, state.workflow_id);
        assert_eq!(loaded.workflow_name, state.workflow_name);
        assert_eq!(loaded.status, state.status);
        assert_eq!(loaded.steps.len(), 1);
        assert_eq!(loaded.steps[0].id, "step_001");
        assert_eq!(loaded.steps[0].status, StepExecutionStatus::Completed);
        assert_eq!(
            loaded
                .metadata
                .get("branch")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "main"
        );
    }

    #[test]
    fn sqlite_store_enables_wal_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("state.sqlite");
        let store = SqliteWorkflowStore::open(&db_path).expect("open sqlite store");

        let mode: String = store
            .conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("query journal_mode");

        assert_eq!(mode.to_lowercase(), "wal");
    }
}

