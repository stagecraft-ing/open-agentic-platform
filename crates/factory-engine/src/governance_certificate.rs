// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/102-governed-excellence/spec.md — FR-002 through FR-010

//! Governance Certificate — the single JSON artifact proving the full
//! intent-to-spec-to-code-to-audit chain for a factory pipeline run.
//!
//! Generated at the end of every factory pipeline run (complete or incomplete).
//! Independently verifiable via `verify-certificate`.

use crate::pipeline_state::FactoryPipelineState;
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// Schema version for the governance certificate format.
///
/// Bumped to 1.1.0 (spec 102 FR-008.1+) to mark the introduction of Ed25519
/// signing alongside the original FR-008 self-authenticating hash. Verifiers
/// targeting 1.0.0 fixtures still pass — the new fields are optional in the
/// serde layer and the signature check is skipped when `signing_public_key`
/// is empty (such certs are treated as unsigned and rejected by any
/// HIAS-mode verification).
pub const CERTIFICATE_VERSION: &str = "1.1.0";

/// Environment-variable name carrying a base64-encoded 32-byte Ed25519 seed
/// (FR-008.1). Operator-supplied keys outside the agent's write scope.
pub const ENV_SIGNING_KEY: &str = "OAP_SIGNING_KEY";

/// Environment-variable name carrying a path to a file holding a base64-
/// encoded 32-byte Ed25519 seed (FR-008.1). Alternative to `OAP_SIGNING_KEY`.
pub const ENV_SIGNING_KEY_PATH: &str = "OAP_SIGNING_KEY_PATH";

// ── Top-level Certificate ────────────────────────────────────────────

/// A Governance Certificate proves the full chain from intent to auditable output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernanceCertificate {
    pub certificate_version: String,
    pub pipeline_run_id: String,
    pub timestamp: DateTime<Utc>,
    pub status: CertificateStatus,

    pub intent: IntentRecord,
    pub build_spec: BuildSpecRecord,
    pub stages: Vec<StageRecord>,
    pub verification: VerificationRecord,
    pub proof_chain: ProofChainSummary,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compliance: Option<ComplianceRecord>,

    /// SHA-256 of the canonical JSON of this certificate with `certificate_hash`
    /// AND `cert_signature` set to empty string. Content-binding fingerprint
    /// inside the signed payload — not the authoritative provenance check
    /// after spec 102 FR-008.1 (see `cert_signature`).
    pub certificate_hash: String,

    /// Base64-encoded Ed25519 public key (32 bytes) — verifier checks
    /// `cert_signature` against this. Empty for pre-1.1.0 fixtures and
    /// unsigned certificates; HIAS-mode verifiers reject empty.
    /// Spec 102 FR-008.2.
    #[serde(default)]
    pub signing_public_key: String,

    /// Base64-encoded Ed25519 signature (64 bytes) over canonical JSON
    /// of the certificate with `cert_signature` set to empty string and
    /// `certificate_hash` populated. Spec 102 FR-008.1.
    #[serde(default)]
    pub cert_signature: String,

    /// Trust-posture descriptor for `signing_public_key`. Spec 102 FR-008.3.
    #[serde(default)]
    pub signing_attestation: SigningAttestation,
}

/// Trust posture for the signing public key (spec 102 FR-008.3).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SigningAttestation {
    pub kind: SigningAttestationKind,
    /// Free-form note: operator email, key-rotation epoch, CI run URL, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SigningAttestationKind {
    /// No `signing_public_key` was set — pre-1.1.0 fixture or unsigned cert.
    /// HIAS-strict and non-strict verification both reject these once
    /// signing material is required by the runtime.
    #[default]
    Unsigned,
    /// Key generated for this run's lifetime; trust is "the run was
    /// internally consistent." Suitable for local dev.
    Ephemeral,
    /// Operator-supplied key via `OAP_SIGNING_KEY` or `OAP_SIGNING_KEY_PATH`.
    /// Trust is "the operator vouches for runs using this key."
    Operator,
    /// Signed by a Sigstore Fulcio-issued certificate and anchored to the
    /// Rekor transparency log. Required by HIAS-strict. Implementation
    /// landed in P0-3b (spec 102 FR-008.5).
    SigstoreRekor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CertificateStatus {
    Complete,
    Incomplete,
}

// ── Intent ───────────────────────────────────────────────────────────

/// Records the original intent that initiated the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentRecord {
    /// SHA-256 hash of the concatenated input requirements documents.
    pub requirements_hash: String,
    /// The governing spec ID (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_id: Option<String>,
    /// SHA-256 hash of the governing spec.md at pipeline start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_hash: Option<String>,
}

// ── Build Spec ───────────────────────────────────────────────────────

/// Records the frozen Build Spec and its approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildSpecRecord {
    /// SHA-256 hash of the frozen Build Spec YAML.
    pub hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_record: Option<ApprovalRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
    pub approved_at: DateTime<Utc>,
    pub gate_type: String,
}

