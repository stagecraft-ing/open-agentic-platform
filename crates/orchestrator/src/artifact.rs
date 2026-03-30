// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/044-multi-agent-orchestration/spec.md

use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Default artifact root when `OAP_ARTIFACT_DIR` is unset (044 FR-003).
pub const DEFAULT_ARTIFACT_DIR: &str = "/tmp/oap-artifacts";

/// Resolves `$OAP_ARTIFACT_DIR/<run_id>/<step_id>/...` paths (044).
#[derive(Clone, Debug)]
pub struct ArtifactManager {
    base_dir: PathBuf,
}

impl ArtifactManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Reads `OAP_ARTIFACT_DIR` or returns [`DEFAULT_ARTIFACT_DIR`].
    pub fn from_env() -> Self {
        let base = std::env::var("OAP_ARTIFACT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_ARTIFACT_DIR));
        Self::new(base)
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn run_dir(&self, run_id: Uuid) -> PathBuf {
        self.base_dir.join(run_id.to_string())
    }

    pub fn step_dir(&self, run_id: Uuid, step_id: &str) -> PathBuf {
        self.run_dir(run_id).join(step_id)
    }

    /// Absolute path for an output artifact file name within a step (044).
    pub fn output_artifact_path(
        &self,
        run_id: Uuid,
        step_id: &str,
        filename: &str,
    ) -> PathBuf {
        self.step_dir(run_id, step_id).join(filename)
    }

    /// Deletes `<base>/<run_id>/` recursively (044 `cleanup_artifacts`).
    pub fn cleanup_run(&self, run_id: Uuid) -> std::io::Result<()> {
        let dir = self.run_dir(run_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    /// Ensures the step directory exists.
    pub fn ensure_step_dir(&self, run_id: Uuid, step_id: &str) -> std::io::Result<PathBuf> {
        let p = self.step_dir(run_id, step_id);
        std::fs::create_dir_all(&p)?;
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_under_base() {
        let am = ArtifactManager::new("/tmp/oap-artifacts");
        let run = Uuid::nil();
        let p = am.output_artifact_path(run, "step-01", "out.md");
        assert!(p.to_string_lossy().contains("00000000-0000-0000-0000-000000000000"));
        assert!(p.to_string_lossy().ends_with("step-01/out.md"));
    }
}
