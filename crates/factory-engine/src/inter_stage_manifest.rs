// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/170-signed-inter-stage-manifests/spec.md

//! Signed inter-stage manifests for the factory-engine two-phase pipeline.
//!
//! Every hand-off between stages (s0–s5 sequential, s6a–s6g fan-out) carries
//! a signed manifest identifying the producer, the artifacts handed over, and
//! a per-stage ephemeral-key signature anchored to a run-level root key
//! (spec 170 §2).
//!
//! The signing discipline is independent of transport. The same manifest
//! format covers in-process hand-offs, sandbox boundaries (spec 162), and
//! future distributed factory runs.

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

/// JSON shape of an inter-stage manifest (spec 170 §2).
///
/// `signature` is the base64-encoded Ed25519 signature over the canonical
/// JSON of the manifest with `signature` set to empty string. The signing
/// key is the dispatching stage's ephemeral key, derived from the run's
/// root key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InterStageManifest {
    pub run_id: String,
    pub from_stage: String,
    pub to_stage: String,
    pub produced_at: DateTime<Utc>,
    pub artifact_hashes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub signer: ManifestSigner,
    pub signature: String,
}

/// Identity of the agent that signed the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestSigner {
    /// Agent identifier (URI). Today this is `factory-engine/stage/<id>`;
    /// future distributed factory may carry a richer identity.
    pub agent_id: String,
    /// Fingerprint of the ephemeral key (SHA-256 of the public-key bytes,
    /// base64-encoded). The receiving stage resolves this against the run's
    /// key chain (§2.1).
    pub ephemeral_key_id: String,
}

/// Errors that can occur while building, signing, or verifying a manifest.
#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("manifest signature verification failed: {0}")]
    SignatureInvalid(String),
    #[error("signer's ephemeral_key_id {fingerprint} is not registered in run {run_id}'s key chain")]
    UnknownSigner { run_id: String, fingerprint: String },
    #[error(
        "manifest's run_id {manifest_run_id} does not match expected run {expected_run_id} (cross-run swap)"
    )]
    RunIdMismatch {
        expected_run_id: String,
        manifest_run_id: String,
    },
    #[error("manifest's to_stage {actual_to_stage} does not match expected receiver {expected_to_stage}")]
    StageMismatch {
        expected_to_stage: String,
        actual_to_stage: String,
    },
    #[error("invalid key material: {0}")]
    KeyMaterial(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("I/O error: {0}")]
    Io(String),
}

// ── Run key chain ────────────────────────────────────────────────────

/// Per-run key chain: the root verifying key plus the registry of stage
/// ephemeral keys established as the run progresses.
///
/// Persisted under the run directory so receiving stages can resolve a
/// manifest's `ephemeral_key_id` offline (FR-006). The signing keys
/// themselves never appear in the chain — only verifying material.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunKeyChain {
    pub run_id: String,
    /// Base64-encoded Ed25519 verifying key (32 bytes). Anchored in the
    /// run's governance certificate at run completion (spec 170 §2.1,
    /// spec 102 FR-007 composition).
    pub root_public_key_b64: String,
    pub stage_keys: BTreeMap<String, StageKeyRecord>,
}

/// One stage's registered ephemeral public key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StageKeyRecord {
    pub stage_id: String,
    /// Base64-encoded Ed25519 verifying key.
    pub ephemeral_public_key_b64: String,
    /// SHA-256 of the verifying-key bytes, base64-encoded. Stable identifier
    /// referenced by `ManifestSigner.ephemeral_key_id`.
    pub key_fingerprint: String,
}

impl RunKeyChain {
    /// Open a fresh chain with the given run-level root signing key. Only
    /// the verifying material is recorded — the signing key remains with
    /// the caller (typically s0/preflight).
    pub fn new_with_root(run_id: impl Into<String>, root_key: &SigningKey) -> Self {
        Self {
            run_id: run_id.into(),
            root_public_key_b64: B64.encode(root_key.verifying_key().to_bytes()),
            stage_keys: BTreeMap::new(),
        }
    }