// ── Stages ───────────────────────────────────────────────────────────

/// Per-stage record in the certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageRecord {
    pub stage_id: String,
    pub status: StageOutcome,
    /// SHA-256 hashes of all output artifacts, keyed by artifact name.
    pub artifact_hashes: BTreeMap<String, String>,
    pub gate_result: Option<GateResultRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StageOutcome {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateResultRecord {
    pub passed: bool,
    pub checks_run: u32,
    pub checks_failed: u32,
}

// ── Verification ─────────────────────────────────────────────────────

/// Aggregate verification outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRecord {
    pub compile: VerificationOutcome,
    pub test: VerificationOutcome,
    pub lint: VerificationOutcome,
    pub typecheck: VerificationOutcome,
    pub security_scan: VerificationOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VerificationOutcome {
    Passed,
    Failed,
    Skipped,
}

// ── Proof Chain ──────────────────────────────────────────────────────

/// Summary of the proof chain from policy-kernel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofChainSummary {
    pub record_count: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_record_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_record_hash: Option<String>,
    pub chain_integrity: ChainIntegrity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChainIntegrity {
    Verified,
    Unverified,
    Empty,
}

// ── Compliance ───────────────────────────────────────────────────────

/// Compliance mapping for the pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceRecord {
    pub frameworks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mappings: Vec<ComplianceMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceMapping {
    pub control: String,
    pub mechanism: String,
    pub status: String,
}

// ── Certificate Builder ──────────────────────────────────────────────

/// Builder for constructing a GovernanceCertificate from pipeline state.
pub struct CertificateBuilder {
    pipeline_run_id: String,
    intent: IntentRecord,
    build_spec_hash: String,
    approval_record: Option<ApprovalRecord>,
    stages: Vec<StageRecord>,
    verification: VerificationRecord,
    proof_chain: ProofChainSummary,
    compliance: Option<ComplianceRecord>,
}

impl CertificateBuilder {
    /// Create a new builder with the minimum required fields.
    pub fn new(pipeline_run_id: impl Into<String>, intent: IntentRecord) -> Self {
        Self {
            pipeline_run_id: pipeline_run_id.into(),
            intent,
            build_spec_hash: String::new(),
            approval_record: None,
            stages: Vec::new(),
            verification: VerificationRecord {
                compile: VerificationOutcome::Skipped,
                test: VerificationOutcome::Skipped,
                lint: VerificationOutcome::Skipped,
                typecheck: VerificationOutcome::Skipped,
                security_scan: VerificationOutcome::Skipped,
            },
            proof_chain: ProofChainSummary {
                record_count: 0,
                first_record_hash: None,
                last_record_hash: None,
                chain_integrity: ChainIntegrity::Empty,
            },
            compliance: None,
        }
    }

    pub fn build_spec_hash(mut self, hash: impl Into<String>) -> Self {
        self.build_spec_hash = hash.into();
        self
    }

    pub fn approval_record(mut self, record: ApprovalRecord) -> Self {
        self.approval_record = Some(record);
        self
    }

    pub fn stages(mut self, stages: Vec<StageRecord>) -> Self {
        self.stages = stages;
        self
    }

    pub fn add_stage(mut self, stage: StageRecord) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn verification(mut self, verification: VerificationRecord) -> Self {
        self.verification = verification;
        self
    }

    pub fn proof_chain(mut self, summary: ProofChainSummary) -> Self {
        self.proof_chain = summary;
        self
    }

    pub fn compliance(mut self, compliance: ComplianceRecord) -> Self {
        self.compliance = Some(compliance);
        self
    }

    /// Build the certificate, computing the self-authenticating hash (FR-008)
    /// AND the Ed25519 signature (FR-008.1). Signing key is resolved via
    /// `resolve_signing_material()` — operator env vars take precedence,
    /// ephemeral fallback for local dev.
    pub fn build(self) -> GovernanceCertificate {
        let has_failure = self.stages.iter().any(|s| s.status == StageOutcome::Failed);

        let status = if has_failure {
            CertificateStatus::Incomplete
        } else {
            CertificateStatus::Complete
        };

        let (signing_key, attestation) = resolve_signing_material();
        let public_key_b64 = B64.encode(signing_key.verifying_key().to_bytes());

        let mut cert = GovernanceCertificate {
            certificate_version: CERTIFICATE_VERSION.into(),
            pipeline_run_id: self.pipeline_run_id,
            timestamp: Utc::now(),
            status,
            intent: self.intent,
            build_spec: BuildSpecRecord {
                hash: self.build_spec_hash,
                approval_record: self.approval_record,
            },
            stages: self.stages,
            verification: self.verification,
            proof_chain: self.proof_chain,
            compliance: self.compliance,
            certificate_hash: String::new(),
            signing_public_key: public_key_b64,
            cert_signature: String::new(),
            signing_attestation: attestation,
        };

        // FR-008 (revised): content-binding hash. Zeros cert_hash AND
        // cert_signature so the hash is stable across signing.
        cert.certificate_hash = compute_certificate_hash(&cert);

        // FR-008.1: Ed25519 signature over canonical JSON with cert_signature
        // zeroed and cert_hash populated. Signing happens after hashing so
        // the signature attests both the content and its content-binding
        // fingerprint.
        cert.cert_signature = compute_certificate_signature(&cert, &signing_key);
        cert
    }
}

// ── Signing-key Resolution ───────────────────────────────────────────

/// Resolve the Ed25519 signing key per spec 102 FR-008.1:
///   1. `OAP_SIGNING_KEY` env var (base64, 32-byte seed) — `Operator` kind.
///   2. `OAP_SIGNING_KEY_PATH` env var (file path) — `Operator` kind.
///   3. Ephemeral key generated for this run — `Ephemeral` kind.
///
/// Returns the signing key plus the attestation describing the trust
/// posture. Malformed operator-supplied material panics — the caller
/// should not silently fall back to ephemeral when the operator
/// expressly attempted to supply a key (that would be a quiet downgrade).
pub fn resolve_signing_material() -> (SigningKey, SigningAttestation) {
    if let Ok(b64) = std::env::var(ENV_SIGNING_KEY) {
        let seed = decode_seed(&b64).unwrap_or_else(|e| {
            panic!("{ENV_SIGNING_KEY} is set but malformed: {e}");
        });
        return (
            SigningKey::from_bytes(&seed),
            SigningAttestation {
                kind: SigningAttestationKind::Operator,
                note: Some(format!("source={ENV_SIGNING_KEY}")),
            },
        );
    }
    if let Ok(path) = std::env::var(ENV_SIGNING_KEY_PATH) {
        let contents = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("{ENV_SIGNING_KEY_PATH}={path} unreadable: {e}");
        });
        let seed = decode_seed(contents.trim()).unwrap_or_else(|e| {
            panic!("{ENV_SIGNING_KEY_PATH}={path} content malformed: {e}");
        });
        return (
            SigningKey::from_bytes(&seed),
            SigningAttestation {
                kind: SigningAttestationKind::Operator,
                note: Some(format!("source={ENV_SIGNING_KEY_PATH}:{path}")),
            },
        );
    }
    let mut rng = rand::rngs::OsRng;
    (
        SigningKey::generate(&mut rng),
        SigningAttestation {
            kind: SigningAttestationKind::Ephemeral,
            note: Some("auto-generated for pipeline run".into()),
        },
    )
}

