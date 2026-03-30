//! FR-009 / FR-010 / NF-004 / SC-009 — append-only proof records, hash chain, independent verification.

use crate::canonical_json_sorted;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Serialized `decision` field on a proof record (`allow` | `deny` | `degrade`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProofRecordDecision {
    Allow,
    Deny,
    Degrade,
}

/// Serialized `privilege_level` field (`full` | `restricted` | `read-only` | `suspended`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProofPrivilege {
    Full,
    Restricted,
    #[serde(rename = "read-only")]
    ReadOnly,
    Suspended,
}

/// One proof record (FR-009). `record_hash` hashes canonical JSON of this object **without** `record_hash`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofRecord {
    pub id: String,
    pub timestamp: String,
    pub policy_bundle_hash: String,
    pub rule_ids: Vec<String>,
    pub input_context_hash: String,
    pub decision: ProofRecordDecision,
    pub privilege_level: ProofPrivilege,
    pub previous_record_hash: String,
    pub record_hash: String,
}

/// NF-004: per-record payload budget excluding the variable-sized context hash field body.
pub const NF004_MAX_BYTES_EXCLUDING_CONTEXT: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofChainError {
    EmptyChain,
    FirstPreviousNotBundleHash,
    PolicyBundleHashMismatch { index: usize },
    RecordHashMismatch { index: usize },
    BrokenLink { index: usize },
    Nf004Exceeded { index: usize, bytes: usize },
}

impl std::fmt::Display for ProofChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyChain => write!(f, "empty chain"),
            Self::FirstPreviousNotBundleHash => write!(
                f,
                "first record previous_record_hash does not match policy bundle hash"
            ),
            Self::PolicyBundleHashMismatch { index } => {
                write!(f, "policy_bundle_hash mismatch at index {index}")
            }
            Self::RecordHashMismatch { index } => write!(f, "record_hash mismatch at index {index}"),
            Self::BrokenLink { index } => write!(f, "broken chain link at index {index}"),
            Self::Nf004Exceeded { index, bytes } => {
                write!(f, "NF-004 size budget exceeded at index {index} ({bytes} bytes)")
            }
        }
    }
}

impl std::error::Error for ProofChainError {}

