// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/186-sandbox-k8s-backend/spec.md

//! K8s sandbox backend (spec 186).
//!
//! Concrete implementation of the [`SandboxClient`] contract defined in
//! spec 162. Targets the cluster execution surface (Surface B in spec
//! 162 §1): kube-rs against the operator's cluster, per-execution Pod +
//! NetworkPolicy synthesis, RuntimeClass-driven isolation tier
//! selection.
//!
//! ## Phase boundary
//!
//! - **Phase 1** (this phase) — scaffold + admission rules (FR-A1,
//!   FR-A2). The client is universally [`SandboxError::Unavailable`]
//!   because the kube-rs probe is wired in Phase 3. Admission rules
//!   that don't depend on cluster state run pre-probe and short-circuit
//!   common misconfigurations without needing a cluster.
//! - Phase 2 — pure builders (`pod_spec`, `network_policy`,
//!   `runtime_class`, `descriptor`).
//! - Phase 3 — kube-rs probe + `lifecycle::run` execute() path.

mod admission;
mod runtime;

use async_trait::async_trait;

use factory_contracts::sandbox::{SandboxExecution, SandboxRequest};
use factory_engine::sandbox::{BackendDescriptor, SandboxClient, SandboxError};

use runtime::RuntimeState;

/// K8s sandbox backend.
///
/// See spec 186 §2 for the design and §3 for the per-FR backend
/// behaviour. Construct via [`K8sSandboxClient::new`] for the default
/// kube-rs `Client::try_default` probe sequence + default execution
/// namespace, or via [`K8sSandboxClient::unavailable`] for tests and
/// for the documented Phase 1 always-unavailable posture.
pub struct K8sSandboxClient {
    runtime: RuntimeState,
}

impl K8sSandboxClient {
    /// Construct a client using kube-rs's default probe sequence
    /// (in-cluster → kubeconfig → none). Phase 1 always returns an
    /// [`Self::unavailable`] client — the kube-rs probe is wired in
    /// Phase 3 (FR-001). The Phase 1 client still honours the spec
    /// 186 §2.5 admission rules so misconfigured requests fail
    /// closed before any cluster work would be attempted.
    pub async fn new() -> Self {
        Self::unavailable(
            "spec 186 Phase 1: kube-rs client probe not yet wired \
            (see spec 186 §2.5 phase boundary); backend is fail-closed by default"
                .into(),
        )
    }

    /// Construct a backend pinned to [`SandboxError::Unavailable`] with
    /// a custom diagnostic. The diagnostic is surfaced verbatim in the
    /// returned error so operators see *why* the backend is unavailable.
    pub fn unavailable(diagnostic: String) -> Self {
        Self {
            runtime: RuntimeState::Unavailable { diagnostic },
        }
    }

    /// Reports whether a kube-rs client is currently connected. Phase
    /// 1 always returns `false`; Phase 3 wires the actual probe.
    pub fn is_available(&self) -> bool {
        matches!(self.runtime, RuntimeState::Connected { .. })
    }

    /// Default execution namespace name per spec 186 §2.1. Operators
    /// pre-create this namespace with PodSecurity `restricted`
    /// admission labels.
    pub const DEFAULT_NAMESPACE: &'static str = "oap-sandbox";
}

impl Default for K8sSandboxClient {
    fn default() -> Self {
        Self::unavailable(
            "K8sSandboxClient::default() — explicit unavailable (use ::new() to probe)".into(),
        )
    }
}

