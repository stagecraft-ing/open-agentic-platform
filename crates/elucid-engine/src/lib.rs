// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/075-elucid-workflow-engine/spec.md

//! Elucid Workflow Engine — two-phase orchestration, manifest generation,
//! verification harness integration, and policy shard generation (spec 075).
//!
//! This crate extends the OAP orchestrator to natively support Elucid pipeline
//! workflows: process stages (s0–s5) that produce a Build Spec, followed by
//! dynamic scaffolding fan-out (s6a–s6g) derived from the Build Spec content.

pub mod agent_bridge;
pub mod engine;
pub mod manifest_gen;
pub mod pipeline_state;
pub mod policy_shard;
pub mod topo_sort;
pub mod verify_harness;

pub use agent_bridge::ElucidAgentBridge;
pub use engine::{ElucidEngine, ElucidEngineConfig};
pub use manifest_gen::{generate_process_manifest, generate_scaffold_manifest};
pub use pipeline_state::{ElucidPhase, ElucidPipelineState, ScaffoldingProgress};
pub use policy_shard::generate_elucid_policy_shard;
pub use verify_harness::run_elucid_gate_check;

use thiserror::Error;

/// Errors specific to the Elucid workflow engine.
#[derive(Debug, Error)]
pub enum ElucidError {
    #[error("adapter not found: {name}")]
    AdapterNotFound { name: String },

    #[error("invalid build spec: {reason}")]
    InvalidBuildSpec { reason: String },

    #[error("manifest generation failed: {reason}")]
    ManifestGeneration { reason: String },

    #[error("circular entity reference: {cycle}")]
    CircularReference { cycle: String },

    #[error("verification harness failed: {reason}")]
    VerificationHarness { reason: String },

    #[error("phase transition error: {reason}")]
    PhaseTransition { reason: String },

    #[error("orchestrator error: {0}")]
    Orchestrator(#[from] orchestrator::OrchestratorError),
}
