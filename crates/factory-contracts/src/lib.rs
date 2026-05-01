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
pub mod build_spec;
pub mod knowledge;
pub mod namespace;
pub mod pattern_resolver;
pub mod pipeline_state;
pub mod validation;
pub mod verification;

pub use adapter_manifest::AdapterManifest;
pub use adapter_registry::AdapterRegistry;
pub use agent_loader::AgentPrompt;
pub use build_spec::BuildSpec;
pub use pattern_resolver::PatternResolver;
pub use pipeline_state::PipelineState;
pub use verification::VerificationContract;