    /// Mint an ephemeral signing key for a stage, deterministic in
    /// `(root_seed, stage_id)` so re-derivation is possible during
    /// retroactive audit. Returns the signing key (caller signs with it
    /// and discards per FR-005) and the verifying-only record to register.
    ///
    /// Derivation: `SHA-256(root_seed || b":stage:" || stage_id)` → 32-byte
    /// seed for `SigningKey::from_bytes`. This is a deterministic HKDF-like
    /// derivation; it is not domain-separated against arbitrary key uses
    /// because the chain itself binds the verifying material per stage.
    pub fn mint_ephemeral_for_stage(
        root_key: &SigningKey,
        stage_id: &str,
    ) -> (SigningKey, StageKeyRecord) {
        let mut hasher = Sha256::new();
        hasher.update(root_key.to_bytes());
        hasher.update(b":stage:");
        hasher.update(stage_id.as_bytes());
        let seed: [u8; 32] = hasher.finalize().into();
        let ephemeral = SigningKey::from_bytes(&seed);
        let pubkey_bytes = ephemeral.verifying_key().to_bytes();
        let pubkey_b64 = B64.encode(pubkey_bytes);
        let fingerprint = fingerprint_of_pubkey(&pubkey_bytes);
        let record = StageKeyRecord {
            stage_id: stage_id.to_string(),
            ephemeral_public_key_b64: pubkey_b64,
            key_fingerprint: fingerprint,
        };
        (ephemeral, record)
    }

    /// Register a stage's verifying material so downstream consumers can
    /// resolve signatures (idempotent — re-registering an identical record
    /// is a no-op; a conflicting record is an error).
    pub fn register_stage_key(&mut self, record: StageKeyRecord) -> Result<(), ManifestError> {
        if let Some(existing) = self.stage_keys.get(&record.stage_id)
            && existing != &record
        {
            return Err(ManifestError::KeyMaterial(format!(
                "conflicting key registration for stage {}: {} != {}",
                record.stage_id, existing.key_fingerprint, record.key_fingerprint
            )));
        }
        self.stage_keys.insert(record.stage_id.clone(), record);
        Ok(())
    }

    /// Resolve a fingerprint to a verifying key by linear scan of the
    /// registered stage keys. Returns the verifying key and the stage_id
    /// it belongs to.
    pub fn resolve_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<(VerifyingKey, &str), ManifestError> {
        for record in self.stage_keys.values() {
            if record.key_fingerprint == fingerprint {
                let bytes: [u8; 32] = B64
                    .decode(&record.ephemeral_public_key_b64)
                    .map_err(|e| ManifestError::KeyMaterial(format!("base64: {e}")))?
                    .try_into()
                    .map_err(|v: Vec<u8>| {
                        ManifestError::KeyMaterial(format!(
                            "ephemeral_public_key length {} != 32",
                            v.len()
                        ))
                    })?;
                let key = VerifyingKey::from_bytes(&bytes)
                    .map_err(|e| ManifestError::KeyMaterial(format!("not Ed25519: {e}")))?;
                return Ok((key, record.stage_id.as_str()));
            }
        }
        Err(ManifestError::UnknownSigner {
            run_id: self.run_id.clone(),
            fingerprint: fingerprint.to_string(),
        })
    }

    /// Save the chain to JSON at the given path. Creates parent directories.
    pub fn save_to_file(&self, path: &Path) -> Result<(), ManifestError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ManifestError::Io(format!("create_dir_all: {e}")))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ManifestError::Serialization(format!("{e}")))?;
        std::fs::write(path, json).map_err(|e| ManifestError::Io(format!("write: {e}")))?;
        Ok(())
    }

    /// Load the chain from JSON.
    pub fn load_from_file(path: &Path) -> Result<Self, ManifestError> {
        let json =
            std::fs::read_to_string(path).map_err(|e| ManifestError::Io(format!("read: {e}")))?;
        serde_json::from_str(&json).map_err(|e| ManifestError::Serialization(format!("{e}")))
    }
}

/// SHA-256 of an Ed25519 public-key payload, base64-encoded.
pub fn fingerprint_of_pubkey(pubkey_bytes: &[u8; 32]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(pubkey_bytes);
    B64.encode(hasher.finalize())
}

// ── Manifest signing / verifying ─────────────────────────────────────

/// Inputs to construct and sign a manifest in one shot.
pub struct ManifestSignInputs<'a> {
    pub run_id: &'a str,
    pub from_stage: &'a str,
    pub to_stage: &'a str,
    pub artifact_hashes: BTreeMap<String, String>,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub agent_id: &'a str,
    pub ephemeral_key_id: &'a str,
}

