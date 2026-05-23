// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/168-per-project-governance-certificate/spec.md
//
// Integration coverage for spec 168 §FR-001 / §FR-002 / §FR-003 / §FR-004
// / §FR-005 / §FR-006 / §FR-007 — the tenant-emit and tenant-verify
// surface, exercised through the published `build-certificate` and
// `verify-certificate` binaries.

use factory_engine::GovernanceCertificate;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin_path(name: &str) -> PathBuf {
    // CARGO_BIN_EXE_<name> is populated by cargo when the test target
    // sits in the same package as the bin target.
    let env_key = format!("CARGO_BIN_EXE_{name}");
    std::env::var_os(&env_key)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("env var {env_key} unset; cargo must run integration tests"))
}

fn write_stage_artifact(root: &Path, stage_id: &str, name: &str, body: &[u8]) {
    let stage_dir = root.join(stage_id);
    std::fs::create_dir_all(&stage_dir).unwrap();
    std::fs::write(stage_dir.join(name), body).unwrap();
}

fn read_certificate(path: &Path) -> GovernanceCertificate {
    let bytes = std::fs::read(path).expect("cert read");
    serde_json::from_slice(&bytes).expect("cert parse")
}

/// FR-002 + FR-003 + FR-007: tenant-mode requires signer flags. The
/// binary halts with a specific diagnostic and exit code 2 when the
/// flags are absent, rather than producing an unsigned certificate.
#[test]
fn tenant_mode_without_signer_halts_before_emission_fr007() {
    let tmp = tempfile::tempdir().unwrap();
    write_stage_artifact(tmp.path(), "tenant-codegen", "app.rs", b"fn main() {}");

    let output = Command::new(bin_path("build-certificate"))
        .arg(tmp.path())
        .arg("--tenant-mode")
        .output()
        .expect("spawn build-certificate");

    assert!(
        !output.status.success(),
        "tenant-mode without signer must NOT succeed"
    );
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("anonymous signing forbidden")
            || stderr.contains("FR-007"),
        "stderr did not surface FR-007 diagnostic: {stderr}"
    );
    assert!(
        !tmp.path().join("governance-certificate.json").exists(),
        "tenant-mode halt must NOT have written a certificate"
    );
}

/// FR-002 + FR-003: a tenant-mode run with signer flags writes a
/// certificate whose JSON carries the signer fields with the supplied
/// values.
#[test]
fn tenant_mode_with_signer_emits_signed_certificate_fr003() {
    let tmp = tempfile::tempdir().unwrap();
    write_stage_artifact(tmp.path(), "tenant-codegen", "app.rs", b"fn main() {}");
    write_stage_artifact(tmp.path(), "tenant-bundle", "bundle.tar", b"<bytes>");

    let output = Command::new(bin_path("build-certificate"))
        .arg(tmp.path())
        .args(["--tenant-mode"])
        .args(["--signer-subject", "alice@tenant.example"])
        .args(["--signer-identity-provider", "rauthy@tenant-org"])
        .args(["--signer-session-id", "sess-42"])
        .args(["--stage-ids", "tenant-codegen,tenant-bundle"])
        .output()
        .expect("spawn build-certificate");

    assert!(
        output.status.success(),
        "tenant-mode with signer must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let cert_path = tmp.path().join("governance-certificate.json");
    assert!(cert_path.exists(), "certificate file missing");

    let cert = read_certificate(&cert_path);
    let signer = cert.signer.as_ref().expect("signer field missing");
    assert_eq!(signer.subject, "alice@tenant.example");
    assert_eq!(signer.identity_provider, "rauthy@tenant-org");
    assert_eq!(signer.session_id.as_deref(), Some("sess-42"));
    assert_eq!(cert.stages.len(), 2);
    assert_eq!(cert.stages[0].stage_id, "tenant-codegen");
    assert_eq!(cert.stages[1].stage_id, "tenant-bundle");
}

/// FR-004 + FR-005: verify-certificate accepts a clean tenant cert
/// offline (no network calls; only the cert file + artifact dir on disk).
#[test]
fn verify_accepts_clean_tenant_certificate_fr004() {
    let tmp = tempfile::tempdir().unwrap();
    write_stage_artifact(tmp.path(), "tenant-codegen", "app.rs", b"fn main() {}");

    let build = Command::new(bin_path("build-certificate"))
        .arg(tmp.path())
        .args(["--tenant-mode"])
        .args(["--signer-subject", "bob@tenant.example"])
        .args(["--signer-identity-provider", "github-actions@tenant/repo"])
        .args(["--stage-ids", "tenant-codegen"])
        .output()
        .expect("spawn build-certificate");
    assert!(build.status.success(), "build failed: {}", String::from_utf8_lossy(&build.stderr));

    let cert_path = tmp.path().join("governance-certificate.json");
    let verify = Command::new(bin_path("verify-certificate"))
        .arg(&cert_path)
        .args(["--artifact-dir", tmp.path().to_str().unwrap()])
        .output()
        .expect("spawn verify-certificate");
    assert!(
        verify.status.success(),
        "verifier rejected clean cert; stderr: {}; stdout: {}",
        String::from_utf8_lossy(&verify.stderr),
        String::from_utf8_lossy(&verify.stdout),
    );
}

/// FR-006: tampering with any artifact file referenced by the
/// certificate causes the verifier to exit non-zero with a specific
/// artifact-hash-mismatch diagnostic.
#[test]
fn verify_rejects_tampered_tenant_artifact_fr006() {
    let tmp = tempfile::tempdir().unwrap();
    write_stage_artifact(tmp.path(), "tenant-codegen", "app.rs", b"fn main() {}");

    let build = Command::new(bin_path("build-certificate"))
        .arg(tmp.path())
        .args(["--tenant-mode"])
        .args(["--signer-subject", "carol@tenant.example"])
        .args(["--signer-identity-provider", "rauthy@tenant-org"])
        .args(["--stage-ids", "tenant-codegen"])
        .output()
        .expect("spawn build-certificate");
    assert!(build.status.success(), "build failed: {}", String::from_utf8_lossy(&build.stderr));

    // Tamper the artifact AFTER the cert was sealed.
    std::fs::write(tmp.path().join("tenant-codegen/app.rs"), b"fn evil(){}").unwrap();

    let cert_path = tmp.path().join("governance-certificate.json");
    let verify = Command::new(bin_path("verify-certificate"))
        .arg(&cert_path)
        .args(["--artifact-dir", tmp.path().to_str().unwrap()])
        .output()
        .expect("spawn verify-certificate");
    assert!(
        !verify.status.success(),
        "verifier accepted tampered artifact; stdout: {}",
        String::from_utf8_lossy(&verify.stdout)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr),
    );
    assert!(
        combined.contains("artifact hash mismatch")
            || combined.contains("hash mismatch"),
        "verifier output did not name the artifact hash mismatch: {combined}"
    );
}