#[async_trait]
impl SandboxClient for K8sSandboxClient {
    async fn execute(
        &self,
        request: SandboxRequest,
    ) -> Result<SandboxExecution, SandboxError> {
        // Backend-specific admission first; FR-A1..A2 short-circuit
        // before the runtime is consulted so misconfigured requests
        // fail closed even when the cluster is unreachable.
        admission::check(&request)?;

        match &self.runtime {
            RuntimeState::Unavailable { diagnostic } => {
                Err(SandboxError::Unavailable(diagnostic.clone()))
            }
            RuntimeState::Connected { .. } => {
                // Phase 3 wires lifecycle::run here. Until then the
                // Connected arm is unreachable (Phase 1 never constructs
                // Connected). Marking explicit so the surface is honest:
                // the contract refuses host fallback by construction.
                Err(SandboxError::Unavailable(
                    "spec 186 Phase 1: Connected state reached but lifecycle::run \
                     is not yet wired (FR-001 deferred to Phase 3)"
                        .into(),
                ))
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

/// Backend identity per spec 186 §FR-002. Surfaced through
/// [`BackendDescriptor::name`].
pub const BACKEND_NAME: &str = "k8s";

#[cfg(test)]
mod tests {
    use super::*;
    use factory_contracts::sandbox::{
        EgressAllowlistEntry, InputArtifact, IsolationTier, ResourceCeilings,
        SandboxRequest, DEFAULT_PID_LIMIT, DEFAULT_TTL_SECONDS,
    };
    use std::collections::BTreeMap;

    fn baseline_request() -> SandboxRequest {
        SandboxRequest {
            command: vec!["echo".into(), "hello".into()],
            input_artifacts: vec![],
            egress_allowlist: vec![],
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
    async fn phase_1_client_is_always_unavailable() {
        let client = K8sSandboxClient::new().await;
        assert!(!client.is_available());
        let err = client.execute(baseline_request()).await.unwrap_err();
        match err {
            SandboxError::Unavailable(msg) => {
                assert!(msg.contains("spec 186"));
                assert!(msg.contains("Phase 1") || msg.contains("not yet wired"));
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unavailable_constructor_preserves_diagnostic() {
        let client = K8sSandboxClient::unavailable("cluster reset: dns".into());
        let err = client.execute(baseline_request()).await.unwrap_err();
        match err {
            SandboxError::Unavailable(msg) => assert_eq!(msg, "cluster reset: dns"),
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn backend_descriptor_reports_k8s() {
        let client = K8sSandboxClient::unavailable("test".into());
        let d = client.backend_descriptor();
        assert_eq!(d.name, BACKEND_NAME);
        assert_eq!(d.name, "k8s");
        assert!(!d.version.is_empty());
    }

    #[test]
    fn default_namespace_constant() {
        assert_eq!(K8sSandboxClient::DEFAULT_NAMESPACE, "oap-sandbox");
    }

    /// FR-A1 — non-empty egress allowlist rejected at admission BEFORE
    /// the runtime probe, even on a Phase 1 always-unavailable client.
    #[tokio::test]
    async fn fr_a1_egress_allowlist_rejected_pre_probe() {
        let client = K8sSandboxClient::unavailable("would-not-be-reached".into());
        let mut req = baseline_request();
        req.egress_allowlist.push(EgressAllowlistEntry {
            hostname: "registry.npmjs.org".into(),
        });
        let err = client.execute(req).await.unwrap_err();
        match err {
            SandboxError::AdmissionRejected(msg) => {
                assert!(msg.contains("FU-001"));
                assert!(msg.contains("registry.npmjs.org"));
            }
            other => panic!("expected AdmissionRejected, got {other:?}"),
        }
    }

    /// FR-A2 — non-empty input artifacts rejected at admission BEFORE
    /// the runtime probe.
    #[tokio::test]
    async fn fr_a2_input_artifacts_rejected_pre_probe() {
        let client = K8sSandboxClient::unavailable("would-not-be-reached".into());
        let mut req = baseline_request();
        req.input_artifacts.push(InputArtifact {
            path: "/in/main.rs".into(),
            sha256: "00".repeat(32),
        });
        let err = client.execute(req).await.unwrap_err();
        match err {
            SandboxError::AdmissionRejected(msg) => {
                assert!(msg.contains("FU-002"));
                assert!(msg.contains("/in/main.rs"));
            }
            other => panic!("expected AdmissionRejected, got {other:?}"),
        }
    }

    /// End-to-end with the spec 162 dispatcher — fail-closed posture
    /// honours FR-009 on the K8s backend the same way spec 185's
    /// peer test does on the local-container backend.
    #[tokio::test]
    async fn exercise_halts_on_unavailable() {
        let client = K8sSandboxClient::new().await;
        let err = factory_engine::sandbox::exercise(&client, baseline_request())
            .await
            .unwrap_err();
        match err {
            factory_engine::FactoryError::SandboxRefusal {
                category,
                diagnostic,
            } => {
                assert_eq!(category, "unavailable");
                assert!(diagnostic.contains("spec 186"));
            }
            other => panic!("expected SandboxRefusal::unavailable, got {other:?}"),
        }
    }
}
