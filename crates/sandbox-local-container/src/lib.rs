// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/185-sandbox-local-container-backend/spec.md

//! Local-container sandbox backend (spec 185).
//!
//! Concrete implementation of the [`SandboxClient`] contract defined in
//! spec 162. Targets the developer-workstation / OPC-laptop execution
//! surface: rootless Podman (preferred) or Docker via the Docker
//! Engine API.
//!
//! ## Phase 1 boundary
//!
//! This module currently lands the **scaffolding**: type surface,
//! trait wiring, and a universally `Unavailable` posture. Subsequent
//! phases land:
//!
//! 1. Runtime detection (Docker / Podman socket probe) and
//!    `backend_descriptor()` population.
//! 2. bollard-based container lifecycle (`execute()` happy path).
//! 3. Isolation hardening (read-only rootfs, drop caps, seccomp,
//!    no-new-privileges, non-root user).
//! 4. TTL + resource ceilings + resource peak polling + I/O artifact
//!    hashing.
//! 5. Tests (unit + env-gated integration).
//!
//! Every `execute()` call in this scaffolding returns
//! [`SandboxError::Unavailable`] with a diagnostic naming the phase
//! 1 state. This preserves spec 162's FR-009 invariant
//! (fail-closed-by-default) while the backend lights up.

use async_trait::async_trait;
use factory_contracts::sandbox::{SandboxExecution, SandboxRequest};
use factory_engine::sandbox::{
    BackendDescriptor, SandboxClient, SandboxError,
};

/// Local-container sandbox backend.
///
/// See spec 185 §2 for the design and §3 for the per-FR backend
/// behaviour. Construct via [`LocalContainerSandboxClient::new`].
pub struct LocalContainerSandboxClient {
    /// Phase 1 placeholder. Subsequent phases populate this with a
    /// resolved bollard client + detected runtime descriptor.
    runtime: RuntimeState,
}

/// Internal state describing the resolved runtime.
///
/// Phase 1 only carries `Unavailable`; phase 2 introduces the
/// `Connected { ... }` variant.
#[derive(Debug)]
enum RuntimeState {
    /// No reachable Docker-compatible socket. Every `execute()` call
    /// returns [`SandboxError::Unavailable`].
    Unavailable { diagnostic: String },
}

impl LocalContainerSandboxClient {
    /// Construct a new client. Phase 1 returns a client in the
    /// `Unavailable` state with a scaffolding diagnostic.
    ///
    /// Future phases will probe for a reachable Docker / Podman
    /// socket per spec 185 §2.1 and connect to it.
    pub fn new() -> Self {
        Self {
            runtime: RuntimeState::Unavailable {
                diagnostic: SCAFFOLDING_DIAGNOSTIC.into(),
            },
        }
    }
}

impl Default for LocalContainerSandboxClient {
    fn default() -> Self {
        Self::new()
    }
}

const SCAFFOLDING_DIAGNOSTIC: &str =
    "local-container backend is in phase 1 scaffolding (spec 185); runtime detection lands in a follow-up phase";

#[async_trait]
impl SandboxClient for LocalContainerSandboxClient {
    async fn execute(
        &self,
        _request: SandboxRequest,
    ) -> Result<SandboxExecution, SandboxError> {
        match &self.runtime {
            RuntimeState::Unavailable { diagnostic } => {
                Err(SandboxError::Unavailable(diagnostic.clone()))
            }
        }
    }

    fn backend_descriptor(&self) -> BackendDescriptor {
        BackendDescriptor {
            name: BACKEND_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Backend identity. Surfaces via [`BackendDescriptor::name`] (spec 185 FR-002).
pub const BACKEND_NAME: &str = "local-container";

#[cfg(test)]
mod tests {
    use super::*;
    use factory_contracts::sandbox::{
        EgressAllowlistEntry, IsolationTier, ResourceCeilings, SandboxRequest,
        DEFAULT_PID_LIMIT, DEFAULT_TTL_SECONDS,
    };
    use std::collections::BTreeMap;

    fn request() -> SandboxRequest {
        SandboxRequest {
            command: vec!["echo".into(), "hello".into()],
            input_artifacts: vec![],
            egress_allowlist: vec![EgressAllowlistEntry {
                hostname: "registry.npmjs.org".into(),
            }],
            ttl_seconds: DEFAULT_TTL_SECONDS,
            resource_ceilings: ResourceCeilings {
                cpu_milli_limit: 500,
                cpu_milli_request: 100,
                memory_bytes_limit: 256 * 1024 * 1024,
                memory_bytes_request: 64 * 1024 * 1024,
                pid_limit: DEFAULT_PID_LIMIT,
            },
            minimum_isolation_tier: IsolationTier::RestrictedContainer,
            env: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn scaffolding_returns_unavailable() {
        let client = LocalContainerSandboxClient::new();
        let err = client.execute(request()).await.unwrap_err();
        match err {
            SandboxError::Unavailable(msg) => {
                assert!(msg.contains("phase 1 scaffolding"));
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn backend_descriptor_reports_local_container() {
        let client = LocalContainerSandboxClient::new();
        let descriptor = client.backend_descriptor();
        assert_eq!(descriptor.name, BACKEND_NAME);
        assert_eq!(descriptor.name, "local-container");
        assert!(!descriptor.version.is_empty());
    }

    /// End-to-end with the spec 162 `exercise()` dispatcher — the
    /// scaffolding client honours FR-009 fail-closed-by-default.
    #[tokio::test]
    async fn exercise_halts_on_scaffolding_backend() {
        let client = LocalContainerSandboxClient::new();
        let err = factory_engine::sandbox::exercise(&client, request())
            .await
            .unwrap_err();
        match err {
            factory_engine::FactoryError::SandboxRefusal {
                category,
                diagnostic,
            } => {
                assert_eq!(category, "unavailable");
                assert!(diagnostic.contains("phase 1 scaffolding"));
            }
            other => panic!("expected SandboxRefusal::unavailable, got {other:?}"),
        }
    }
}
