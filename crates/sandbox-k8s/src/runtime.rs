// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/186-sandbox-k8s-backend/spec.md — §2.1, §3 FR-001

//! Runtime state for the K8s sandbox backend.
//!
//! Two-state enum: either a kube-rs `Client` is connected to a cluster
//! whose execution namespace exists and whose installed `RuntimeClass`
//! set has been probed, or it isn't. Phase 1 only constructs the
//! `Unavailable` arm — the kube-rs probe lands in Phase 3 alongside
//! `lifecycle::run`.
//!
//! Holding the `Connected` arm under `#[allow(dead_code)]` for Phase
//! 1 is deliberate: the `Unavailable → Connected` transition is the
//! sole shape Phase 3 needs to implement; defining the target state
//! up-front keeps the Phase 1 → Phase 3 diff small and bisect-safe.

#[allow(dead_code)]
pub(crate) enum RuntimeState {
    /// No usable kube-rs client. The contained diagnostic is surfaced
    /// verbatim through [`SandboxError::Unavailable`](
    /// factory_engine::sandbox::SandboxError::Unavailable) so operators
    /// see *why* the backend is unavailable (no kubeconfig, namespace
    /// absent, apiserver unreachable, etc.).
    Unavailable { diagnostic: String },

    /// A kube-rs client is connected, the execution namespace exists,
    /// and the installed `RuntimeClass` set has been probed. Phase 3
    /// will populate the contained fields; the enum variant is defined
    /// here so the `RuntimeState` shape is stable across phases.
    ///
    /// Held under `#[allow(dead_code)]` until Phase 3 wires the
    /// constructor + `lifecycle::run` consumer.
    Connected {
        /// Phase 3: `kube::Client`.
        ///
        /// Held as a unit struct rather than the real type to avoid
        /// pulling kube-rs into the Phase 1 surface area before
        /// `lifecycle::run` exists. The Phase 3 commit replaces this
        /// with the live client.
        client_marker: (),
    },
}
