// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
//
// Stage 6 — deterministic baseline synthesiser. Phase 1: skeleton.
// Phase 7 emits draft spec.md files satisfying spec 161/147/154.

use chrono::Utc;

use crate::error::PipelineError;
use crate::persistence::{RunDirectory, hash_stage_dir};
use crate::types::{DraftSpecRef, PipelineConfig, StageId, StageRecord, StageStatus};

pub struct SynthesisOutput {
    pub record: StageRecord,
    pub emitted: Vec<DraftSpecRef>,
}

pub fn run(
    _config: &PipelineConfig,
    run_dir: &RunDirectory,
) -> Result<SynthesisOutput, PipelineError> {
    let started_at = Utc::now();
    let stage_dir = run_dir.stage_dir(StageId::Synthesis);
    let placeholder = stage_dir.join("PENDING");
    std::fs::write(&placeholder, b"phase-7 emits draft specs\n")
        .map_err(|e| PipelineError::io(&placeholder, e))?;
    let content_hash = hash_stage_dir(&stage_dir)?;
    Ok(SynthesisOutput {
        record: StageRecord {
            id: StageId::Synthesis,
            status: StageStatus::Pending,
            content_hash,
            output_relpath: StageId::Synthesis.dir_name(),
            started_at,
            completed_at: Utc::now(),
            degraded: None,
        },
        emitted: Vec::new(),
    })
}