/// `record_hash = SHA-256(canonical_json(record without record_hash field))` per spec.
pub fn compute_record_hash(record: &ProofRecord) -> String {
    let mut v = serde_json::to_value(record).expect("proof record json");
    if let Value::Object(ref mut m) = v {
        m.remove("record_hash");
    }
    let s = canonical_json_sorted(v);
    sha256_hex(s.as_bytes())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

/// Approximate size of the fixed fields as JSON with `input_context_hash` emptied (NF-004).
pub fn nf004_payload_bytes(record: &ProofRecord) -> usize {
    let mut r = record.clone();
    r.input_context_hash.clear();
    serde_json::to_string(&r).expect("json").len()
}

/// Append-only writer: first record's `previous_record_hash` is the bundle hash (genesis link).
pub struct ProofChainWriter {
    bundle_hash: String,
    last_link_hash: String,
}

impl ProofChainWriter {
    pub fn new(bundle_hash: String) -> Self {
        Self {
            last_link_hash: bundle_hash.clone(),
            bundle_hash,
        }
    }

    pub fn last_link_hash(&self) -> &str {
        &self.last_link_hash
    }

    pub fn append(
        &mut self,
        id: String,
        timestamp: String,
        rule_ids: Vec<String>,
        input_context_hash: String,
        decision: ProofRecordDecision,
        privilege_level: ProofPrivilege,
    ) -> ProofRecord {
        let mut record = ProofRecord {
            id,
            timestamp,
            policy_bundle_hash: self.bundle_hash.clone(),
            rule_ids,
            input_context_hash,
            decision,
            privilege_level,
            previous_record_hash: self.last_link_hash.clone(),
            record_hash: String::new(),
        };
        record.record_hash = compute_record_hash(&record);
        self.last_link_hash.clone_from(&record.record_hash);
        record
    }
}

/// FR-010 / SC-009: verify chain integrity given the expected policy bundle content hash.
pub fn verify_proof_chain(records: &[ProofRecord], expected_bundle_hash: &str) -> Result<(), ProofChainError> {
    if records.is_empty() {
        return Err(ProofChainError::EmptyChain);
    }
    for (i, rec) in records.iter().enumerate() {
        if rec.policy_bundle_hash != expected_bundle_hash {
            return Err(ProofChainError::PolicyBundleHashMismatch { index: i });
        }
        let computed = compute_record_hash(rec);
        if computed != rec.record_hash {
            return Err(ProofChainError::RecordHashMismatch { index: i });
        }
        let n = nf004_payload_bytes(rec);
        if n > NF004_MAX_BYTES_EXCLUDING_CONTEXT {
            return Err(ProofChainError::Nf004Exceeded { index: i, bytes: n });
        }
        if i == 0 {
            if rec.previous_record_hash != expected_bundle_hash {
                return Err(ProofChainError::FirstPreviousNotBundleHash);
            }
        } else if rec.previous_record_hash != records[i - 1].record_hash {
            return Err(ProofChainError::BrokenLink { index: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bundle_hash() -> String {
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()
    }

    #[test]
    fn sc009_hundred_record_chain_verifies() {
        let bundle = sample_bundle_hash();
        let mut w = ProofChainWriter::new(bundle.clone());
        let mut chain = Vec::new();
        for i in 0..100 {
            let id = format!("{:08x}-0000-4000-8000-{:012x}", i, i);
            let rec = w.append(
                id,
                format!("2026-03-30T12:{:02}:00Z", i % 60),
                vec!["R-001".into()],
                format!("sha256:{:064x}", i),
                ProofRecordDecision::Allow,
                ProofPrivilege::Full,
            );
            chain.push(rec);
        }
        verify_proof_chain(&chain, &bundle).expect("chain should verify");
    }

    #[test]
    fn genesis_previous_is_bundle_hash() {
        let bundle = sample_bundle_hash();
        let mut w = ProofChainWriter::new(bundle.clone());
        let r = w.append(
            "00000000-0000-4000-8000-000000000001".into(),
            "2026-03-30T12:00:00Z".into(),
            vec!["R-1".into()],
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            ProofRecordDecision::Deny,
            ProofPrivilege::Restricted,
        );
        assert_eq!(r.previous_record_hash, bundle);
        verify_proof_chain(&[r], &bundle).unwrap();
    }

    #[test]
    fn broken_link_fails() {
        let bundle = sample_bundle_hash();
        let mut w = ProofChainWriter::new(bundle.clone());
        let a = w.append(
            "a".into(),
            "t".into(),
            vec![],
            "sha256:00".into(),
            ProofRecordDecision::Allow,
            ProofPrivilege::Full,
        );
        let mut b = w.append(
            "b".into(),
            "t".into(),
            vec![],
            "sha256:11".into(),
            ProofRecordDecision::Allow,
            ProofPrivilege::Full,
        );
        b.previous_record_hash = "sha256:deadbeef".into();
        b.record_hash = compute_record_hash(&b);
        let err = verify_proof_chain(&[a, b], &bundle).unwrap_err();
        assert!(matches!(err, ProofChainError::BrokenLink { index: 1 }));
    }

    #[test]
    fn nf004_small_record_ok() {
        let bundle = sample_bundle_hash();
        let mut w = ProofChainWriter::new(bundle.clone());
        let r = w.append(
            "id".into(),
            "2026-03-30T12:00:00Z".into(),
            vec!["R-1".into()],
            "sha256:cc".into(),
            ProofRecordDecision::Allow,
            ProofPrivilege::Full,
        );
        assert!(nf004_payload_bytes(&r) <= NF004_MAX_BYTES_EXCLUDING_CONTEXT);
    }
}
