// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: TASK_RUNNER
// Spec: spec/run/skills.md

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use log::error;
use run::{RunConfig, Runner, StateStore, registry};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

pub struct RunTools {
    runs: Arc<Mutex<HashMap<String, RunContext>>>,
    state_dir: PathBuf,
    run_root: PathBuf,
}

#[derive(Clone)]
struct RunContext {
    status: String, // "pending", "running", "pass", "fail"
    logs_path: PathBuf,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    exit_code: Option<i32>,
}

impl RunTools {
    pub fn new(root: &Path) -> Self {
        let state_dir = root.join(".axiomregent/run");
        let logs_dir = state_dir.join("logs");
        fs::create_dir_all(&logs_dir).unwrap_or(());

        Self {
            runs: Arc::new(Mutex::new(HashMap::new())),
            state_dir,
            run_root: root.to_path_buf(),
        }
    }

    pub fn execute(
        &self,
        skill: String,
        env_vars: Option<HashMap<String, String>>,
    ) -> Result<serde_json::Value> {
        let run_id = Uuid::new_v4().to_string();
        let logs_path = self.state_dir.join("logs").join(format!("{}.log", run_id));

        let context = RunContext {
            status: "pending".to_string(),
            logs_path: logs_path.clone(),
            start_time: Some(Utc::now()),
            end_time: None,
            exit_code: None,
        };

        {
            let mut runs = self.runs.lock().unwrap();
            runs.insert(run_id.clone(), context);
        }

        let runs_handle = self.runs.clone();
        let run_id_clone = run_id.clone();
        let state_dir_str = self.state_dir.to_string_lossy().into_owned();
        let run_root_str = self.run_root.to_string_lossy().into_owned();
        let logs_path_clone = logs_path.clone();

        thread::spawn(move || {
            {
                let mut runs = runs_handle.lock().unwrap();
                if let Some(ctx) = runs.get_mut(&run_id_clone) {
                    ctx.status = "running".to_string();
                }
            }

            let log_file = match File::create(&logs_path_clone) {
                Ok(f) => f,
                Err(e) => {
                    let mut runs = runs_handle.lock().unwrap();
                    if let Some(ctx) = runs.get_mut(&run_id_clone) {
                        ctx.status = "fail".to_string();
                        ctx.end_time = Some(Utc::now());
                    }
                    error!("Failed to create log file: {}", e);
                    return;
                }
            };

            // Setup RunConfig
            let current_exe = env::current_exe().unwrap_or_else(|_| "axiomregent".into());
            let bin_path = current_exe.to_string_lossy().into_owned();

            let config = RunConfig {
                json: false,
                state_dir: state_dir_str.clone(),
                fail_on_warning: false,
                files0: false,
                bin_path,
                stdin_buffer: None,
                env: env_vars.unwrap_or_default(),
            };

            let store = StateStore::new(&state_dir_str);
            let registry = registry::get_registry(Some(Path::new(&run_root_str)));
            let runner = Runner::new(registry, store, config, Some(Box::new(log_file)));

            let result = runner.run_specific(&[skill]);

            let mut runs = runs_handle.lock().unwrap();
            if let Some(ctx) = runs.get_mut(&run_id_clone) {
                ctx.end_time = Some(Utc::now());
                match result {
                    Ok(true) => {
                        ctx.status = "pass".to_string();
                        ctx.exit_code = Some(0);
                    }
                    Ok(false) => {
                        ctx.status = "fail".to_string();
                        ctx.exit_code = Some(1);
                    }
                    Err(_) => {
                        ctx.status = "fail".to_string();
                        ctx.exit_code = Some(1);
                    }
                }
            }
        });

        Ok(serde_json::json!({"run_id": run_id}))
    }

    pub fn status(&self, run_id: &str) -> Result<serde_json::Value> {
        let runs = self.runs.lock().unwrap();
        if let Some(ctx) = runs.get(run_id) {
            Ok(serde_json::json!({
                "run_id": run_id,
                "status": ctx.status,
                "start_time": ctx.start_time,
                "end_time": ctx.end_time,
                "exit_code": ctx.exit_code,
                "note": null
            }))
        } else {
            Ok(serde_json::json!({ "run_id": run_id, "status": "unknown" }))
        }
    }

    pub fn logs(&self, run_id: &str, offset: Option<u64>, limit: Option<u64>) -> Result<serde_json::Value> {
        let logs_path = {
            let runs = self.runs.lock().unwrap();
            if let Some(ctx) = runs.get(run_id) {
                ctx.logs_path.clone()
            } else {
                return Ok(serde_json::json!({
                    "run_id": run_id,
                    "lines": [],
                    "total": 0,
                    "truncated": false
                }));
            }
        };

        if !logs_path.exists() {
            return Ok(serde_json::json!({
                "run_id": run_id,
                "lines": [],
                "total": 0,
                "truncated": false
            }));
        }

        let contents = fs::read_to_string(&logs_path).context("reading log file")?;
        let all_lines: Vec<&str> = contents.lines().collect();
        let total = all_lines.len() as u64;

        let line_offset = offset.unwrap_or(0) as usize;
        let sliced: &[&str] = if line_offset < all_lines.len() {
            &all_lines[line_offset..]
        } else {
            &[]
        };

        let (lines, truncated) = if let Some(lim) = limit {
            let lim = lim as usize;
            if sliced.len() > lim {
                (&sliced[..lim], true)
            } else {
                (sliced, false)
            }
        } else {
            (sliced, false)
        };

        Ok(serde_json::json!({
            "run_id": run_id,
            "lines": lines,
            "total": total,
            "truncated": truncated
        }))
    }
}
