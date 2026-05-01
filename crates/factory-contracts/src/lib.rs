// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Rust contract types for the Factory delivery engine (spec 074).
//!
//! This crate provides typed Rust representations of all four Factory contract
//! schemas: Build Spec, Adapter Manifest, Pipeline State, and Verification
//! Contract. It also provides adapter discovery, agent prompt loading, pattern
//! resolution, and schema validation.

pub mod adapter_manifest;
pub mod adapter_registry;
pub mod agent_loader;
pub mod agent_reference;
pub mod budget;
pub mod build_spec;
pub mod knowledge;
pub mod namespace;
pub mod pattern_resolver;
pub mod pipeline_state;
pub mod provenance;
pub mod provenance_config;
pub mod stakeholder_docs;
pub mod validation;
pub mod verification;

pub use adapter_manifest::AdapterManifest;
pub use adapter_registry::AdapterRegistry;
pub use agent_loader::AgentPrompt;
pub use agent_reference::AgentReference;
pub use budget::AssumptionBudget;
pub use build_spec::BuildSpec;
pub use pattern_resolver::PatternResolver;
pub use pipeline_state::PipelineState;
pub use provenance::{
    anchor_canonical_tokens, anchor_hash, quote_hash,
    PROVENANCE_SCHEMA_VERSION,
};
pub use provenance_config::{
    FactoryProvenanceMode, ProvenanceConfig, ProvenanceConfigError,
};
pub use stakeholder_docs::{
    AnchorKind, AnchoredSection, AppliedFromEntry, AuthoringStatus, DocKind,
    SectionAnchor, SemVer, StakeholderDoc, StakeholderDocParseError,
    StakeholderFrontmatter, STAKEHOLDER_DOC_SCHEMA_VERSION,
};
pub use verification::VerificationContract;

/// Re-export of `chrono::DateTime`, `Utc`, and `TimeZone` so downstream
/// crates that consume contract types like `AssumptionTag.expires_at`
/// can name them without adding `chrono` as a direct dependency. The
/// `provenance-validator` crate (FR-001 caps its `[dependencies]` at the
/// five enumerated crates) consumes the chrono types exclusively through
/// this re-export.
pub use chrono::{DateTime, TimeZone, Utc};

/// Wall-clock timestamp helper for callers that need `now` without
/// pulling `chrono` into their own `[dependencies]`. Used by
/// `provenance-validator`'s audit binary entry point.
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}
