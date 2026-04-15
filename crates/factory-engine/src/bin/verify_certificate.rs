// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/102-governed-excellence/spec.md — FR-007

//! CLI to independently verify a governance certificate.
//!
//! Exits 0 if the certificate is valid, 1 if any mismatch is detected.
//!
//! Usage:
//!   verify-certificate <path-to-certificate.json> [--artifact-dir <dir>]

use clap::Parser;
use factory_engine::governance_certificate::{GovernanceCertificate, verify_certificate};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "verify-certificate",
    about = "Verify a governance certificate by re-deriving hashes and checking proof chain integrity (spec 102 FR-007)"
)]
struct Cli {
    /// Path to the governance-certificate.json file.
    certificate: PathBuf,

    /// Optional directory containing stage artifacts for hash re-derivation.
    #[arg(long)]
    artifact_dir: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    let json = match std::fs::read_to_string(&cli.certificate) {
        Ok(j) => j,
        Err(e) => {
            eprintln!(
                "error: cannot read {}: {e}",
                cli.certificate.display()
            );
            std::process::exit(2);
        }
    };

    let cert: GovernanceCertificate = match serde_json::from_str(&json) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: invalid certificate JSON: {e}");
            std::process::exit(2);
        }
    };

    let result = verify_certificate(&cert, cli.artifact_dir.as_deref());

    if result.valid {
        eprintln!(
            "governance certificate VERIFIED (pipeline: {}, status: {:?})",
            cert.pipeline_run_id, cert.status
        );
        eprintln!("  stages: {}", cert.stages.len());
        eprintln!("  proof chain records: {}", cert.proof_chain.record_count);
        eprintln!("  certificate hash: {}", &cert.certificate_hash[..16]);
        std::process::exit(0);
    } else {
        eprintln!(
            "governance certificate INVALID ({} error(s)):",
            result.errors.len()
        );
        for err in &result.errors {
            eprintln!("  - {err}");
        }
        std::process::exit(1);
    }
}