/// Build and sign a manifest. The caller supplies the dispatching stage's
/// ephemeral signing key; the resulting manifest carries the base64
/// signature over the manifest's canonical JSON (with the `signature`
/// field set to empty).
pub fn sign_manifest(
    inputs: ManifestSignInputs<'_>,
    ephemeral_signing_key: &SigningKey,
) -> InterStageManifest {
    let mut manifest = InterStageManifest {
        run_id: inputs.run_id.to_string(),
        from_stage: inputs.from_stage.to_string(),
        to_stage: inputs.to_stage.to_string(),
        produced_at: Utc::now(),
        artifact_hashes: inputs.artifact_hashes,
        metadata: inputs.metadata,
        signer: ManifestSigner {
            agent_id: inputs.agent_id.to_string(),
            ephemeral_key_id: inputs.ephemeral_key_id.to_string(),
        },
        signature: String::new(),
    };
    manifest.signature = compute_manifest_signature(&manifest, ephemeral_signing_key);
    manifest
}

/// Verify a manifest against the run's key chain.
///
/// Checks, in order:
///   1. `manifest.run_id == expected_run_id` (cross-run-swap guard, SC-003).
///   2. `manifest.to_stage == expected_to_stage` when supplied (the consumer
///      provides its own stage id so a manifest produced for a different
///      receiver doesn't pass).
///   3. The `ephemeral_key_id` resolves in the chain.
///   4. The Ed25519 signature verifies against the canonical bytes.
pub fn verify_manifest(
    manifest: &InterStageManifest,
    key_chain: &RunKeyChain,
    expected_to_stage: Option<&str>,
) -> Result<(), ManifestError> {
    if manifest.run_id != key_chain.run_id {
        return Err(ManifestError::RunIdMismatch {
            expected_run_id: key_chain.run_id.clone(),
            manifest_run_id: manifest.run_id.clone(),
        });
    }
    if let Some(expected) = expected_to_stage
        && manifest.to_stage != expected
    {
        return Err(ManifestError::StageMismatch {
            expected_to_stage: expected.to_string(),
            actual_to_stage: manifest.to_stage.clone(),
        });
    }
    let (verifying_key, _stage_id) =
        key_chain.resolve_fingerprint(&manifest.signer.ephemeral_key_id)?;

    let sig_bytes: [u8; 64] = B64
        .decode(&manifest.signature)
        .map_err(|e| ManifestError::SignatureInvalid(format!("base64: {e}")))?
        .try_into()
        .map_err(|v: Vec<u8>| {
            ManifestError::SignatureInvalid(format!("signature length {} != 64", v.len()))
        })?;
    let signature = Signature::from_bytes(&sig_bytes);

    let canonical = canonical_bytes_for_signing(manifest)?;
    verifying_key
        .verify(&canonical, &signature)
        .map_err(|e| ManifestError::SignatureInvalid(format!("Ed25519: {e}")))
}

/// Compute the signature bytes (base64) for a manifest. Signs the canonical
/// JSON with `signature` set to the empty string.
pub fn compute_manifest_signature(manifest: &InterStageManifest, key: &SigningKey) -> String {
    let canonical =
        canonical_bytes_for_signing(manifest).expect("manifest serialises to JSON for signing");
    let sig: Signature = key.sign(&canonical);
    B64.encode(sig.to_bytes())
}

fn canonical_bytes_for_signing(
    manifest: &InterStageManifest,
) -> Result<Vec<u8>, ManifestError> {
    let mut clone = manifest.clone();
    clone.signature = String::new();
    serde_json::to_vec(&clone)
        .map_err(|e| ManifestError::Serialization(format!("{e}")))
}

// ── Persistence helpers ──────────────────────────────────────────────

/// Persist a manifest under `run_dir/manifests/<from>-to-<to>.json`.
pub fn persist_manifest(
    manifest: &InterStageManifest,
    run_dir: &Path,
) -> Result<std::path::PathBuf, ManifestError> {
    let dir = run_dir.join("manifests");
    std::fs::create_dir_all(&dir).map_err(|e| ManifestError::Io(format!("create_dir_all: {e}")))?;
    let filename = format!("{}-to-{}.json", manifest.from_stage, manifest.to_stage);
    let path = dir.join(filename);
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| ManifestError::Serialization(format!("{e}")))?;
    std::fs::write(&path, json).map_err(|e| ManifestError::Io(format!("write: {e}")))?;
    Ok(path)
}

