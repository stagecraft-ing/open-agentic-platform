// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
//
// Stage 2 — structural fingerprint. Phase 1: skeleton. Phase 3 wires
// `xray::scan_target` + `xray::fingerprint::generate_fingerprint`.

use chrono::Utc;

use crate::error::PipelineError;
use crate::persistence::{RunDirectory, hash_stage_dir};
use crate::types::{PipelineConfig, StageId, StageRecord, StageStatus};

pub fn run(_config: &PipelineConfig, run_dir: &RunDirectory) -> Result<StageRecord, PipelineError> {
    let started_at = Utc::now();
    let stage_dir = run_dir.stage_dir(StageId::Fingerprint);
    let placeholder = stage_dir.join("PENDING");
    std::fs::write(&placeholder, b"phase-3 wires xray\n")
        .map_err(|e| PipelineError::io(&placeholder, e))?;
    let content_hash = hash_stage_dir(&stage_dir)?;
    Ok(StageRecord {
        id: StageId::Fingerprint,
        status: StageStatus::Pending,
        content_hash,
        output_relpath: StageId::Fingerprint.dir_name(),
        started_at,
        completed_at: Utc::now(),
        degraded: None,
    })
}
