// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/102-governed-excellence/spec.md — FR-002 through FR-010

//! Governance Certificate — the single JSON artifact proving the full
//! intent-to-spec-to-code-to-audit chain for a factory pipeline run.
//!
//! Generated at the end of every factory pipeline run (complete or incomplete).
//! Independently verifiable via `verify-certificate`.

use crate::pipeline_state::FactoryPipelineState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// Schema version for the governance certificate format.
pub const CERTIFICATE_VERSION: &str = "1.0.0";

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
    /// set to empty string. Any post-generation tampering is detectable.
    pub certificate_hash: String,
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

    /// Build the certificate, computing the self-authenticating hash (FR-008).
    pub fn build(self) -> GovernanceCertificate {
        let has_failure = self.stages.iter().any(|s| s.status == StageOutcome::Failed);

        let status = if has_failure {
            CertificateStatus::Incomplete
        } else {
            CertificateStatus::Complete
        };

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
        };

        // FR-008: self-authenticating hash.
        cert.certificate_hash = compute_certificate_hash(&cert);
        cert
    }
}

// ── Hash Computation ─────────────────────────────────────────────────

/// Compute the self-authenticating SHA-256 hash of a certificate.
///
/// Sets `certificateHash` to empty string, serialises to canonical JSON
/// (serde_json with BTreeMap keys already sorted), then hashes.
pub fn compute_certificate_hash(cert: &GovernanceCertificate) -> String {
    let mut cert_for_hash = cert.clone();
    cert_for_hash.certificate_hash = String::new();

    // Canonical JSON: serde_json produces deterministic output for BTreeMap.
    // For Vec fields, order is preserved as inserted.
    let canonical = serde_json::to_string(&cert_for_hash).expect("certificate serialises to JSON");

    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    format!("{:x}", hasher.finalize())
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
pub fn verify_certificate(
    cert: &GovernanceCertificate,
    artifact_dir: Option<&Path>,
) -> VerificationResult {
    let mut errors = Vec::new();

    // 1. Verify certificate self-hash (FR-008).
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

// ── Tests ────────────────────────────────────────────────────────────

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

        // Tamper with a field.
        let mut tampered = cert.clone();
        tampered.intent.requirements_hash = "TAMPERED".into();
        // Don't recompute the hash — the original hash should now be wrong.

        let result = verify_certificate(&tampered, None);
        assert!(!result.valid);
        assert!(result.errors[0].contains("certificate hash mismatch"));
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
