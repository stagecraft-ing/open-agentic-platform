// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
//
// Stage 5 — temporal lineage from git history. Phase 1: skeleton.
// Phase 6 shells out to `git log` per logical unit.

use chrono::Utc;

use crate::error::PipelineError;
use crate::persistence::{RunDirectory, hash_stage_dir};
use crate::types::{PipelineConfig, StageId, StageRecord, StageStatus};

pub fn run(_config: &PipelineConfig, run_dir: &RunDirectory) -> Result<StageRecord, PipelineError> {
    let started_at = Utc::now();
    let stage_dir = run_dir.stage_dir(StageId::Lineage);
    let placeholder = stage_dir.join("PENDING");
    std::fs::write(&placeholder, b"phase-6 wires git lineage\n")
        .map_err(|e| PipelineError::io(&placeholder, e))?;
    let content_hash = hash_stage_dir(&stage_dir)?;
    Ok(StageRecord {
        id: StageId::Lineage,
        status: StageStatus::Pending,
        content_hash,
        output_relpath: StageId::Lineage.dir_name(),
        started_at,
        completed_at: Utc::now(),
        degraded: None,
    })
}
