// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::PipelineError;
use crate::types::{PipelineRun, RunId, StageId};

/// Filesystem layout under `<project>/.opc/decomposition/<run-id>/`.
///
/// ```text
/// <run_id>/
///   run.json                # PipelineRun manifest
///   s1-extraction/
///   s2-fingerprint/
///   s3-clusters/
///   s4-callgraph/
///   s5-lineage/
///   s6-synthesis/
///     specs/
///       NNN-slug/spec.md    # one per cluster
/// ```
pub struct RunDirectory {
    root: PathBuf,
    run_id: RunId,
}

impl RunDirectory {
    /// Build a handle without creating directories. Call `ensure()` to
    /// materialise the on-disk layout.
    pub fn new(output_root: impl Into<PathBuf>, run_id: RunId) -> Self {
        let root = output_root.into().join(run_id.as_str());
        Self { root, run_id }
    }

    pub fn run_id(&self) -> &RunId {
        &self.run_id
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ensure(&self) -> Result<(), PipelineError> {
        fs::create_dir_all(&self.root).map_err(|e| PipelineError::io(&self.root, e))?;
        for stage in StageId::all() {
            let dir = self.stage_dir(stage);
            fs::create_dir_all(&dir).map_err(|e| PipelineError::io(&dir, e))?;
        }
        // Synthesis nests `specs/` underneath itself.
        let specs_dir = self.synthesis_specs_dir();
        fs::create_dir_all(&specs_dir).map_err(|e| PipelineError::io(&specs_dir, e))?;
        Ok(())
    }

    pub fn stage_dir(&self, stage: StageId) -> PathBuf {
        self.root.join(stage.dir_name())
    }

    pub fn synthesis_specs_dir(&self) -> PathBuf {
        self.stage_dir(StageId::Synthesis).join("specs")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("run.json")
    }

    pub fn write_manifest(&self, run: &PipelineRun) -> Result<(), PipelineError> {
        let path = self.manifest_path();
        let bytes = serde_json::to_vec_pretty(run)?;
        fs::write(&path, bytes).map_err(|e| PipelineError::io(&path, e))?;
        Ok(())
    }

    pub fn load_manifest(&self) -> Result<Option<PipelineRun>, PipelineError> {
        let path = self.manifest_path();
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(PipelineError::io(&path, e)),
        }
    }
}

/// Hash a stage's on-disk output directory by SHA-256-ing the
/// concatenated bytes of each regular file in lexicographic order. The
/// orchestrator uses this as the cache key for stage re-runs and as
/// the value persisted in `StageRecord::content_hash`.
pub fn hash_stage_dir(dir: &Path) -> Result<String, PipelineError> {
    let mut entries: Vec<PathBuf> = Vec::new();
    if dir.exists() {
        for entry in walkdir::WalkDir::new(dir)
            .sort_by_file_name()
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            entries.push(entry.into_path());
        }
    }
    let mut hasher = Sha256::new();
    for path in entries {
        let rel = path.strip_prefix(dir).unwrap_or(&path);
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update(b"\0");
        let bytes = fs::read(&path).map_err(|e| PipelineError::io(&path, e))?;
        hasher.update(&bytes);
        hasher.update(b"\0");
    }
    Ok(hex::encode(hasher.finalize()))
}

/// SHA-256 over the bytes of a single file, lowercase hex. Used by
/// the synthesiser to populate `DraftSpecRef::content_hash`.
pub fn hash_file(path: &Path) -> Result<String, PipelineError> {
    let bytes = fs::read(path).map_err(|e| PipelineError::io(path, e))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}
