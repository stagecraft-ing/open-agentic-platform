// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: MCP_SNAPSHOT_WORKSPACE
// Spec: spec/core/snapshot-workspace.md

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize}; // Kept because Fingerprint::to_canonical_json still uses it
use sha2::{Digest, Sha256}; // Kept because Fingerprint::compute still uses it
use std::collections::{HashMap, HashSet}; // HashMap and HashSet are still used
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, RwLock}; // Kept because LeaseStore uses it
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)] // Added Eq, kept Serialize/Deserialize for to_canonical_json
pub struct Fingerprint {
    pub head_oid: String,
    pub index_oid: String,
    pub status_hash: String,
}

impl Fingerprint {
    pub fn compute(repo_root: &Path) -> Result<Self> {
        // 1. head_oid
        let head_output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_root)
            .output()?;

        let head_oid = if head_output.status.success() {
            String::from_utf8_lossy(&head_output.stdout)
                .trim()
                .to_string()
        } else {
            // Unborn branch or empty repo?
            // rev-parse HEAD passes even if unborn? No, usually fails.
            // Check if symbolic-ref HEAD exists?
            // Fallback for unborn: empty string.
            // We can treat failure as unborn for now if verifying it's a git repo.
            // Assume it is a git repo.
            "".to_string()
        };

        // 2. index_oid
        let write_tree_output = Command::new("git")
            .arg("write-tree")
            .current_dir(repo_root)
            .output()?;

        let index_oid = if write_tree_output.status.success() {
            String::from_utf8_lossy(&write_tree_output.stdout)
                .trim()
                .to_string()
        } else {
            // "no tree possible" -> e.g. merge conflict state when index is invalid?
            // Spec: "Empty string only if a tree is provably impossible"
            "".to_string()
        };

        // 3. status_hash
        // git status --porcelain=v1 -z
        let status_output = Command::new("git")
            .args(["status", "--porcelain=v1", "-z"])
            .current_dir(repo_root)
            .output()?;

        if !status_output.status.success() {
            return Err(anyhow!("Failed to run git status"));
        }

        let status_hash = hex::encode(Sha256::digest(&status_output.stdout));

        Ok(Self {
            head_oid,
            index_oid,
            status_hash,
        })
    }

    /// Canonical JSON representation for snapshot ID derivation
    pub fn to_canonical_json(&self) -> Result<String> {
        let val = serde_json::to_value(self)?;
        // sort keys
        Ok(serde_json::to_string(&val)?)
    }
}

#[derive(Clone, Debug)]
pub struct Lease {
    pub id: String,
    pub fingerprint: Fingerprint,
    pub touched_files: HashSet<String>,
}

#[derive(Default, Clone)]
pub struct LeaseStore {
    leases: Arc<RwLock<HashMap<String, Lease>>>,
}

impl LeaseStore {
    pub fn new() -> Self {
        Self {
            leases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn issue(&self, fingerprint: Fingerprint) -> String {
        let id = Uuid::new_v4().to_string();
        let lease = Lease {
            id: id.clone(),
            fingerprint,
            touched_files: HashSet::new(),
        };
        self.leases.write().unwrap().insert(id.clone(), lease);
        id
    }

    pub fn get_fingerprint(&self, lease_id: &str) -> Option<Fingerprint> {
        self.leases
            .read()
            .unwrap()
            .get(lease_id)
            .map(|l| l.fingerprint.clone())
    }

    pub fn touch_files(&self, lease_id: &str, files: Vec<String>) {
        let mut leases = self.leases.write().unwrap();
        if let Some(lease) = leases.get_mut(lease_id) {
            for f in files {
                lease.touched_files.insert(f);
            }
        }
    }

    pub fn get_touched_files(&self, lease_id: &str) -> Option<Vec<String>> {
        let leases = self.leases.read().unwrap();
        leases.get(lease_id).map(|l| {
            let mut v: Vec<String> = l.touched_files.iter().cloned().collect();
            v.sort(); // Lexicographic order
            v
        })
    }

    /// Verifies lease against current repo state.
    /// Returns Ok(()) if valid.
    /// Returns Err(STALE_LEASE) if mismatch.
    pub fn check_lease(&self, lease_id: &str, repo_root: &Path) -> Result<()> {
        let recorded_fp = self
            .get_fingerprint(lease_id)
            .ok_or_else(|| anyhow!("Lease not found: {}", lease_id))?; // Or separate error? "Lease not found" is INVALID_ARGUMENT or NOT_FOUND
        // Spec says "missing lease" logic issues new one, but if *passed* lease is invalid?
        // "Validation: Every worktree-mode request with a lease_id validates it..."

        let current_fp = Fingerprint::compute(repo_root)?;

        if recorded_fp != current_fp {
            // Construct STALE_LEASE error JSON
            // We use anyhow context or a specific error type?
            // The tool implementation layer usually handles mapping Result to JSON-RPC error.
            // But we need to pass the current_fp out.
            // We'll return a specific error that contains the details.

            // For now, return a formatted error string that the caller can parse or wrap?
            // Better: use a custom error type or just return serde_json::Value as error?
            // anyhow::Error is generic.
            // We can return an error that *downcasts* to a StaleLeaseError.

            return Err(StaleLeaseError {
                lease_id: lease_id.to_string(),
                current_fingerprint: current_fp,
                msg: "Lease is stale (repo changed)".into(),
            }
            .into());
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct StaleLeaseError {
    pub lease_id: String,
    pub current_fingerprint: Fingerprint,
    pub msg: String,
}

impl std::fmt::Display for StaleLeaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for StaleLeaseError {}
