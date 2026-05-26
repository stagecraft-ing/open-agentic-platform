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
//! ## Phase 2 boundary
//!
//! Phases landed:
//!
//! 1. Scaffolding — type surface, trait wiring, universally Unavailable.
//! 2. **Runtime detection** — probe DOCKER_HOST + rootless Podman +
//!    Docker default sockets; resolve to a `bollard::Docker` handle plus
//!    captured `Version`. (this phase)
//!
//! Phases pending: execute() lifecycle, isolation hardening, TTL +
//! resource ceilings + peak, tests.
//!
//! While runtime detection is wired, `execute()` still returns
//! `Unavailable` with a phase-state diagnostic. This preserves spec 162
//! FR-009 (fail-closed-by-default) while the backend lights up.

use async_trait::async_trait;
use bollard::Docker;
use bollard::system::Version;
use factory_contracts::sandbox::{SandboxExecution, SandboxRequest};
use factory_engine::sandbox::{
    BackendDescriptor, SandboxClient, SandboxError,
};
use std::path::{Path, PathBuf};

/// Local-container sandbox backend.
///
/// See spec 185 §2 for the design and §3 for the per-FR backend
/// behaviour. Construct via [`LocalContainerSandboxClient::new`] for
/// the default socket-probe sequence, or
/// [`LocalContainerSandboxClient::with_candidates`] for tests / custom
/// deployments.
pub struct LocalContainerSandboxClient {
    runtime: RuntimeState,
}

/// Internal state describing the resolved runtime.
#[derive(Debug)]
enum RuntimeState {
    /// No reachable Docker-compatible socket. Every `execute()` call
    /// returns [`SandboxError::Unavailable`] with the captured
    /// probe diagnostics.
    Unavailable { diagnostic: String },
    /// A socket probe succeeded and the Engine reports a version.
    /// Subsequent phases will use `docker` to drive the container
    /// lifecycle; the captured `version` populates `runtime_descriptor`.
    #[allow(dead_code)] // wired across phases — fields consumed in phase 3.
    Connected {
        docker: Docker,
        socket: PathBuf,
        runtime: DetectedRuntime,
        // Boxed to keep RuntimeState compact; Version carries ~360 bytes
        // of optional fields (Engine version string, components vec, etc.)
        // that the Unavailable variant does not need.
        version: Box<Version>,
    },
}

/// Runtime family detected at probe time.
///
/// Distinguished by inspecting `Version.platform.name` or the
/// `Components` block — Podman includes a `"Podman Engine"` component;
/// Docker includes `"Engine"` without that qualifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedRuntime {
    Docker,
    Podman,
}

impl DetectedRuntime {
    /// Lowercase identifier for diagnostics + `runtime_descriptor` JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            DetectedRuntime::Docker => "docker",
            DetectedRuntime::Podman => "podman",
        }
    }
}

/// A single probe failure, kept for diagnostic compositing.
#[derive(Debug, Clone)]
struct ProbeFailure {
    socket: PathBuf,
    reason: String,
}

impl LocalContainerSandboxClient {
    /// Construct a client using the default socket-probe sequence
    /// (spec 185 §2.1):
    ///
    /// 1. `DOCKER_HOST` env override.
    /// 2. Rootless Podman socket at
    ///    `${XDG_RUNTIME_DIR:-/run/user/$UID}/podman/podman.sock`.
    /// 3. Docker default socket at `/var/run/docker.sock`.
    pub async fn new() -> Self {
        Self::with_candidates(default_socket_candidates()).await
    }

    /// Construct a client by probing an explicit ordered list of socket
    /// candidates. Used by tests and custom deployments. The first
    /// candidate whose probe succeeds wins.
    pub async fn with_candidates(candidates: Vec<PathBuf>) -> Self {
        if candidates.is_empty() {
            return Self {
                runtime: RuntimeState::Unavailable {
                    diagnostic: "no socket candidates provided to LocalContainerSandboxClient::with_candidates".into(),
                },
            };
        }
        let mut failures: Vec<ProbeFailure> = Vec::new();
        for socket in candidates {
            match probe_socket(&socket).await {
                Ok((docker, version)) => {
                    let runtime = classify_runtime(&version);
                    return Self {
                        runtime: RuntimeState::Connected {
                            docker,
                            socket,
                            runtime,
                            version: Box::new(version),
                        },
                    };
                }
                Err(reason) => failures.push(ProbeFailure { socket, reason }),
            }
        }
        Self {
            runtime: RuntimeState::Unavailable {
                diagnostic: format_probe_failures(&failures),
            },
        }
    }

