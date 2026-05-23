// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
//
// Stage 1 — extraction. Phase 1: skeleton. Implementation lands in
// Phase 2 and wires `artifact_extract::extract_deterministic`.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::PipelineError;
use crate::persistence::{RunDirectory, hash_stage_dir};
use crate::types::{DegradedReason, PipelineConfig, StageId, StageRecord, StageStatus};

/// Record persisted under `s1-extraction/index.json` summarising the
/// inputs the stage processed. The full `ExtractionOutput` objects
/// (one per file) land in `s1-extraction/objects/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionIndex {
    pub source_bundle: Option<String>,
    pub objects: Vec<ExtractionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionEntry {
    pub source_relpath: String,
    pub kind: String,
    pub output_relpath: String,
}

pub fn run(config: &PipelineConfig, run_dir: &RunDirectory) -> Result<StageRecord, PipelineError> {
    let started_at = Utc::now();
    let stage_dir = run_dir.stage_dir(StageId::Extraction);
    let index_path = stage_dir.join("index.json");

    // Phase-1 skeleton: write an empty index marking the stage as
    // degraded due to "no knowledge bundle" so downstream stages see a
    // consistent shape. Phase 2 replaces this with real extraction.
    let index = ExtractionIndex {
        source_bundle: config.knowledge_bundle.as_ref().map(|p| p.display().to_string()),
        objects: Vec::new(),
    };
    let bytes = serde_json::to_vec_pretty(&index)?;
    std::fs::write(&index_path, bytes).map_err(|e| PipelineError::io(&index_path, e))?;

    let content_hash = hash_stage_dir(&stage_dir)?;
    Ok(StageRecord {
        id: StageId::Extraction,
        status: StageStatus::Degraded,
        content_hash,
        output_relpath: StageId::Extraction.dir_name(),
        started_at,
        completed_at: Utc::now(),
        degraded: Some(DegradedReason::NoKnowledgeBundle),
    })
}
