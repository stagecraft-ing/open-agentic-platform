// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/167-born-with-spec-spine-kernel/spec.md

//! Born-with spec-spine kernel emission (spec 167).
//!
//! When `factory-engine` produces a new project from an adapter, the
//! resulting repo ships with a pre-populated spec-spine kernel:
//!
//! 1. A verbatim copy of `specs/000-bootstrap-spec-system/spec.md`.
//! 2. A verbatim copy of OAP's current `standards/spec/` directory.
//! 3. A pre-compiled `.derived/spec-registry/registry.json`.
//! 4. A `.kernel-version` marker recording source commit + content hash,
//!    factory-engine version, adapter identity + manifest hash, and the
//!    distribution mode chosen for tenant-side spine binaries (FR-005).
//! 5. Tenant-side gate wiring (GitHub Actions workflow + Makefile target)
//!    that invokes the coupling gate against the tenant's own spec spine.
//! 6. Adapter-seeded initial spec drafts capturing what was scaffolded.
//!
//! Emission is deterministic: two invocations on identical inputs produce
//! hash-equal kernels (FR-009).

pub mod adapter_specs;
pub mod emit;
pub mod gather;
pub mod templates;
pub mod version;

pub use adapter_specs::{AdapterSeededSpec, build_scaffold_claim_spec};
pub use emit::{EmissionMode, KernelEmissionConfig, KernelEmissionReport, emit_kernel};
pub use version::ToolchainMode;
pub use gather::{KernelContent, KernelSource, compute_kernel_hash, gather_kernel_content};
pub use templates::{
    TenantGateContext, TenantToolchainContext, render_tenant_makefile, render_tenant_toolchain,
    render_tenant_workflow,
};
pub use version::{AdapterIdentity, CertificateToolchainRef, KernelOrigin, KernelVersion};

use thiserror::Error;

/// Errors raised by the kernel-emission pipeline.
#[derive(Debug, Error)]
pub enum KernelEmissionError {
    #[error("kernel source path not found: {0}")]
    SourceNotFound(String),

    #[error("kernel source missing required entry: {0}")]
    SourceIncomplete(String),

    #[error("target path is not empty: {0}")]
    TargetNotEmpty(String),

    #[error("filesystem error during emission: {0}")]
    Io(#[from] std::io::Error),

    #[error("template render error: {0}")]
    Template(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("adapter manifest invalid: {0}")]
    InvalidAdapter(String),
}