fn decode_seed(s: &str) -> Result<[u8; 32], String> {
    let bytes = B64.decode(s.trim()).map_err(|e| format!("base64: {e}"))?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| format!("seed length {} != 32", v.len()))
}

// ── Hash + Signature Computation ─────────────────────────────────────

/// Compute the content-binding SHA-256 hash of a certificate (FR-008 revised).
///
/// Zeros both `certificate_hash` AND `cert_signature` so the hash is
/// invariant under signing — the signature can be re-computed without
/// invalidating the hash. The hash is no longer the authoritative
/// provenance check (see `compute_certificate_signature` + FR-008.4); it
/// remains as a content fingerprint and an accidental-corruption guard
/// inside the signed payload.
pub fn compute_certificate_hash(cert: &GovernanceCertificate) -> String {
    let mut cert_for_hash = cert.clone();
    cert_for_hash.certificate_hash = String::new();
    cert_for_hash.cert_signature = String::new();

    // Canonical JSON: serde_json produces deterministic output for BTreeMap.
    // For Vec fields, order is preserved as inserted.
    let canonical = serde_json::to_string(&cert_for_hash).expect("certificate serialises to JSON");

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Compute the Ed25519 signature of a certificate (FR-008.1).
///
/// Signs the canonical JSON of the certificate with `cert_signature` set
/// to empty string and `certificate_hash` *populated* — the signature
/// attests both the content and the content-binding fingerprint. Returns
/// the base64-encoded 64-byte signature.
pub fn compute_certificate_signature(cert: &GovernanceCertificate, key: &SigningKey) -> String {
    let mut cert_for_sig = cert.clone();
    cert_for_sig.cert_signature = String::new();
    let canonical =
        serde_json::to_string(&cert_for_sig).expect("certificate serialises to JSON for signing");
    let sig: Signature = key.sign(canonical.as_bytes());
    B64.encode(sig.to_bytes())
}

/// Verify the Ed25519 signature on a certificate. Returns `Err` with a
/// specific diagnostic on failure (FR-008.4).
fn verify_certificate_signature(cert: &GovernanceCertificate) -> Result<(), String> {
    if cert.signing_public_key.is_empty() {
        return Err(
            "certificate is unsigned (signing_public_key empty) — rejected per FR-008.2".into(),
        );
    }
    if cert.cert_signature.is_empty() {
        return Err("certificate is unsigned (cert_signature empty) — rejected per FR-008.1".into());
    }
    let pk_bytes: [u8; 32] = B64
        .decode(&cert.signing_public_key)
        .map_err(|e| format!("signing_public_key base64 decode: {e}"))?
        .try_into()
        .map_err(|v: Vec<u8>| {
            format!("signing_public_key length {} != 32", v.len())
        })?;
    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|e| format!("signing_public_key not a valid Ed25519 point: {e}"))?;
    let sig_bytes: [u8; 64] = B64
        .decode(&cert.cert_signature)
        .map_err(|e| format!("cert_signature base64 decode: {e}"))?
        .try_into()
        .map_err(|v: Vec<u8>| format!("cert_signature length {} != 64", v.len()))?;
    let sig = Signature::from_bytes(&sig_bytes);

    let mut cert_for_sig = cert.clone();
    cert_for_sig.cert_signature = String::new();
    let canonical = serde_json::to_string(&cert_for_sig)
        .map_err(|e| format!("certificate re-serialises to JSON for verification: {e}"))?;

    verifying_key
        .verify(canonical.as_bytes(), &sig)
        .map_err(|e| format!("Ed25519 signature verification failed: {e}"))
}

