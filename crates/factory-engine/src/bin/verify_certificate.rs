// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/102-governed-excellence/spec.md — FR-007
// Spec: specs/198-factory-governance-envelope/spec.md — FR-014 (platform seal)

//! CLI to independently verify a governance certificate.
//!
//! Exits 0 if the certificate is valid, 1 if any mismatch is detected.
//!
//! Usage:
//!   verify-certificate <path-to-certificate.json> [--artifact-dir <dir>]
//!       [--platform-jwks <file> | --jwks-url <url>] [--require-sealed]
//!
//! The offline artifact-chain check is unchanged and needs no keys. The
//! platform seal (spec 198 FR-014) is verified when a JWKS is supplied:
//! `--platform-jwks` reads a saved JWKS JSON file (air-gapped audit);
//! `--jwks-url` fetches `GET <stagecraft>/api/factory/.well-known/jwks.json`.
//! A certificate with no countersign verifies as visibly UNSEALED (exit 0)
//! unless `--require-sealed` is set.

use clap::Parser;
use factory_engine::governance_certificate::{
    GovernanceCertificate, verify_certificate_with_platform,
};
use factory_engine::platform_jws::PlatformJwks;
use factory_engine::{validate_spec_id_resolution, write_validation_warnings};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "verify-certificate",
    about = "Verify a governance certificate by re-deriving hashes, checking proof chain integrity (spec 102 FR-007), and adjudicating the platform seal (spec 198 FR-014)"
)]
struct Cli {
    /// Path to the governance-certificate.json file.
    certificate: PathBuf,

    /// Optional directory containing stage artifacts for hash re-derivation.
    #[arg(long)]
    artifact_dir: Option<PathBuf>,

    /// Repository root used to locate `build/spec-registry/registry.json`
    /// for `intent.spec_id` resolution validation (Cut D W-10 / spec 102 G-2).
    /// Defaults to the current working directory.
    #[arg(long)]
    repo_root: Option<PathBuf>,

    /// Path to a saved platform JWKS JSON file (offline seal verification).
    #[arg(long, conflicts_with = "jwks_url")]
    platform_jwks: Option<PathBuf>,

    /// URL of the platform JWKS endpoint
    /// (e.g. https://stagecraft.example/api/factory/.well-known/jwks.json).
    #[arg(long)]
    jwks_url: Option<String>,

    /// Fail (exit 1) when the certificate carries no verifiable platform
    /// countersign. Default posture reports unsealed certificates as a
    /// visible notice with exit 0.
    #[arg(long)]
    require_sealed: bool,
}

fn load_jwks(cli: &Cli) -> Option<PlatformJwks> {
    if let Some(path) = &cli.platform_jwks {
        let raw = std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("error: cannot read --platform-jwks {}: {e}", path.display());
            std::process::exit(2);
        });
        let jwks: PlatformJwks = serde_json::from_str(&raw).unwrap_or_else(|e| {
            eprintln!("error: --platform-jwks {} is not a JWKS JSON: {e}", path.display());
            std::process::exit(2);
        });
        return Some(jwks);
    }
    if let Some(url) = &cli.jwks_url {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let jwks = rt.block_on(async {
            let resp = reqwest::get(url).await.map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()));
            }
            resp.json::<PlatformJwks>().await.map_err(|e| e.to_string())
        });
        match jwks {
            Ok(j) => return Some(j),
            Err(e) => {
                eprintln!("error: --jwks-url {url} fetch failed: {e}");
                std::process::exit(2);
            }
        }
    }
    None
}

fn main() {
    let cli = Cli::parse();

    let json = match std::fs::read_to_string(&cli.certificate) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", cli.certificate.display());
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

    let jwks = load_jwks(&cli);
    let result = verify_certificate_with_platform(
        &cert,
        cli.artifact_dir.as_deref(),
        jwks.as_ref(),
        cli.require_sealed,
    );

    let repo_root = cli
        .repo_root
        .clone()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let warnings = validate_spec_id_resolution(&cert, &repo_root);
    match write_validation_warnings(&warnings, &cli.certificate) {
        Ok(Some(p)) => eprintln!(
            "validation warnings written: {} ({} warning(s))",
            p.display(),
            warnings.len()
        ),
        Ok(None) => {}
        Err(e) => eprintln!(
            "warning: failed to write validation warnings next to {}: {e}",
            cli.certificate.display()
        ),
    }

    for notice in &result.notices {
        eprintln!("notice: {notice}");
    }

    if result.valid {
        eprintln!(
            "governance certificate VERIFIED (pipeline: {}, status: {:?})",
            cert.pipeline_run_id, cert.status
        );
        eprintln!("  stages: {}", cert.stages.len());
        eprintln!("  proof chain records: {}", cert.proof_chain.record_count);
        eprintln!("  certificate hash: {}", &cert.certificate_hash[..16]);
        eprintln!(
            "  platform seal: {}",
            if cert.platform_countersign.is_some() {
                "present"
            } else {
                "ABSENT (unsealed)"
            }
        );
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