    /// Reports whether a runtime is currently connected. Useful for
    /// callers that want to dispatch differently when no runtime is
    /// available without waiting for the first `execute()` to fail.
    pub fn is_available(&self) -> bool {
        matches!(self.runtime, RuntimeState::Connected { .. })
    }

    /// Detected runtime family when connected, `None` otherwise.
    pub fn detected_runtime(&self) -> Option<DetectedRuntime> {
        match &self.runtime {
            RuntimeState::Connected { runtime, .. } => Some(*runtime),
            RuntimeState::Unavailable { .. } => None,
        }
    }
}

/// Default socket-probe sequence per spec 185 §2.1.
fn default_socket_candidates() -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    // 1. DOCKER_HOST env override (only honour unix:// forms here; tcp://
    //    URIs are an FU-005 follow-up — see spec 185 §5).
    if let Ok(host) = std::env::var("DOCKER_HOST")
        && let Some(stripped) = host.strip_prefix("unix://")
    {
        candidates.push(PathBuf::from(stripped));
    }
    // 2. Rootless Podman socket under XDG_RUNTIME_DIR.
    if let Some(podman) = rootless_podman_socket() {
        candidates.push(podman);
    }
    // 3. Docker default.
    candidates.push(PathBuf::from("/var/run/docker.sock"));
    candidates
}

fn rootless_podman_socket() -> Option<PathBuf> {
    let runtime_dir: PathBuf = match std::env::var("XDG_RUNTIME_DIR").ok() {
        Some(dir) => PathBuf::from(dir),
        None => fallback_runtime_dir()?,
    };
    Some(runtime_dir.join("podman").join("podman.sock"))
}

/// Systemd-convention `/run/user/<uid>` on Linux; `None` on macOS /
/// Windows (Podman Machine's socket is discoverable via
/// `podman system connection list`; surfacing that is FU-005).
#[cfg(target_os = "linux")]
fn fallback_runtime_dir() -> Option<PathBuf> {
    let uid = unsafe { libc_getuid() };
    Some(PathBuf::from(format!("/run/user/{uid}")))
}

#[cfg(not(target_os = "linux"))]
fn fallback_runtime_dir() -> Option<PathBuf> {
    None
}

#[cfg(target_os = "linux")]
extern "C" {
    #[link_name = "getuid"]
    fn libc_getuid() -> u32;
}

/// Probe a single Unix socket: connect, call `version()`, capture the
/// response.
async fn probe_socket(socket: &Path) -> Result<(Docker, Version), String> {
    if !socket.exists() {
        return Err(format!("socket {} does not exist", socket.display()));
    }
    let socket_str = socket
        .to_str()
        .ok_or_else(|| format!("socket path {} is not valid UTF-8", socket.display()))?;
    let docker = Docker::connect_with_unix(
        socket_str,
        DOCKER_API_TIMEOUT_SECS,
        bollard::API_DEFAULT_VERSION,
    )
    .map_err(|e| format!("bollard connect: {e}"))?;
    match docker.version().await {
        Ok(version) => Ok((docker, version)),
        Err(e) => Err(format!("/version probe failed: {e}")),
    }
}

const DOCKER_API_TIMEOUT_SECS: u64 = 4;

/// Distinguish Podman from Docker by inspecting `Version`.
///
/// Podman's `Components` list contains an entry whose `Name` starts
/// with `Podman Engine`. Docker's `Components` begin with `"Engine"`
/// without that qualifier.
fn classify_runtime(version: &Version) -> DetectedRuntime {
    if let Some(components) = version.components.as_ref()
        && components
            .iter()
            .any(|c| c.name.to_lowercase().contains("podman"))
    {
        return DetectedRuntime::Podman;
    }
    if let Some(platform) = version.platform.as_ref()
        && platform.name.to_lowercase().contains("podman")
    {
        return DetectedRuntime::Podman;
    }
    DetectedRuntime::Docker
}

fn format_probe_failures(failures: &[ProbeFailure]) -> String {
    if failures.is_empty() {
        return PHASE_STATE_DIAGNOSTIC.into();
    }
    let mut buf = String::from(
        "no reachable Docker-compatible socket (spec 185 §2.1 probe sequence); attempted: ",
    );
    for (i, f) in failures.iter().enumerate() {
        if i > 0 {
            buf.push_str(", ");
        }
        buf.push_str(&format!("{} ({})", f.socket.display(), f.reason));
    }
    buf
}