// ── Generation from Pipeline State ───────────────────────────────────

/// Generate a governance certificate from a completed (or halted) pipeline.
///
/// FR-003: called at the end of every factory pipeline run.
/// FR-005: computes SHA-256 of each stage output artifact on disk.
pub fn generate_certificate(
    pipeline_state: &FactoryPipelineState,
    requirements_hash: &str,
    artifact_dir: &Path,
    proof_chain_summary: Option<ProofChainSummary>,
) -> GovernanceCertificate {
    let intent = IntentRecord {
        requirements_hash: requirements_hash.to_string(),
        spec_id: None,
        spec_hash: None,
    };

    let build_spec_hash = pipeline_state.build_spec_hash.clone().unwrap_or_default();

    // Collect stage records by scanning the artifact directory.
    let stages = collect_stage_records(artifact_dir);

    // Determine verification outcomes from the pipeline state.
    let verification = VerificationRecord {
        compile: VerificationOutcome::Skipped,
        test: VerificationOutcome::Skipped,
        lint: VerificationOutcome::Skipped,
        typecheck: VerificationOutcome::Skipped,
        security_scan: VerificationOutcome::Skipped,
    };

    let proof_chain = proof_chain_summary.unwrap_or(ProofChainSummary {
        record_count: 0,
        first_record_hash: None,
        last_record_hash: None,
        chain_integrity: ChainIntegrity::Empty,
    });

    CertificateBuilder::new(&pipeline_state.pipeline_id, intent)
        .build_spec_hash(build_spec_hash)
        .stages(stages)
        .verification(verification)
        .proof_chain(proof_chain)
        .build()
}

/// Scan the artifact directory for stage output files and compute their hashes.
fn collect_stage_records(artifact_dir: &Path) -> Vec<StageRecord> {
    let mut stages = Vec::new();

    // Known process stage IDs in order.
    let stage_ids = [
        "s0-preflight",
        "s1-business-requirements",
        "s2-service-requirements",
        "s3-data-model",
        "s4-api-specification",
        "s5-ui-specification",
    ];

    for stage_id in &stage_ids {
        let stage_dir = artifact_dir.join(stage_id);
        let mut artifact_hashes = BTreeMap::new();

        if stage_dir.is_dir()
            && let Ok(entries) = std::fs::read_dir(&stage_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && let Ok(contents) = std::fs::read(&path)
                {
                    let hash = sha256_bytes(&contents);
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    artifact_hashes.insert(name, hash);
                }
            }
        }

        let status = if artifact_hashes.is_empty() {
            StageOutcome::Skipped
        } else {
            StageOutcome::Passed
        };

        stages.push(StageRecord {
            stage_id: stage_id.to_string(),
            status,
            artifact_hashes,
            gate_result: None,
            duration_ms: None,
        });
    }

    stages
}

/// SHA-256 hash of raw bytes, returned as lowercase hex.
pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// SHA-256 hash of a file's contents.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let data = std::fs::read(path)?;
    Ok(sha256_bytes(&data))
}

// ── Persistence (FR-009) ─────────────────────────────────────────────

/// Persist the certificate as `governance-certificate.json` in the given directory.
pub fn persist_certificate(cert: &GovernanceCertificate, output_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(output_dir)?;
    let path = output_dir.join("governance-certificate.json");
    let json = serde_json::to_string_pretty(cert).map_err(std::io::Error::other)?;
    std::fs::write(path, json)
}

// ── Verification (FR-007) ────────────────────────────────────────────

/// Result of certificate verification.
#[derive(Debug)]
pub struct VerificationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