/// Load a manifest from a JSON file.
pub fn load_manifest(path: &Path) -> Result<InterStageManifest, ManifestError> {
    let json = std::fs::read_to_string(path).map_err(|e| ManifestError::Io(format!("read: {e}")))?;
    serde_json::from_str(&json).map_err(|e| ManifestError::Serialization(format!("{e}")))
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    fn fresh_root() -> SigningKey {
        let mut rng = OsRng;
        SigningKey::generate(&mut rng)
    }

    fn artifacts(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn roundtrip_sign_and_verify() {
        let root = fresh_root();
        let mut chain = RunKeyChain::new_with_root("run-1", &root);
        let (eph_key, record) = RunKeyChain::mint_ephemeral_for_stage(&root, "s0-preflight");
        chain.register_stage_key(record.clone()).unwrap();

        let manifest = sign_manifest(
            ManifestSignInputs {
                run_id: "run-1",
                from_stage: "s0-preflight",
                to_stage: "s1-business-requirements",
                artifact_hashes: artifacts(&[("preflight.json", "abc123")]),
                metadata: BTreeMap::new(),
                agent_id: "factory-engine/stage/s0-preflight",
                ephemeral_key_id: &record.key_fingerprint,
            },
            &eph_key,
        );

        verify_manifest(&manifest, &chain, Some("s1-business-requirements")).unwrap();
    }

    #[test]
    fn tamper_artifact_hash_breaks_signature() {
        let root = fresh_root();
        let mut chain = RunKeyChain::new_with_root("run-2", &root);
        let (eph_key, record) = RunKeyChain::mint_ephemeral_for_stage(&root, "s2-service");
        chain.register_stage_key(record.clone()).unwrap();

        let mut manifest = sign_manifest(
            ManifestSignInputs {
                run_id: "run-2",
                from_stage: "s2-service",
                to_stage: "s3-data-model",
                artifact_hashes: artifacts(&[("svc.json", "originalhash")]),
                metadata: BTreeMap::new(),
                agent_id: "factory-engine/stage/s2-service",
                ephemeral_key_id: &record.key_fingerprint,
            },
            &eph_key,
        );

        // Mutate after signing.
        manifest
            .artifact_hashes
            .insert("svc.json".into(), "tamperedhash".into());

        let err = verify_manifest(&manifest, &chain, None).unwrap_err();
        assert!(matches!(err, ManifestError::SignatureInvalid(_)), "got {err:?}");
    }

    #[test]
    fn cross_run_swap_is_rejected() {
        let root_a = fresh_root();
        let mut chain_a = RunKeyChain::new_with_root("run-A", &root_a);
        let (eph_a, record_a) = RunKeyChain::mint_ephemeral_for_stage(&root_a, "s0");
        chain_a.register_stage_key(record_a.clone()).unwrap();

        let manifest_a = sign_manifest(
            ManifestSignInputs {
                run_id: "run-A",
                from_stage: "s0",
                to_stage: "s1",
                artifact_hashes: artifacts(&[("x", "y")]),
                metadata: BTreeMap::new(),
                agent_id: "agent-A",
                ephemeral_key_id: &record_a.key_fingerprint,
            },
            &eph_a,
        );

        // Different run's chain — doesn't even know about run-A's key.
        let root_b = fresh_root();
        let chain_b = RunKeyChain::new_with_root("run-B", &root_b);

        let err = verify_manifest(&manifest_a, &chain_b, None).unwrap_err();
        // run_id mismatch wins before we even look up the fingerprint.
        assert!(matches!(err, ManifestError::RunIdMismatch { .. }), "got {err:?}");
    }

    #[test]
    fn unknown_signer_when_chain_omits_key() {
        let root = fresh_root();
        let chain = RunKeyChain::new_with_root("run-3", &root); // no register_stage_key
        let (eph_key, record) = RunKeyChain::mint_ephemeral_for_stage(&root, "s4");

        let manifest = sign_manifest(
            ManifestSignInputs {
                run_id: "run-3",
                from_stage: "s4",
                to_stage: "s5",
                artifact_hashes: artifacts(&[]),
                metadata: BTreeMap::new(),
                agent_id: "agent",
                ephemeral_key_id: &record.key_fingerprint,
            },
            &eph_key,
        );

        let err = verify_manifest(&manifest, &chain, None).unwrap_err();
        assert!(matches!(err, ManifestError::UnknownSigner { .. }), "got {err:?}");
    }

    #[test]
    fn stage_mismatch_is_rejected() {
        let root = fresh_root();
        let mut chain = RunKeyChain::new_with_root("run-4", &root);
        let (eph, record) = RunKeyChain::mint_ephemeral_for_stage(&root, "s2");
        chain.register_stage_key(record.clone()).unwrap();

        let manifest = sign_manifest(
            ManifestSignInputs {
                run_id: "run-4",
                from_stage: "s2",
                to_stage: "s3",
                artifact_hashes: artifacts(&[]),
                metadata: BTreeMap::new(),
                agent_id: "agent",
                ephemeral_key_id: &record.key_fingerprint,
            },
            &eph,
        );

        let err = verify_manifest(&manifest, &chain, Some("s4")).unwrap_err();
        assert!(matches!(err, ManifestError::StageMismatch { .. }), "got {err:?}");
    }

    #[test]
    fn deterministic_derivation_recovers_same_fingerprint() {
        let root = fresh_root();
        let (_k1, r1) = RunKeyChain::mint_ephemeral_for_stage(&root, "s6a-scaffold-init");
        let (_k2, r2) = RunKeyChain::mint_ephemeral_for_stage(&root, "s6a-scaffold-init");
        assert_eq!(r1.key_fingerprint, r2.key_fingerprint);
        assert_eq!(r1.ephemeral_public_key_b64, r2.ephemeral_public_key_b64);
    }

    #[test]
    fn fingerprints_differ_across_stages() {
        let root = fresh_root();
        let (_a, ra) = RunKeyChain::mint_ephemeral_for_stage(&root, "s5");
        let (_b, rb) = RunKeyChain::mint_ephemeral_for_stage(&root, "s6b-data-Org");
        assert_ne!(ra.key_fingerprint, rb.key_fingerprint);
    }

    #[test]
    fn chain_save_load_roundtrip() {
        let root = fresh_root();
        let mut chain = RunKeyChain::new_with_root("run-rt", &root);
        let (_, rec_a) = RunKeyChain::mint_ephemeral_for_stage(&root, "s0");
        let (_, rec_b) = RunKeyChain::mint_ephemeral_for_stage(&root, "s1");
        chain.register_stage_key(rec_a).unwrap();
        chain.register_stage_key(rec_b).unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("chain.json");
        chain.save_to_file(&path).unwrap();
        let restored = RunKeyChain::load_from_file(&path).unwrap();
        assert_eq!(restored, chain);
    }

    #[test]
    fn conflicting_register_is_rejected() {
        let root = fresh_root();
        let mut chain = RunKeyChain::new_with_root("run-c", &root);
        let (_, rec) = RunKeyChain::mint_ephemeral_for_stage(&root, "s0");
        chain.register_stage_key(rec.clone()).unwrap();
        // Second register with same key is idempotent.
        chain.register_stage_key(rec.clone()).unwrap();
        // A conflicting record (different fingerprint, same stage_id) fails.
        let bogus = StageKeyRecord {
            stage_id: rec.stage_id.clone(),
            ephemeral_public_key_b64: "AAAA".into(),
            key_fingerprint: "ZZZZ".into(),
        };
        assert!(chain.register_stage_key(bogus).is_err());
    }

    #[test]
    fn persist_manifest_writes_to_run_dir() {
        let root = fresh_root();
        let (eph, record) = RunKeyChain::mint_ephemeral_for_stage(&root, "s0");
        let manifest = sign_manifest(
            ManifestSignInputs {
                run_id: "run-p",
                from_stage: "s0",
                to_stage: "s1",
                artifact_hashes: artifacts(&[]),
                metadata: BTreeMap::new(),
                agent_id: "agent",
                ephemeral_key_id: &record.key_fingerprint,
            },
            &eph,
        );

        let tmp = tempfile::tempdir().unwrap();
        let path = persist_manifest(&manifest, tmp.path()).unwrap();
        assert!(path.exists());
        let loaded = load_manifest(&path).unwrap();
        assert_eq!(loaded, manifest);
    }
}
