// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/102-governed-excellence/spec.md — FR-003, FR-009

//! CLI to build a governance certificate from an existing factory run
//! directory, without re-running the pipeline.
//!
//! Useful for retroactive certification (auditor receives a run directory
//! and wants the certificate) and for the `make build-certificate FILE=...`
//! demo flow that pairs with `verify-certificate` to close the OWASP ASI
//! traceability story end-to-end.
//!
//! The run directory layout matches the orchestrator's `ArtifactManager`:
//! `<run-dir>/<step-id>/<artifact-files>`. The build-spec hash is derived
//! from `<run-dir>/s5-ui-specification/build-spec.yaml` when present.
//!
//! Usage:
//!   build-certificate <run-dir> \
//!     [--adapter <name>] [--requirements-hash <hash>] \
//!     [--business-docs <path> ...] [--out <path>]

use clap::Parser;
use factory_engine::{
    FactoryPipelineState, generate_certificate, governance_certificate::sha256_file,
    persist_certificate,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "build-certificate",
    about = "Build a governance certificate from a factory run directory (spec 102 FR-003)"
)]
struct Cli {
    /// Path to the factory run directory (`.factory/runs/<run_id>`).
    run_dir: PathBuf,

    /// Adapter name. Defaults to `unknown` if not supplied.
    #[arg(long, default_value = "unknown")]
    adapter: String,

    /// SHA-256 of the input requirements documents. If `--business-docs`
    /// is supplied, the hash is computed from those files and this flag
    /// is ignored.
    #[arg(long)]
    requirements_hash: Option<String>,

    /// Optional requirement document paths. When supplied, their concatenated
    /// SHA-256 is recorded as `intent.requirementsHash`.
    #[arg(long, num_args = 1..)]
    business_docs: Vec<PathBuf>,

    /// Override certificate output path. Defaults to
    /// `<run-dir>/governance-certificate.json`.
    #[arg(long)]
    out: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    if !cli.run_dir.is_dir() {
        eprintln!(
            "error: run directory does not exist: {}",
            cli.run_dir.display()
        );
        std::process::exit(2);
    }

    let pipeline_id = cli
        .run_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut state = FactoryPipelineState::new(&pipeline_id, &cli.adapter);

    // Lift the build-spec hash from disk if the frozen artifact is present.
    let build_spec_path = cli
        .run_dir
        .join("s5-ui-specification")
        .join("build-spec.yaml");
    if build_spec_path.is_file() {
        match sha256_file(&build_spec_path) {
            Ok(hash) => state.transition_to_scaffolding(hash),
            Err(e) => eprintln!(
                "warning: could not hash build-spec at {}: {e}",
                build_spec_path.display()
            ),
        }
    }

    let requirements_hash = if !cli.business_docs.is_empty() {
        let mut hasher = Sha256::new();
        for p in &cli.business_docs {
            match std::fs::read(p) {
                Ok(bytes) => hasher.update(&bytes),
                Err(e) => {
                    eprintln!("warning: could not read {}: {e}", p.display());
                }
            }
        }
        format!("{:x}", hasher.finalize())
    } else {
        cli.requirements_hash.unwrap_or_default()
    };

    let cert = generate_certificate(&state, &requirements_hash, &cli.run_dir, None);

    let out_dir = match cli.out.as_ref() {
        Some(p) => p
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")),
        None => cli.run_dir.clone(),
    };

    if let Err(e) = persist_certificate(&cert, &out_dir) {
        eprintln!("error: failed to persist certificate at {}: {e}", out_dir.display());
        std::process::exit(1);
    }

    let cert_path = out_dir.join("governance-certificate.json");
    println!(
        "governance certificate written: {} (status={:?}, stages={}, hash={}…)",
        cert_path.display(),
        cert.status,
        cert.stages.len(),
        &cert.certificate_hash[..16]
    );
}