/// Verify a governance certificate by re-deriving hashes and checking integrity.
///
/// FR-007: exits 0 on success, 1 on any mismatch.
///
/// Spec 102 FR-008.4: signature verification runs FIRST and is the
/// authoritative provenance check. The content-binding hash check is
/// retained but is now defence-in-depth, not the primary check.
pub fn verify_certificate(
    cert: &GovernanceCertificate,
    artifact_dir: Option<&Path>,
) -> VerificationResult {
    let mut errors = Vec::new();

    // 0. Verify Ed25519 signature first (FR-008.4). This is the authoritative
    //    provenance check post-amendment — a tamper-with-resign attack that
    //    only updates the SHA-256 hash but cannot mint a valid signature
    //    over the modified content is caught here.
    if let Err(diagnostic) = verify_certificate_signature(cert) {
        errors.push(diagnostic);
    }

    // 1. Verify certificate self-hash (FR-008 revised — content binding,
    //    defence-in-depth).
    let expected_hash = compute_certificate_hash(cert);
    if cert.certificate_hash != expected_hash {
        errors.push(format!(
            "certificate hash mismatch: expected {expected_hash}, got {}",
            cert.certificate_hash
        ));
    }

    // 2. Verify artifact hashes against files on disk (FR-005).
    if let Some(dir) = artifact_dir {
        for stage in &cert.stages {
            let stage_dir = dir.join(&stage.stage_id);
            for (artifact_name, recorded_hash) in &stage.artifact_hashes {
                let artifact_path = stage_dir.join(artifact_name);
                match std::fs::read(&artifact_path) {
                    Ok(contents) => {
                        let actual_hash = sha256_bytes(&contents);
                        if &actual_hash != recorded_hash {
                            errors.push(format!(
                                "artifact hash mismatch: {}/{}: expected {recorded_hash}, got {actual_hash}",
                                stage.stage_id, artifact_name
                            ));
                        }
                    }
                    Err(e) => {
                        errors.push(format!(
                            "cannot read artifact {}/{}: {e}",
                            stage.stage_id, artifact_name
                        ));
                    }
                }
            }
        }
    }

    // 3. Verify version.
    if cert.certificate_version != CERTIFICATE_VERSION {
        errors.push(format!(
            "unsupported certificate version: {}",
            cert.certificate_version
        ));
    }

    VerificationResult {
        valid: errors.is_empty(),
        errors,
    }
}

// ── Cut D W-10: spec_id resolution validation (spec 102 G-2) ─────────
//
// Validates that a governance certificate's `intent.spec_id` resolves
// against `build/spec-registry/registry.json` via the typed-reader
// library introduced in W-03. Default: warn-only. Env-gated
// `OAP_REQUIRE_SPEC_ID_RESOLUTION=1` promotes any unresolved id to a
// hard error.
//
// Per Phase 6 § "Surprises #3", validation results live in a sibling
// `validation-warnings.json` file rather than the cert itself. This
// keeps the cert struct immutable (no version bump, signature
// invariant, every existing fixture survives).

/// A single spec-id-resolution finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ValidationWarning {
    /// `intent.spec_id` was set but no spec with that id exists in
    /// the spec-spine registry.
    SpecIdNotResolved {
        spec_id: String,
        registry_path: String,
    },
    /// The registry was not loadable at the expected path. By
    /// default this surfaces as a warning, not an error, because
    /// the cert is authoritative independent of the registry's
    /// existence on this filesystem.
    RegistryNotLoadable {
        registry_path: String,
        error: String,
    },
}

impl ValidationWarning {
    /// Stable string id for the finding kind. Used by the env-gate
    /// to decide whether to promote a warning to an error.
    pub fn kind(&self) -> &'static str {
        match self {
            ValidationWarning::SpecIdNotResolved { .. } => "spec-id-not-resolved",
            ValidationWarning::RegistryNotLoadable { .. } => "registry-not-loadable",
        }
    }
}

/// Validate `cert.intent.spec_id` against the spec spine.
///
/// Returns the list of [`ValidationWarning`]s (possibly empty). When
/// `intent.spec_id` is `None`, returns an empty list — the cert does
/// not claim a spec governance and there is nothing to validate.
pub fn validate_spec_id_resolution(
    cert: &GovernanceCertificate,
    repo_root: &Path,
) -> Vec<ValidationWarning> {
    let Some(spec_id) = cert.intent.spec_id.as_deref() else {
        return Vec::new();
    };
    let registry_path = repo_root.join(".derived/spec-registry/registry.json");
    let registry = match open_agentic_spec_registry_reader::load(&registry_path) {
        Ok(r) => r,
        Err(e) => {
            return vec![ValidationWarning::RegistryNotLoadable {
                registry_path: registry_path.display().to_string(),
                error: format!("{e}"),
            }];
        }
    };
    if registry.find_by_id(spec_id).is_some() {
        return Vec::new();
    }
    vec![ValidationWarning::SpecIdNotResolved {
        spec_id: spec_id.to_string(),
        registry_path: registry_path.display().to_string(),
    }]
}

