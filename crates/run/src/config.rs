// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: TASK_RUNNER
// Spec: spec/run/skills.md

use crate::runner::{RunConfig, Skill};
use crate::state::{SkillResult, SkillStatus};
use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Deserialize)]
pub struct TasksConfig {
    #[serde(default)]
    pub skills: HashMap<String, TaskDef>,
}

#[derive(Debug, Deserialize)]
pub struct TaskDef {
    pub command: Vec<String>,
    pub description: Option<String>,
    /// Timeout in milliseconds. Default: 300 000 ms (5 min).
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn default_timeout_ms() -> u64 {
    300_000
}

pub struct ConfiguredSkill {
    id: String,
    def: TaskDef,
    base_cwd: PathBuf,
}

impl Skill for ConfiguredSkill {
    fn id(&self) -> &str {
        &self.id
    }

    fn run(&self, config: &RunConfig) -> Result<SkillResult> {
        let (prog, args) = self
            .def
            .command
            .split_first()
            .ok_or_else(|| anyhow!("Command is empty for skill '{}'", self.id))?;

        let mut cmd = Command::new(prog);
        cmd.args(args);

        let cwd = self
            .def
            .cwd
            .as_ref()
            .map(|c| self.base_cwd.join(c))
            .unwrap_or_else(|| self.base_cwd.clone());
        cmd.current_dir(&cwd);

        for (k, v) in &config.env {
            cmd.env(k, v);
        }
        for (k, v) in &self.def.env {
            cmd.env(k, v);
        }

        let output = cmd
            .output()
            .map_err(|e| anyhow!("Failed to spawn skill '{}': {}", self.id, e))?;

        if output.status.success() {
            return Ok(SkillResult {
                skill: self.id.clone(),
                status: SkillStatus::Pass,
                exit_code: 0,
                note: None,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);
        let lines: Vec<&str> = combined.lines().collect();
        let note = if lines.len() > 20 {
            format!("...(truncated)...\n{}", lines[lines.len() - 20..].join("\n"))
        } else {
            combined.trim().to_string()
        };

        Ok(SkillResult {
            skill: self.id.clone(),
            status: SkillStatus::Fail,
            exit_code: output.status.code().unwrap_or(1),
            note: Some(note),
        })
    }
}

/// Load skills from `axiomregent.tasks.yaml` at `path`.
/// Returns a list of configured skills. On malformed YAML, returns an error.
/// Silently skips entries with empty command lists.
pub fn load_from_file(path: &Path, base_cwd: &Path) -> Result<Vec<Box<dyn Skill>>> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("Cannot read {}: {}", path.display(), e))?;
    let cfg: TasksConfig = serde_yaml::from_str(&contents)
        .map_err(|e| anyhow!("Cannot parse {}: {}", path.display(), e))?;

    let mut skills: Vec<Box<dyn Skill>> = Vec::new();
    for (id, def) in cfg.skills {
        if def.command.is_empty() {
            continue;
        }
        skills.push(Box::new(ConfiguredSkill {
            id,
            def,
            base_cwd: base_cwd.to_path_buf(),
        }));
    }
    skills.sort_by(|a, b| a.id().cmp(b.id()));
    Ok(skills)
}
