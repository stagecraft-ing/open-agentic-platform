// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Pipeline orchestrator.
//!
//! Walks stages 1 -> 6 in fixed order. Each stage writes its outputs
//! into `<run>/<sN-stage>/` and returns a `StageRecord` summarising
//! status + content hash. The orchestrator collects records, persists
//! the assembled `PipelineRun` manifest, and returns it.

use chrono::Utc;

use crate::error::PipelineError;
use crate::persistence::RunDirectory;
use crate::stages::{callgraph, clustering, extraction, fingerprint, lineage, synthesis};
use crate::types::{PIPELINE_RUN_SCHEMA_VERSION, PipelineConfig, PipelineRun, RunId};

pub struct PipelineRunner {
    config: PipelineConfig,
}

impl PipelineRunner {
    pub fn new(config: PipelineConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &PipelineConfig {
        &self.config
    }

    /// Execute the full pipeline. Creates the run directory layout,
    /// runs each stage in order, persists the manifest, returns it.
    pub fn run(&self) -> Result<PipelineRun, PipelineError> {
        if !self.config.project_root.is_dir() {
            return Err(PipelineError::InvalidProjectRoot(
                self.config.project_root.clone(),
            ));
        }

        let started_at = Utc::now();
        let run_id = RunId::new(&self.config.project_root, started_at);
        let run_dir = RunDirectory::new(&self.config.output_root, run_id.clone());
        run_dir.ensure()?;

        let mut stages = Vec::with_capacity(6);
        stages.push(extraction::run(&self.config, &run_dir)?);
        stages.push(fingerprint::run(&self.config, &run_dir)?);
        stages.push(clustering::run(&self.config, &run_dir)?);
        stages.push(callgraph::run(&self.config, &run_dir)?);
        stages.push(lineage::run(&self.config, &run_dir)?);

        let synth = synthesis::run(&self.config, &run_dir)?;
        stages.push(synth.record);
        let emitted_specs = synth.emitted;

        let completed_at = Utc::now();
        let manifest = PipelineRun {
            run_id,
            project_root: self.config.project_root.clone(),
            schema_version: PIPELINE_RUN_SCHEMA_VERSION.to_string(),
            started_at,
            completed_at: Some(completed_at),
            stages,
            emitted_specs,
            embeddings_enabled: self.config.embeddings_enabled,
        };
        run_dir.write_manifest(&manifest)?;

        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn pipeline_creates_run_layout() {
        let project = tempdir().unwrap();
        let output_root = project.path().join(".opc").join("decomposition");

        // Drop a file so the working tree isn't empty.
        std::fs::write(project.path().join("README.md"), "# fixture").unwrap();

        let cfg = PipelineConfig {
            project_root: project.path().to_path_buf(),
            knowledge_bundle: None,
            output_root: output_root.clone(),
            embeddings_enabled: false,
        };
        let manifest = PipelineRunner::new(cfg).run().expect("pipeline run");
        assert_eq!(manifest.stages.len(), 6);
        assert!(manifest.run_id.as_str().len() > 0);
        assert!(output_root.join(manifest.run_id.as_str()).is_dir());
        assert!(
            output_root
                .join(manifest.run_id.as_str())
                .join("run.json")
                .is_file()
        );
    }
}