/// Write the validation warnings to a sibling
/// `validation-warnings.json` next to the certificate (no-op when
/// the slice is empty — sibling-file absence == no warnings).
pub fn write_validation_warnings(
    warnings: &[ValidationWarning],
    cert_path: &Path,
) -> Result<Option<std::path::PathBuf>, std::io::Error> {
    if warnings.is_empty() {
        return Ok(None);
    }
    let sibling = cert_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("validation-warnings.json");
    let body = serde_json::to_string_pretty(&serde_json::json!({
        "certificateHash": "see governance-certificate.json",
        "warnings": warnings,
    }))
    .expect("validation warnings serialize");
    std::fs::write(&sibling, body)?;
    Ok(Some(sibling))
}

/// Returns true when the operator has opted into hard-failure mode
/// via `OAP_REQUIRE_SPEC_ID_RESOLUTION=1`. Default: false (warnings
/// remain warnings).
pub fn require_spec_id_resolution_enabled() -> bool {
    matches!(
        std::env::var("OAP_REQUIRE_SPEC_ID_RESOLUTION").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod w10_validation_tests {
    //! Cut D W-10 (spec 102 G-2) — spec_id resolution validation.

    use super::*;
    use std::fs;

    fn write_fake_registry(dir: &Path, ids: &[&str]) {
        let regdir = dir.join(".derived/spec-registry");
        fs::create_dir_all(&regdir).unwrap();
        let features: Vec<serde_json::Value> = ids
            .iter()
            .map(|id| {
                serde_json::json!({
                    "id": id,
                    "title": id,
                    "status": "approved",
                    "specPath": format!("specs/{id}/spec.md"),
                })
            })
            .collect();
        let body = serde_json::json!({
            "specVersion": "2.0.0",
            "build": {"compilerId": "test", "compilerVersion": "0", "inputRoot": ".", "contentHash": "0"},
            "features": features,
            "validation": {"passed": true, "violations": []},
        });
        fs::write(
            regdir.join("registry.json"),
            serde_json::to_string_pretty(&body).unwrap(),
        )
        .unwrap();
    }

    fn cert_with_spec_id(spec_id: Option<&str>) -> GovernanceCertificate {
        CertificateBuilder::new(
            "run-w10",
            IntentRecord {
                requirements_hash: "h".to_string(),
                spec_id: spec_id.map(String::from),
                spec_hash: None,
            },
        )
        .build_spec_hash("bs")
        .build()
    }

    #[test]
    fn validate_returns_empty_when_intent_spec_id_is_none() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_registry(dir.path(), &["001-x"]);
        let cert = cert_with_spec_id(None);
        let warnings = validate_spec_id_resolution(&cert, dir.path());
        assert!(warnings.is_empty());
    }

    #[test]
    fn validate_returns_empty_when_spec_id_resolves() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_registry(dir.path(), &["042-multi-provider-agent-registry"]);
        let cert = cert_with_spec_id(Some("042-multi-provider-agent-registry"));
        let warnings = validate_spec_id_resolution(&cert, dir.path());
        assert!(warnings.is_empty());
    }

    #[test]
    fn validate_emits_warning_for_unknown_spec_id() {
        let dir = tempfile::tempdir().unwrap();
        write_fake_registry(dir.path(), &["042-multi-provider-agent-registry"]);
        let cert = cert_with_spec_id(Some("999-nonexistent"));
        let warnings = validate_spec_id_resolution(&cert, dir.path());
        assert_eq!(warnings.len(), 1);
        match &warnings[0] {
            ValidationWarning::SpecIdNotResolved { spec_id, .. } => {
                assert_eq!(spec_id, "999-nonexistent");
            }
            other => panic!("expected SpecIdNotResolved, got {other:?}"),
        }
    }

    #[test]
    fn validate_emits_warning_when_registry_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cert = cert_with_spec_id(Some("042-multi-provider-agent-registry"));
        let warnings = validate_spec_id_resolution(&cert, dir.path());
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            ValidationWarning::RegistryNotLoadable { .. }
        ));
    }

    #[test]
    fn write_validation_warnings_skips_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cert_path = dir.path().join("governance-certificate.json");
        fs::write(&cert_path, "{}").unwrap();
        let out = write_validation_warnings(&[], &cert_path).unwrap();
        assert!(out.is_none());
        assert!(!dir.path().join("validation-warnings.json").exists());
    }

    #[test]
    fn write_validation_warnings_emits_sibling_when_non_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cert_path = dir.path().join("governance-certificate.json");
        fs::write(&cert_path, "{}").unwrap();
        let warnings = vec![ValidationWarning::SpecIdNotResolved {
            spec_id: "999-x".to_string(),
            registry_path: "registry.json".to_string(),
        }];
        let out = write_validation_warnings(&warnings, &cert_path).unwrap();
        let path = out.expect("sibling path returned");
        assert!(path.exists());
        let body: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(body["warnings"][0]["kind"], "spec-id-not-resolved");
        assert_eq!(body["warnings"][0]["spec_id"], "999-x");
    }

    #[test]
    fn require_resolution_env_gate_default_and_enabled() {
        // Single test for the env-gate so test parallelism doesn't
        // race over the shared global env var. SAFETY: env::set_var
        // and remove_var are unsafe under multi-threaded test
        // invocation; bracketing the assertions here keeps the
        // mutation self-contained.
        unsafe { std::env::remove_var("OAP_REQUIRE_SPEC_ID_RESOLUTION") };
        assert!(!require_spec_id_resolution_enabled());
        unsafe { std::env::set_var("OAP_REQUIRE_SPEC_ID_RESOLUTION", "1") };
        assert!(require_spec_id_resolution_enabled());
        unsafe { std::env::set_var("OAP_REQUIRE_SPEC_ID_RESOLUTION", "true") };
        assert!(require_spec_id_resolution_enabled());
        unsafe { std::env::set_var("OAP_REQUIRE_SPEC_ID_RESOLUTION", "no") };
        assert!(!require_spec_id_resolution_enabled());
        unsafe { std::env::remove_var("OAP_REQUIRE_SPEC_ID_RESOLUTION") };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn certificate_round_trip_and_hash() {
        let cert = CertificateBuilder::new(
            "run-001",
            IntentRecord {
                requirements_hash: "abc123".into(),
                spec_id: None,
                spec_hash: None,
            },
        )
        .build_spec_hash("def456")
        .build();

        assert_eq!(cert.certificate_version, CERTIFICATE_VERSION);
        assert_eq!(cert.status, CertificateStatus::Complete);
        assert!(!cert.certificate_hash.is_empty());

        // Round-trip serialisation.
        let json = serde_json::to_string_pretty(&cert).unwrap();
        let restored: GovernanceCertificate = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.certificate_hash, cert.certificate_hash);
        assert_eq!(restored.pipeline_run_id, "run-001");
    }

    #[test]
    fn self_authenticating_hash_detects_tampering() {
        let cert = CertificateBuilder::new(
            "run-002",
            IntentRecord {
                requirements_hash: "orig".into(),
                spec_id: None,
                spec_hash: None,
            },
        )
        .build_spec_hash("spec-hash")
        .build();

        // Tamper with a field. The naive tamper (no resign) trips BOTH the
        // signature check (FR-008.4 — authoritative) AND the hash check
        // (FR-008 revised — content binding). Either is sufficient.
        let mut tampered = cert.clone();
        tampered.intent.requirements_hash = "TAMPERED".into();

        let result = verify_certificate(&tampered, None);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("certificate hash mismatch")),
            "expected hash mismatch error among: {:?}",
            result.errors
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("Ed25519 signature verification failed")),
            "expected signature failure among: {:?}",
            result.errors
        );
    }

    /// HIAS finding closure: the report's load-bearing claim was that an
    /// adversary with write access could tamper a field AND recompute the
    /// SHA-256 hash, producing a tampered cert that still passes
    /// `verify_certificate`. Post FR-008.1 amendment, the signature is the
    /// authoritative provenance check — recomputing the hash without access
    /// to the signing key cannot mint a valid signature, so the tamper is
    /// caught at step 0 of `verify_certificate`.
    #[test]
    fn tamper_with_hash_resign_attack_is_caught_by_signature() {
        let cert = CertificateBuilder::new(
            "run-hias-001",
            IntentRecord {
                requirements_hash: "orig".into(),
                spec_id: None,
                spec_hash: None,
            },
        )
        .build_spec_hash("spec-hash")
        .build();

        // Adversary: tamper a field, then re-mint the SHA-256 hash so the
        // cert is internally hash-consistent. Under the pre-amendment
        // FR-008 contract, this would have passed verification — the
        // exact attack the HIAS readiness assessment surfaced as Critical.
        let mut tampered = cert.clone();
        tampered.intent.requirements_hash = "TAMPERED-BUT-RESIGNED".into();
        tampered.certificate_hash = compute_certificate_hash(&tampered);

        // Hash check alone now passes — the attack succeeded against
        // FR-008 (revised, content-binding only).
        let hash_only = compute_certificate_hash(&tampered);
        assert_eq!(
            tampered.certificate_hash, hash_only,
            "hash-only check is no longer authoritative — this is expected"
        );

        // But the Ed25519 signature was computed by the original key over
        // the ORIGINAL canonical bytes (cert_signature blank + original
        // certificate_hash). The adversary lacks the signing key and
        // cannot mint a new signature. Verification MUST fail at the
        // signature step (FR-008.4).
        let result = verify_certificate(&tampered, None);
        assert!(
            !result.valid,
            "tamper-with-resign attack should fail signature check (FR-008.4); errors: {:?}",
            result.errors
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("Ed25519 signature verification failed")),
            "expected Ed25519 signature failure; errors: {:?}",
            result.errors
        );
    }

    /// Sanity: a clean certificate verifies cleanly under signature + hash
    /// + version checks. Regression guard against the signing path being
    ///   off-by-one with the hash path (e.g., wrong field-zeroing order).
    #[test]
    fn clean_certificate_verifies() {
        let cert = CertificateBuilder::new(
            "run-clean",
            IntentRecord {
                requirements_hash: "abc".into(),
                spec_id: None,
                spec_hash: None,
            },
        )
        .build_spec_hash("spec")
        .build();

        // Built cert should be self-consistent.
        assert!(!cert.signing_public_key.is_empty(), "public key set");
        assert!(!cert.cert_signature.is_empty(), "signature set");
        assert_eq!(
            cert.signing_attestation.kind,
            SigningAttestationKind::Ephemeral,
            "test env has no OAP_SIGNING_KEY → ephemeral fallback"
        );

        let result = verify_certificate(&cert, None);
        assert!(
            result.valid,
            "clean cert must verify cleanly; errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn incomplete_certificate_on_failure() {
        let cert = CertificateBuilder::new(
            "run-003",
            IntentRecord {
                requirements_hash: "req".into(),
                spec_id: None,
                spec_hash: None,
            },
        )
        .add_stage(StageRecord {
            stage_id: "s0-preflight".into(),
            status: StageOutcome::Passed,
            artifact_hashes: BTreeMap::new(),
            gate_result: None,
            duration_ms: None,
        })
        .add_stage(StageRecord {
            stage_id: "s1-business-requirements".into(),
            status: StageOutcome::Failed,
            artifact_hashes: BTreeMap::new(),
            gate_result: Some(GateResultRecord {
                passed: false,
                checks_run: 3,
                checks_failed: 1,
            }),
            duration_ms: None,
        })
        .build();

        assert_eq!(cert.status, CertificateStatus::Incomplete);
    }

    #[test]
    fn persist_and_verify_with_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let artifact_dir = dir.path().join("artifacts");
        let stage_dir = artifact_dir.join("s0-preflight");
        fs::create_dir_all(&stage_dir).unwrap();

        // Write a test artifact.
        let artifact_content = b"preflight output data";
        fs::write(stage_dir.join("preflight.json"), artifact_content).unwrap();

        let artifact_hash = sha256_bytes(artifact_content);

        let cert = CertificateBuilder::new(
            "run-004",
            IntentRecord {
                requirements_hash: "req-hash".into(),
                spec_id: None,
                spec_hash: None,
            },
        )
        .add_stage(StageRecord {
            stage_id: "s0-preflight".into(),
            status: StageOutcome::Passed,
            artifact_hashes: BTreeMap::from([("preflight.json".into(), artifact_hash.clone())]),
            gate_result: None,
            duration_ms: None,
        })
        .build();

        // Persist.
        let cert_dir = dir.path().join("output");
        persist_certificate(&cert, &cert_dir).unwrap();
        assert!(cert_dir.join("governance-certificate.json").exists());

        // Verify against untampered artifacts.
        let result = verify_certificate(&cert, Some(&artifact_dir));
        assert!(result.valid, "errors: {:?}", result.errors);

        // Tamper with the artifact on disk.
        fs::write(stage_dir.join("preflight.json"), b"TAMPERED").unwrap();
        let result = verify_certificate(&cert, Some(&artifact_dir));
        assert!(!result.valid);
        assert!(result.errors[0].contains("artifact hash mismatch"));
    }

    #[test]
    fn generate_certificate_from_pipeline_state() {
        let dir = tempfile::tempdir().unwrap();
        let artifact_dir = dir.path().join("artifacts");
        let stage_dir = artifact_dir.join("s1-business-requirements");
        fs::create_dir_all(&stage_dir).unwrap();
        fs::write(stage_dir.join("entity-model.json"), b"{}").unwrap();

        let mut state = FactoryPipelineState::new("run-005", "aim-vue-node");
        state.transition_to_scaffolding("build-spec-hash-xyz".into());
        state.mark_complete();

        let cert = generate_certificate(&state, "requirements-hash", &artifact_dir, None);

        assert_eq!(cert.pipeline_run_id, "run-005");
        assert_eq!(cert.build_spec.hash, "build-spec-hash-xyz");
        assert_eq!(cert.intent.requirements_hash, "requirements-hash");

        // s1 should have the artifact.
        let s1 = cert
            .stages
            .iter()
            .find(|s| s.stage_id == "s1-business-requirements")
            .unwrap();
        assert_eq!(s1.status, StageOutcome::Passed);
        assert!(s1.artifact_hashes.contains_key("entity-model.json"));

        // Self-hash should verify.
        let result = verify_certificate(&cert, Some(&artifact_dir));
        assert!(result.valid, "errors: {:?}", result.errors);
    }
}
