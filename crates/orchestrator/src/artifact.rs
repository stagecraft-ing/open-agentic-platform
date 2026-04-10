// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/044-multi-agent-orchestration/spec.md

use sha2::{Digest, Sha256};
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
    pub fn output_artifact_path(&self, run_id: Uuid, step_id: &str, filename: &str) -> PathBuf {
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

    /// SHA-256 hash of the file at `path`, hex-encoded (64 chars).
    ///
    /// Spec 082 FR-001.
    pub fn hash_artifact(path: &Path) -> std::io::Result<String> {
        let mut hasher = Sha256::new();
        let mut file = std::fs::File::open(path)?;
        std::io::copy(&mut file, &mut hasher)?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Returns `true` if the file at `path` hashes to `expected_hash`.
    ///
    /// Spec 082 FR-001.
    pub fn verify_artifact(path: &Path, expected_hash: &str) -> std::io::Result<bool> {
        let actual = Self::hash_artifact(path)?;
        Ok(actual == expected_hash)
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
        assert!(
            p.to_string_lossy()
                .contains("00000000-0000-0000-0000-000000000000")
        );
        assert!(p.to_string_lossy().ends_with("step-01/out.md"));
    }

    #[test]
    fn hash_artifact_produces_64_char_hex() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("data.txt");
        std::fs::write(&path, "hello world\n").unwrap();
        let hash = ArtifactManager::hash_artifact(&path).unwrap();
        assert_eq!(hash.len(), 64, "SHA-256 hex should be 64 chars");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn verify_artifact_detects_match_and_mismatch() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("data.txt");
        std::fs::write(&path, "test content").unwrap();

        let hash = ArtifactManager::hash_artifact(&path).unwrap();
        assert!(ArtifactManager::verify_artifact(&path, &hash).unwrap());
        assert!(
            !ArtifactManager::verify_artifact(
                &path,
                "0000000000000000000000000000000000000000000000000000000000000000"
            )
            .unwrap()
        );
    }

    #[test]
    fn verify_artifact_detects_tampering() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("artifact.md");
        std::fs::write(&path, "original content").unwrap();
        let hash = ArtifactManager::hash_artifact(&path).unwrap();

        // Tamper with file
        std::fs::write(&path, "tampered content").unwrap();
        assert!(!ArtifactManager::verify_artifact(&path, &hash).unwrap());
    }
}