const PHASE_STATE_DIAGNOSTIC: &str =
    "local-container backend connected to runtime; execute() lands in a follow-up phase (spec 185)";

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
            RuntimeState::Connected { .. } => {
                Err(SandboxError::Unavailable(PHASE_STATE_DIAGNOSTIC.into()))
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
    use bollard::system::VersionComponents;
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
    async fn no_candidates_returns_unavailable_with_diagnostic() {
        let client = LocalContainerSandboxClient::with_candidates(vec![]).await;
        assert!(!client.is_available());
        assert_eq!(client.detected_runtime(), None);
        let err = client.execute(request()).await.unwrap_err();
        match err {
            SandboxError::Unavailable(msg) => {
                assert!(msg.contains("no socket candidates"));
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn nonexistent_paths_yield_compound_diagnostic() {
        let bogus = vec![
            PathBuf::from("/tmp/oap-test-nonexistent-sock-a"),
            PathBuf::from("/tmp/oap-test-nonexistent-sock-b"),
        ];
        let client = LocalContainerSandboxClient::with_candidates(bogus).await;
        assert!(!client.is_available());
        let err = client.execute(request()).await.unwrap_err();
        match err {
            SandboxError::Unavailable(msg) => {
                assert!(msg.contains("/tmp/oap-test-nonexistent-sock-a"));
                assert!(msg.contains("/tmp/oap-test-nonexistent-sock-b"));
                assert!(msg.contains("does not exist"));
                assert!(msg.contains("spec 185"));
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn socket_not_listening_yields_probe_failure() {
        // Create a regular file at a path — it "exists" but is not a
        // listening socket. The connect succeeds at the bollard layer
        // but the /version HTTP call fails.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let candidates = vec![tmp.path().to_path_buf()];
        let client = LocalContainerSandboxClient::with_candidates(candidates).await;
        assert!(!client.is_available());
        let err = client.execute(request()).await.unwrap_err();
        match err {
            SandboxError::Unavailable(msg) => {
                assert!(msg.contains("/version probe failed"));
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn backend_descriptor_reports_local_container() {
        // The runtime identity is conveyed via runtime_descriptor in
        // execute() outcomes (phase 3+), not via backend_descriptor.
        // backend_descriptor stays stable across phases.
        let client = LocalContainerSandboxClient {
            runtime: RuntimeState::Unavailable {
                diagnostic: "test".into(),
            },
        };
        let descriptor = client.backend_descriptor();
        assert_eq!(descriptor.name, BACKEND_NAME);
        assert_eq!(descriptor.name, "local-container");
        assert!(!descriptor.version.is_empty());
    }

    #[test]
    fn detected_runtime_as_str() {
        assert_eq!(DetectedRuntime::Docker.as_str(), "docker");
        assert_eq!(DetectedRuntime::Podman.as_str(), "podman");
    }

    #[test]
    fn classify_runtime_recognises_podman_component() {
        let version = Version {
            components: Some(vec![VersionComponents {
                name: "Podman Engine".into(),
                version: "4.9.0".into(),
                details: None,
            }]),
            ..Default::default()
        };
        assert_eq!(classify_runtime(&version), DetectedRuntime::Podman);
    }

    #[test]
    fn classify_runtime_defaults_to_docker() {
        let version = Version {
            components: Some(vec![VersionComponents {
                name: "Engine".into(),
                version: "28.0.0".into(),
                details: None,
            }]),
            ..Default::default()
        };
        assert_eq!(classify_runtime(&version), DetectedRuntime::Docker);
    }

    #[test]
    fn classify_runtime_recognises_podman_platform_name() {
        let version = Version {
            platform: Some(bollard::models::SystemVersionPlatform {
                name: "podman".into(),
            }),
            ..Default::default()
        };
        assert_eq!(classify_runtime(&version), DetectedRuntime::Podman);
    }

    /// End-to-end with the spec 162 `exercise()` dispatcher — the
    /// not-connected client still honours FR-009 fail-closed-by-default.
    #[tokio::test]
    async fn exercise_halts_on_unavailable() {
        let client = LocalContainerSandboxClient::with_candidates(vec![]).await;
        let err = factory_engine::sandbox::exercise(&client, request())
            .await
            .unwrap_err();
        match err {
            factory_engine::FactoryError::SandboxRefusal {
                category,
                diagnostic,
            } => {
                assert_eq!(category, "unavailable");
                assert!(diagnostic.contains("no socket candidates"));
            }
            other => panic!("expected SandboxRefusal::unavailable, got {other:?}"),
        }
    }
}
