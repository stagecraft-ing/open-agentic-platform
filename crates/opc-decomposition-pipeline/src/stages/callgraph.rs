// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
//
// Stage 4 — call graph. Phase 1: skeleton. Phase 5 wires xray's
// `analysis::call_graph::analyze_directory`.

use chrono::Utc;

use crate::error::PipelineError;
use crate::persistence::{RunDirectory, hash_stage_dir};
use crate::types::{PipelineConfig, StageId, StageRecord, StageStatus};

pub fn run(_config: &PipelineConfig, run_dir: &RunDirectory) -> Result<StageRecord, PipelineError> {
    let started_at = Utc::now();
    let stage_dir = run_dir.stage_dir(StageId::CallGraph);
    let placeholder = stage_dir.join("PENDING");
    std::fs::write(&placeholder, b"phase-5 wires call graph\n")
        .map_err(|e| PipelineError::io(&placeholder, e))?;
    let content_hash = hash_stage_dir(&stage_dir)?;
    Ok(StageRecord {
        id: StageId::CallGraph,
        status: StageStatus::Pending,
        content_hash,
        output_relpath: StageId::CallGraph.dir_name(),
        started_at,
        completed_at: Utc::now(),
        degraded: None,
    })
}
