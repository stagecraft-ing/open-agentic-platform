// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/167-born-with-spec-spine-kernel/spec.md

//! `.kernel-version` schema (spec 167 §2.3).
//!
//! The marker file is the load-bearing substrate for kernel-update
//! propagation. It records the source commit + content hash of OAP's
//! kernel at emission time, the factory-engine version that emitted it,
//! and the adapter identity + manifest hash. The propagation mechanism
//! itself is deferred to a follow-up spec (§5).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Top-level `.kernel-version` record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelVersion {
    pub kernel: KernelOrigin,
    pub adapter: AdapterIdentity,
    /// FR-005: chosen mode — `vendor-binaries` or `pinned-toolchain`.
    pub toolchain_mode: ToolchainMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelOrigin {
    /// OAP commit SHA the kernel was lifted from. Empty string when
    /// emission runs outside a git working tree (test fixtures).
    pub source_commit: String,
    /// Content hash over the gathered kernel files (gather.rs).
    pub source_hash: String,
    /// `factory-engine` semver at emission time. Sourced from CARGO_PKG_VERSION.
    pub factory_engine_version: String,
    /// ISO-8601 UTC timestamp. Excluded from determinism comparisons —
    /// callers verifying FR-009 must compare `source_hash` instead.
    pub emitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterIdentity {
    pub id: String,
    pub version: String,
    /// Content hash of the adapter manifest (or adapter-scopes entry) that
    /// drove this emission. Lets propagation distinguish kernels emitted
    /// under different adapter revisions.
    pub manifest_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolchainMode {
    /// Tenant-resident binaries shipped under `<project>/tools/spec-spine/`.
    VendorBinaries,
    /// Tenant CI references a pinned OAP toolchain distribution.
    PinnedToolchain,
}

impl KernelVersion {
    /// Serialise to canonical YAML for the `.kernel-version` file.
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Parse a `.kernel-version` YAML payload.
    pub fn from_yaml(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample() -> KernelVersion {
        KernelVersion {
            kernel: KernelOrigin {
                source_commit: "0e334041".into(),
                source_hash: "abc123".into(),
                factory_engine_version: "0.1.0".into(),
                emitted_at: Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap(),
            },
            adapter: AdapterIdentity {
                id: "aim-vue-node".into(),
                version: "0.1.0".into(),
                manifest_hash: "def456".into(),
            },
            toolchain_mode: ToolchainMode::VendorBinaries,
        }
    }

    #[test]
    fn round_trip_yaml() {
        let v = sample();
        let yaml = v.to_yaml().unwrap();
        let parsed = KernelVersion::from_yaml(&yaml).unwrap();
        assert_eq!(v, parsed);
    }

    #[test]
    fn toolchain_mode_serialises_kebab_case() {
        let yaml = serde_yaml::to_string(&ToolchainMode::VendorBinaries).unwrap();
        assert!(yaml.contains("vendor-binaries"));
        let yaml = serde_yaml::to_string(&ToolchainMode::PinnedToolchain).unwrap();
        assert!(yaml.contains("pinned-toolchain"));
    }
}