/// FR-009: deterministic emission — the same inputs (modulo per-run
/// timestamp + ephemeral signing material, both of which are excluded
/// from artifact content) produce identical artifact hashes inside the
/// certificate's stage records.
#[test]
fn tenant_emission_is_artifact_hash_deterministic_fr009() {
    let tmp = tempfile::tempdir().unwrap();
    write_stage_artifact(tmp.path(), "tenant-codegen", "app.rs", b"fn main() {}");
    write_stage_artifact(tmp.path(), "tenant-codegen", "lib.rs", b"pub fn lib() {}");

    let invoke = || {
        let cert_dir = tempfile::tempdir().unwrap();
        let output = Command::new(bin_path("build-certificate"))
            .arg(tmp.path())
            .args(["--tenant-mode"])
            .args(["--signer-subject", "alice@tenant.example"])
            .args(["--signer-identity-provider", "rauthy@tenant-org"])
            .args(["--stage-ids", "tenant-codegen"])
            .args(["--out", cert_dir.path().join("c.json").to_str().unwrap()])
            .output()
            .expect("spawn build-certificate");
        assert!(output.status.success());
        let cert = read_certificate(&cert_dir.path().join("governance-certificate.json"));
        // tempdir lives until we move the cert out
        std::mem::forget(cert_dir);
        cert
    };

    let a = invoke();
    let b = invoke();
    assert_eq!(a.stages.len(), b.stages.len());
    for (sa, sb) in a.stages.iter().zip(b.stages.iter()) {
        assert_eq!(sa.stage_id, sb.stage_id);
        assert_eq!(sa.artifact_hashes, sb.artifact_hashes);
    }
}

/// Defensive: partial signer flags exit 2 with a specific diagnostic so
/// a misconfigured CI invocation does not silently fall through to an
/// unsigned cert (FR-007's intent at the CLI surface).
#[test]
fn partial_signer_flags_are_rejected_explicitly() {
    let tmp = tempfile::tempdir().unwrap();
    write_stage_artifact(tmp.path(), "tenant-codegen", "app.rs", b"fn main() {}");

    let output = Command::new(bin_path("build-certificate"))
        .arg(tmp.path())
        .args(["--signer-subject", "alice@tenant.example"])
        .output()
        .expect("spawn build-certificate");
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("requires --signer-identity-provider"),
        "stderr did not name the partial-flag requirement: {stderr}"
    );
}
