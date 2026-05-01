// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/122-stakeholder-doc-inversion/spec.md — FR-014, FR-016, FR-018 to FR-021

//! Stage CD comparator (Phase 2 of the stage).
//!
//! Phase 3 ships the orchestration shell + a stub `run()` that emits an
//! empty diff so Stage CD's compare-mode wiring is exercised end-to-end.
//! Phase 4 fills in the pairing + 6-class diff classifier
//! (`wording | structural | scope | external-entity | ownership |
//! citation`) and integrates spec 121's allowlist + `anchor_hash`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComparatorMode {
    /// Standard run: gate evaluated as defined.
    Standard,
    /// Operator overrode the workspace policy threshold; gate may
    /// permit `structural` diffs that have explicit approval. Phase 5
    /// fills in the policy-driven semantics.
    OperatorOverride,
}

#[derive(Debug, Clone)]
pub struct ComparatorInputs {
    pub project: PathBuf,
    pub artifact_store: PathBuf,
    pub candidate_charter: PathBuf,
    pub candidate_client_document: PathBuf,
    pub authored_charter: PathBuf,
    pub authored_client_document: PathBuf,
    pub mode: ComparatorMode,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageCdDiff {
    /// Absolute timestamp the comparator ran.
    pub generated_at: DateTime<Utc>,
    /// Mirrors `StageCdMode` from the orchestrator. Phase 3 always
    /// records `compare` because the seed branch never reaches the
    /// comparator (FR-015).
    pub mode: String,
    /// Per-section diffs. Phase 3 emits an empty list — Phase 4 fills.
    pub findings: Vec<StageCdDiffFinding>,
    /// Aggregate counts per classification class. Phase 3 emits zeros.
    pub counts: StageCdDiffCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageCdDiffFinding {
    pub doc: String,
    pub anchor: String,
    /// One of `wording | structural | scope | external-entity |
    /// ownership | citation`.
    pub class: String,
    pub authored_excerpt: Option<String>,
    pub candidate_excerpt: Option<String>,
    /// Optional resolution recorded by an operator action (FR-024 /
    /// FR-026).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<DiffResolution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResolution {
    /// `rejected | accepted | force-approved`.
    pub action: String,
    pub actor: String,
    pub at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageCdDiffCounts {
    pub wording: u32,
    pub structural: u32,
    pub scope: u32,
    pub external_entity: u32,
    pub ownership: u32,
    pub citation: u32,
}

#[derive(Debug, Error)]
pub enum ComparatorError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Phase 3 stub. Returns an empty `StageCdDiff` so the orchestration
/// shell and the bytes-on-disk invariant test can exercise the
/// compare-mode wiring without depending on Phase 4's classifier.
/// Phase 4 replaces this body.
pub fn run(inputs: &ComparatorInputs) -> Result<StageCdDiff, ComparatorError> {
    let _ = inputs.candidate_charter;
    let _ = inputs.candidate_client_document;
    let _ = inputs.authored_charter;
    let _ = inputs.authored_client_document;
    let _ = inputs.project;
    let _ = inputs.artifact_store;
    let _ = inputs.mode;
    Ok(StageCdDiff {
        generated_at: inputs.now,
        mode: "compare".to_string(),
        findings: Vec::new(),
        counts: StageCdDiffCounts::default(),
    })
}
