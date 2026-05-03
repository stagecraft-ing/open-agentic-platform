// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/124-opc-factory-run-platform-integration/spec.md
//   - §5 platform fetch replaces the spec-108 in-tree walk-up
//   - §6 reservation flow + duplex emit lifecycle
//   - T050..T055 OPC migration (Phase 5)

//! Bridge between the desktop's factory commands and `factory-platform-client`.
//!
//! Responsibilities of this module:
//!
//!   * Wrap [`StagecraftClient::auth_token`] as an [`OidcTokenProvider`] —
//!     the "no token" path surfaces as `MissingToken`, never as an empty
//!     bearer header.
//!   * Build a `(PlatformClient, AgentResolver, base_url, org_id)` quad
//!     from the desktop's app state in one place so the run-start and
//!     resume paths stay symmetric.
//!   * Resolve every stage's agent reference up-front so a retired binding
//!     fails BEFORE reservation (spec 124 §4.1) with structured info
//!     suitable for a deep-link UI message.
//!   * Reserve the run on the platform, materialise the cache root, and
//!     return a [`PreparedRun`] the caller can hand to `FactoryEngine`.
//!   * Emit `factory.run.*` envelopes through [`SyncClientInner`] and
//!     spool them to an on-disk replay queue when the duplex stream is
//!     disconnected (spec 124 §6 / T053).
//!   * Map [`FactoryClientError`] / [`ResolveError`] into a typed
//!     [`FactoryError`] whose `RetiredAgent` variant carries the data
//!     the UI needs to deep-link the user to the project's binding page.

use async_trait::async_trait;
use chrono::Utc;
use factory_engine::agent_resolver::{
    AgentReference as EngineAgentReference, AgentResolver, ResolveError,
};
use factory_platform_client::{
    walk_process_for_agent_refs, FactoryClientError, OidcTokenProvider, PlatformClient,
    ReserveRunRequest, RunReservation, RunRoot,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::io::AsyncBufReadExt;
use uuid::Uuid;

use super::stagecraft_client::{StagecraftClient, StagecraftState};
use super::sync_client::{
    FactoryAgentRef, FactoryRunTokenSpend, FactoryStageOutcome, SyncClientInner, SyncClientState,
};

// ---------------------------------------------------------------------------
// FactoryError — typed errors for the desktop factory command surface
// ---------------------------------------------------------------------------

/// Errors returned by the desktop's factory command path (spec 124 Phase 5).
///
/// `Display` for each variant is what the React UI shows to the user; the
/// `RetiredAgent` variant deliberately embeds the deep-link path so the UI
/// can render a clickable affordance without parsing structured fields.
#[derive(Debug, thiserror::Error)]
pub enum FactoryError {
    /// No `StagecraftClient` is loaded — the desktop is not signed in or
    /// has not picked an active workspace.
    #[error("not signed in to stagecraft (no active session)")]
    NoStagecraft,

    /// The active stagecraft session has no `org_id` populated.
    #[error("no active organization (sign-in incomplete)")]
    NoOrgId,

    /// The OIDC token provider returned no token. UX: prompt the user to
    /// sign in again.
    #[error("missing OIDC access token — please sign in again")]
    MissingToken,

    /// Spec 124 §4.1 / T055 — a project's agent binding points at a row
    /// that has been retired upstream. The UI deep-links the user to the
    /// project's binding management page (`/app/project/{id}/agents`).
    /// `agent_name` may be empty for `ById` references that don't carry a
    /// name in the process definition.
    #[error(
        "agent \"{agent_name}\" ({org_agent_id} v{version}) is retired upstream. \
         Open project bindings: /app/project/{}/agents",
        project_id.as_deref().unwrap_or("<ad-hoc>")
    )]
    RetiredAgent {
        agent_name: String,
        org_agent_id: String,
        version: i64,
        project_id: Option<String>,
    },

    /// Catch-all for other resolver errors (NotFound, AmbiguousName,
    /// VersionMismatch, transport).
    #[error("agent resolver error: {0}")]
    Resolver(String),

    /// `POST /api/factory/runs` failed with a non-RetiredAgent error.
    #[error("factory run reservation failed: {0}")]
    Reservation(String),

    /// Materialisation of the per-run cache root failed.
    #[error("failed to materialise run cache: {0}")]
    Materialisation(String),

    /// Underlying `factory-engine` start/dispatch error.
    #[error("factory engine error: {0}")]
    Engine(String),

    /// Local I/O error (replay queue, project path).
    #[error("io error: {0}")]
    Io(String),

    /// Any other unstructured failure.
    #[error("{0}")]
    Other(String),
}

impl FactoryError {
    /// Convert to a `String` suitable for a Tauri command return value.
    /// `Display` already includes the deep-link for `RetiredAgent`.
    pub fn into_user_message(self) -> String {
        self.to_string()
    }
}

impl From<FactoryClientError> for FactoryError {
    fn from(e: FactoryClientError) -> Self {
        match e {
            FactoryClientError::MissingToken => FactoryError::MissingToken,
            FactoryClientError::TokenProvider(m) => FactoryError::MissingToken
                .also(format!("token provider: {m}"))
                .unwrap_or(FactoryError::Other(format!("token provider: {m}"))),
            FactoryClientError::RetiredAgent(body) => {
                // Server-side path returns a 412 with body shaped like
                //   `agent "<name>" (<org_agent_id> v<version>) is retired upstream`
                // — parse it into the structured variant so the UI can
                // deep-link without string-sniffing.
                parse_retired_agent_body(&body).unwrap_or(FactoryError::Other(format!(
                    "retired agent: {body}"
                )))
            }
            FactoryClientError::AgentDrift(m) => FactoryError::Materialisation(format!(
                "source_shas / resolver drift: {m}"
            )),
            FactoryClientError::Resolver(m) => FactoryError::Resolver(m),
            FactoryClientError::CacheIo(m) => FactoryError::Materialisation(m),
            FactoryClientError::Network(m) => FactoryError::Reservation(format!("network: {m}")),
            FactoryClientError::NotFound(m) => FactoryError::Reservation(format!("not found: {m}")),
            FactoryClientError::Http { status, body } => {
                FactoryError::Reservation(format!("http {status}: {body}"))
            }
            FactoryClientError::Decode(m) => FactoryError::Reservation(format!("decode: {m}")),
        }
    }
}

impl FactoryError {
    /// No-op chain helper to keep the `From<FactoryClientError>` impl tidy.
    fn also(self, _: String) -> Option<FactoryError> {
        Some(self)
    }
}

/// Parse the platform-side `RetiredAgentError.message` body into a typed
/// variant. Format produced by `runAgentRefs.ts`:
///
/// ```text
/// agent "<name>" (<org_agent_id> v<version>) is retired upstream
/// ```
///
/// Returns `None` when the body does not match the expected shape; the
/// caller falls back to a generic [`FactoryError::Other`] in that case.
fn parse_retired_agent_body(body: &str) -> Option<FactoryError> {
    let after_agent = body.strip_prefix("agent \"")?;
    let (name, rest) = after_agent.split_once("\" (")?;
    let (id_and_v, _tail) = rest.split_once(")")?;
    let (org_agent_id, v_part) = id_and_v.split_once(' ')?;
    let version_str = v_part.strip_prefix('v')?;
    let version: i64 = version_str.parse().ok()?;
    Some(FactoryError::RetiredAgent {
        agent_name: name.to_string(),
        org_agent_id: org_agent_id.to_string(),
        version,
        project_id: None,
    })
}

// ---------------------------------------------------------------------------
// OIDC token provider — wraps StagecraftClient::auth_token()
// ---------------------------------------------------------------------------

/// Surfaces the desktop's persisted Rauthy JWT to `factory-platform-client`.
///
/// `StagecraftClient::auth_token()` returns `Option<String>`; this provider
/// converts an empty string to `Ok(None)` so the platform client's
/// `MissingToken` path fires uniformly. The platform client is responsible
/// for refresh policy — see spec 112 §6.3 in `stagecraft_client.rs`.
pub struct StagecraftOidcProvider(pub StagecraftClient);

#[async_trait]
impl OidcTokenProvider for StagecraftOidcProvider {
    async fn fetch_token(&self) -> Result<Option<String>, FactoryClientError> {
        Ok(self.0.auth_token().filter(|s| !s.is_empty()))
    }
}

// ---------------------------------------------------------------------------
// Platform context — bundle of everything a run start needs
// ---------------------------------------------------------------------------

/// Bundle of dependencies a run-start path needs to talk to the platform.
/// Constructed once via [`platform_context`] and threaded through the rest
/// of the pipeline.
pub struct PlatformContext {
    pub client: PlatformClient,
    pub resolver: AgentResolver,
    pub org_id: String,
    pub base_url: String,
}

/// Build a [`PlatformContext`] from the desktop's app state.
pub fn platform_context(app: &AppHandle) -> Result<PlatformContext, FactoryError> {
    let sc_state = app
        .try_state::<StagecraftState>()
        .ok_or(FactoryError::NoStagecraft)?;
    let sc = sc_state.current().ok_or(FactoryError::NoStagecraft)?;
    let org_id = sc.org_id();
    if org_id.is_empty() {
        return Err(FactoryError::NoOrgId);
    }
    let base_url = sc.base_url().to_string();
    let provider: Arc<dyn OidcTokenProvider> = Arc::new(StagecraftOidcProvider(sc.clone()));
    let client = PlatformClient::new(base_url.clone(), provider);
    let resolver = AgentResolver::new(org_id.clone(), Box::new(client.clone()));
    Ok(PlatformContext {
        client,
        resolver,
        org_id,
        base_url,
    })
}

// ---------------------------------------------------------------------------
// Stage walker — pairs each AgentReference with its owning stage_id
// ---------------------------------------------------------------------------

/// Per-stage agent reference extracted from the process body. The platform
/// client's `walk_process_for_agent_refs` returns a flat list in stage
/// order; this walker preserves the stage_id so the desktop can stamp each
/// `factory.run.stage_started` envelope with the correct `agent_ref`.
///
/// Currently only the `stages: [...]` shape is supported — that is the
/// shape the in-tree process YAML used (spec 108 §6) and the shape the
/// stagecraft `factory_processes.definition` mirrors. Unknown shapes
/// degrade to "no agent ref for this stage", which is non-fatal.
pub fn walk_stage_agents(
    process_definition: &serde_json::Value,
) -> Vec<(String, EngineAgentReference)> {
    let mut out = Vec::new();
    let Some(stages) = process_definition.get("stages").and_then(|v| v.as_array()) else {
        return out;
    };
    for stage in stages {
        let Some(stage_id) = stage.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(agent_ref) = stage.get("agent_ref") else {
            continue;
        };
        let refs = walk_process_for_agent_refs(agent_ref);
        if let Some(first) = refs.into_iter().next() {
            out.push((stage_id.to_string(), first));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// PreparedRun — the result of a successful platform-side reservation
// ---------------------------------------------------------------------------

/// Result of [`prepare_run_root`]. `engine_factory_root` is the cache path
/// `FactoryEngine::new` consumes via `FactoryEngineConfig.factory_root`;
/// `run_id` is the platform-issued UUID the desktop emits envelopes
/// against.
pub struct PreparedRun {
    pub run_id: String,
    pub run_root: RunRoot,
    pub engine_factory_root: PathBuf,
    /// `(stage_id, agent_ref)` pairs in stage order. The desktop stamps
    /// `factory.run.stage_started` with the matching entry by `stage_id`.
    pub stage_agents: Vec<(String, FactoryAgentRef)>,
    /// Same triples by index in `source_shas.agents[]` for downstream
    /// completeness (e.g. token-spend rollup).
    pub source_agents: Vec<FactoryAgentRef>,
}

impl PreparedRun {
    /// Look up the agent triple for a given stage. Returns `None` for
    /// process definitions that do not embed an `agent_ref:` block under
    /// the stage.
    pub fn agent_for_stage(&self, stage_id: &str) -> Option<FactoryAgentRef> {
        self.stage_agents
            .iter()
            .find(|(sid, _)| sid == stage_id)
            .map(|(_, ar)| ar.clone())
    }
}

/// Reserve a run on the platform, resolve every agent reference (failing
/// fast with [`FactoryError::RetiredAgent`] when applicable), and
/// materialise the per-run cache root.
///
/// Returns a [`PreparedRun`] the caller hands to `FactoryEngine::new`.
pub async fn prepare_run_root(
    ctx: &PlatformContext,
    adapter_name: &str,
    process_name: &str,
    project_id: Option<&str>,
) -> Result<PreparedRun, FactoryError> {
    let client_run_id = Uuid::new_v4().to_string();

    // Pre-flight resolver walk: catch retired bindings before the server
    // round-trip so the UI deep-link surfaces with full structured info.
    let process = ctx.client.get_process(process_name).await?;
    let stage_pairs = walk_stage_agents(&process.definition);
    for (_stage_id, reference) in &stage_pairs {
        if let Err(e) = ctx.resolver.resolve(reference.clone()).await {
            return Err(map_resolve_err(e, reference, project_id));
        }
    }

    let reservation: RunReservation = ctx
        .client
        .reserve_run(ReserveRunRequest {
            adapter_name: adapter_name.to_string(),
            process_name: process_name.to_string(),
            project_id: project_id.map(String::from),
            client_run_id,
        })
        .await
        .map_err(|e| {
            // The server-side T020 also rejects retired bindings; preserve
            // the structured RetiredAgent path even when the desktop's
            // resolver passed (e.g. cache staleness window).
            let mut err: FactoryError = e.into();
            if let FactoryError::RetiredAgent {
                project_id: ref mut pid,
                ..
            } = err
            {
                *pid = project_id.map(String::from);
            }
            err
        })?;

    let run_root = ctx
        .client
        .materialise_run_root(&reservation, adapter_name, process_name, &ctx.resolver)
        .await
        .map_err(FactoryError::from)?;

    let source_agents: Vec<FactoryAgentRef> = reservation
        .source_shas
        .agents
        .iter()
        .map(|w| FactoryAgentRef {
            org_agent_id: w.org_agent_id.clone(),
            version: w.version,
            content_hash: w.content_hash.clone(),
        })
        .collect();

    // Map walked stage refs to the reservation's resolved triples in
    // stage-order. The two walks match because the materialiser cross-
    // checks them (spec 124 T043) — but be defensive: pair by index, and
    // skip missing entries cleanly.
    let stage_agents: Vec<(String, FactoryAgentRef)> = stage_pairs
        .into_iter()
        .zip(source_agents.iter())
        .map(|((sid, _), triple)| (sid, triple.clone()))
        .collect();

    let engine_factory_root = run_root.path.clone();

    Ok(PreparedRun {
        run_id: reservation.run_id,
        run_root,
        engine_factory_root,
        stage_agents,
        source_agents,
    })
}

fn map_resolve_err(
    e: ResolveError,
    reference: &EngineAgentReference,
    project_id: Option<&str>,
) -> FactoryError {
    match e {
        ResolveError::RetiredAgent {
            org_agent_id,
            version,
        } => FactoryError::RetiredAgent {
            agent_name: agent_name_from_reference(reference),
            org_agent_id,
            version,
            project_id: project_id.map(String::from),
        },
        ResolveError::NotFound { reference: r } => {
            FactoryError::Resolver(format!("not found: {r}"))
        }
        ResolveError::AmbiguousName { name, count } => {
            FactoryError::Resolver(format!("ambiguous name {name}: {count} matches"))
        }
        ResolveError::VersionMismatch { requested, actual } => FactoryError::Resolver(format!(
            "version mismatch: requested {requested}, catalog has {actual}",
        )),
        ResolveError::Client(c) => FactoryError::Resolver(c.to_string()),
    }
}

fn agent_name_from_reference(r: &EngineAgentReference) -> String {
    match r {
        EngineAgentReference::ById { org_agent_id, .. } => org_agent_id.clone(),
        EngineAgentReference::ByName { name, .. }
        | EngineAgentReference::ByNameLatest { name } => name.clone(),
    }
}

// ---------------------------------------------------------------------------
// On-disk replay queue — spec 124 T053
// ---------------------------------------------------------------------------

/// Maximum events stored per run before the queue is considered overflowed.
/// Exceeding this budget marks the run failed locally (spec 124 T053).
pub const REPLAY_QUEUE_MAX: usize = 1000;

/// Test-only mutex serialising every test that mutates `XDG_DATA_HOME`
/// in this binary. Cargo runs lib tests in parallel by default; without
/// this, two tests reading the same env var see each other's writes.
///
/// Async-aware on purpose: tests that mutate `XDG_DATA_HOME` then `await`
/// on operations reading it must hold the guard across the await points
/// (the lock's whole reason for existing). A `std::sync::MutexGuard` held
/// across `.await` is a clippy-flagged footgun in a multi-thread tokio
/// runtime; `tokio::sync::Mutex` is the textbook fix.
#[cfg(test)]
pub static REPLAY_QUEUE_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Resolve `$XDG_DATA_HOME/oap/factory-run-events/`. Falls back to
/// `dirs::data_dir()` and finally to a temp directory.
pub fn replay_queue_dir() -> PathBuf {
    if let Ok(explicit) = std::env::var("XDG_DATA_HOME")
        && !explicit.is_empty()
    {
        return PathBuf::from(explicit)
            .join("oap")
            .join("factory-run-events");
    }
    if let Some(data) = dirs::data_dir() {
        return data.join("oap").join("factory-run-events");
    }
    std::env::temp_dir()
        .join("oap")
        .join("factory-run-events")
}

fn replay_queue_path(run_id: &str) -> PathBuf {
    replay_queue_dir().join(format!("{run_id}.ndjson"))
}

fn replay_queue_path_in(dir: &std::path::Path, run_id: &str) -> PathBuf {
    dir.join(format!("{run_id}.ndjson"))
}

/// Persisted shape of a queued frame. The variants mirror the
/// `factory.run.*` `OutboundFrame` set; reconstructed on replay through
/// the typed `send_factory_run_*` helpers so envelope-version drift
/// surfaces at compile time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QueuedFrame {
    StageStarted {
        run_id: String,
        stage_id: String,
        agent_ref: FactoryAgentRef,
    },
    StageCompleted {
        run_id: String,
        stage_id: String,
        outcome: FactoryStageOutcome,
        error: Option<String>,
    },
    Completed {
        run_id: String,
        token_spend: FactoryRunTokenSpend,
    },
    Failed {
        run_id: String,
        error: String,
    },
    Cancelled {
        run_id: String,
        reason: Option<String>,
    },
}

/// Append a single frame line to the run's NDJSON queue. Resolves the
/// queue directory once so a parallel test that mutates `XDG_DATA_HOME`
/// between the create-dir and open-file calls cannot stranded a write.
pub async fn enqueue_frame(run_id: &str, frame: &QueuedFrame) -> Result<(), FactoryError> {
    use tokio::io::AsyncWriteExt;
    let dir = replay_queue_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| FactoryError::Io(format!("create replay dir: {e}")))?;
    let path = replay_queue_path_in(&dir, run_id);
    let mut line = serde_json::to_string(frame)
        .map_err(|e| FactoryError::Io(format!("serialise queued frame: {e}")))?;
    line.push('\n');
    let mut f = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await
        .map_err(|e| FactoryError::Io(format!("open replay queue {}: {e}", path.display())))?;
    f.write_all(line.as_bytes())
        .await
        .map_err(|e| FactoryError::Io(format!("write replay queue: {e}")))?;
    Ok(())
}

/// Count the number of frames currently spooled for `run_id`. Used to
/// enforce [`REPLAY_QUEUE_MAX`] without loading the whole file.
pub async fn queue_len(run_id: &str) -> usize {
    let path = replay_queue_path(run_id);
    let Ok(file) = tokio::fs::File::open(&path).await else {
        return 0;
    };
    let mut reader = tokio::io::BufReader::new(file);
    let mut line = String::new();
    let mut count = 0usize;
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => return count,
            Ok(_) => count += 1,
            Err(_) => return count,
        }
    }
}

/// Replay every queued frame for `run_id` through `sync` and clear the
/// file on success. Frames are emitted in append order to preserve the
/// `(run_id, stage_id, status)` ordering the platform handler expects.
pub async fn replay_queue(run_id: &str, sync: &SyncClientInner) -> Result<usize, FactoryError> {
    let path = replay_queue_path(run_id);
    if !path.exists() {
        return Ok(0);
    }
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| FactoryError::Io(format!("read replay queue: {e}")))?;
    let mut sent = 0usize;
    for line in bytes.split(|b| *b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let frame: QueuedFrame = match serde_json::from_slice(line) {
            Ok(f) => f,
            Err(e) => {
                log::warn!(
                    "replay_queue: dropping malformed line in {}: {e}",
                    path.display()
                );
                continue;
            }
        };
        if !dispatch_queued(sync, &frame).await {
            // Still disconnected — stop replay and leave the queue intact
            // so the next reconnect tries again.
            return Ok(sent);
        }
        sent += 1;
    }
    let _ = tokio::fs::remove_file(&path).await;
    Ok(sent)
}

async fn dispatch_queued(sync: &SyncClientInner, frame: &QueuedFrame) -> bool {
    match frame {
        QueuedFrame::StageStarted {
            run_id,
            stage_id,
            agent_ref,
        } => {
            sync.send_factory_run_stage_started(run_id, stage_id, agent_ref.clone())
                .await
        }
        QueuedFrame::StageCompleted {
            run_id,
            stage_id,
            outcome,
            error,
        } => {
            sync.send_factory_run_stage_completed(run_id, stage_id, *outcome, error.clone())
                .await
        }
        QueuedFrame::Completed {
            run_id,
            token_spend,
        } => sync.send_factory_run_completed(run_id, token_spend.clone()).await,
        QueuedFrame::Failed { run_id, error } => {
            sync.send_factory_run_failed(run_id, error.clone()).await
        }
        QueuedFrame::Cancelled { run_id, reason } => {
            sync.send_factory_run_cancelled(run_id, reason.clone()).await
        }
    }
}

// ---------------------------------------------------------------------------
// Run emitter — sends a frame, falls back to the on-disk queue on failure
// ---------------------------------------------------------------------------

/// Owns the duplex handle + the local replay queue for a single in-flight
/// run. Cloning is cheap — only the `Arc<SyncClientInner>` and `String`
/// are duplicated.
#[derive(Clone)]
pub struct RunEmitter {
    sync: Arc<SyncClientInner>,
    run_id: String,
    /// Set when the queue overflows ([`REPLAY_QUEUE_MAX`]) so subsequent
    /// emits stop spooling — a flooded run is already failed locally.
    overflowed: Arc<std::sync::atomic::AtomicBool>,
}

impl RunEmitter {
    pub fn new(app: &AppHandle, run_id: String) -> Result<Self, FactoryError> {
        let sync_state = app
            .try_state::<SyncClientState>()
            .ok_or_else(|| FactoryError::Other("SyncClientState not managed".into()))?;
        Ok(Self {
            sync: sync_state.handle(),
            run_id,
            overflowed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Test seam: build an emitter from an explicit `SyncClientInner`
    /// without going through Tauri state. The integration tests in
    /// `commands/factory.rs` use this to drive a full
    /// stage_started → stage_completed → completed sequence against a
    /// captured outbound channel.
    #[doc(hidden)]
    pub fn from_inner(sync: Arc<SyncClientInner>, run_id: String) -> Self {
        Self {
            sync,
            run_id,
            overflowed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn overflowed(&self) -> bool {
        self.overflowed.load(std::sync::atomic::Ordering::Acquire)
    }

    async fn try_send_or_queue(&self, frame: QueuedFrame) -> Result<(), FactoryError> {
        if self.overflowed() {
            return Err(FactoryError::Other(format!(
                "run {} replay queue overflowed (>{} events)",
                self.run_id, REPLAY_QUEUE_MAX
            )));
        }
        let sent = dispatch_queued(&self.sync, &frame).await;
        if sent {
            return Ok(());
        }
        // Disconnected — spool to disk. Cap enforced before writing.
        let current = queue_len(&self.run_id).await;
        if current >= REPLAY_QUEUE_MAX {
            self.overflowed
                .store(true, std::sync::atomic::Ordering::Release);
            return Err(FactoryError::Other(format!(
                "run {} replay queue overflowed (>{} events)",
                self.run_id, REPLAY_QUEUE_MAX
            )));
        }
        enqueue_frame(&self.run_id, &frame).await
    }

    pub async fn stage_started(
        &self,
        stage_id: &str,
        agent_ref: FactoryAgentRef,
    ) -> Result<(), FactoryError> {
        self.try_send_or_queue(QueuedFrame::StageStarted {
            run_id: self.run_id.clone(),
            stage_id: stage_id.to_string(),
            agent_ref,
        })
        .await
    }

    pub async fn stage_completed(
        &self,
        stage_id: &str,
        outcome: FactoryStageOutcome,
        error: Option<String>,
    ) -> Result<(), FactoryError> {
        self.try_send_or_queue(QueuedFrame::StageCompleted {
            run_id: self.run_id.clone(),
            stage_id: stage_id.to_string(),
            outcome,
            error,
        })
        .await
    }

    pub async fn completed(&self, token_spend: FactoryRunTokenSpend) -> Result<(), FactoryError> {
        self.try_send_or_queue(QueuedFrame::Completed {
            run_id: self.run_id.clone(),
            token_spend,
        })
        .await
    }

    pub async fn failed(&self, error: String) -> Result<(), FactoryError> {
        self.try_send_or_queue(QueuedFrame::Failed {
            run_id: self.run_id.clone(),
            error,
        })
        .await
    }

    pub async fn cancelled(&self, reason: Option<String>) -> Result<(), FactoryError> {
        self.try_send_or_queue(QueuedFrame::Cancelled {
            run_id: self.run_id.clone(),
            reason,
        })
        .await
    }

    /// Best-effort drain of any leftover frames after the duplex stream
    /// reconnects. Called from reconnect handlers; safe to invoke even
    /// when nothing is queued.
    pub async fn drain(&self) -> Result<usize, FactoryError> {
        replay_queue(&self.run_id, &self.sync).await
    }
}

// ---------------------------------------------------------------------------
// ISO timestamp helper — kept here so the duplex helpers in this file
// don't pull a chrono dep through commands/factory.rs.
// ---------------------------------------------------------------------------

pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_retired_agent_body_extracts_triple() {
        let body = "agent \"extract\" (org-agent-1 v3) is retired upstream";
        let err = parse_retired_agent_body(body).unwrap();
        match err {
            FactoryError::RetiredAgent {
                agent_name,
                org_agent_id,
                version,
                project_id,
            } => {
                assert_eq!(agent_name, "extract");
                assert_eq!(org_agent_id, "org-agent-1");
                assert_eq!(version, 3);
                assert!(project_id.is_none());
            }
            other => panic!("expected RetiredAgent, got {other:?}"),
        }
    }

    #[test]
    fn parse_retired_agent_body_returns_none_for_garbage() {
        assert!(parse_retired_agent_body("not a retired-agent body").is_none());
        assert!(parse_retired_agent_body("agent \"x\"").is_none());
    }

    #[test]
    fn walk_stage_agents_pairs_stage_with_first_ref() {
        let proc_def = json!({
            "stages": [
                { "id": "s0", "agent_ref": { "by_name_latest": { "name": "extract" } } },
                { "id": "s1", "agent_ref": { "by_name": { "name": "design", "version": 2 } } },
            ]
        });
        let pairs = walk_stage_agents(&proc_def);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, "s0");
        assert_eq!(pairs[1].0, "s1");
    }

    #[test]
    fn walk_stage_agents_skips_stages_without_agent_ref() {
        let proc_def = json!({
            "stages": [
                { "id": "s0" },
                { "id": "s1", "agent_ref": { "by_name_latest": { "name": "design" } } },
            ]
        });
        let pairs = walk_stage_agents(&proc_def);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "s1");
    }

    #[test]
    fn factory_error_retired_agent_message_includes_deep_link() {
        let err = FactoryError::RetiredAgent {
            agent_name: "extract".into(),
            org_agent_id: "ag-1".into(),
            version: 2,
            project_id: Some("p-1".into()),
        };
        let s = err.to_string();
        assert!(s.contains("extract"));
        assert!(s.contains("ag-1"));
        assert!(s.contains("v2"));
        assert!(s.contains("/app/project/p-1/agents"));
    }

    #[test]
    fn factory_error_retired_agent_ad_hoc_run_renders_placeholder() {
        let err = FactoryError::RetiredAgent {
            agent_name: "extract".into(),
            org_agent_id: "ag-1".into(),
            version: 2,
            project_id: None,
        };
        let s = err.to_string();
        assert!(s.contains("/app/project/<ad-hoc>/agents"));
    }

    #[tokio::test]
    async fn replay_queue_dir_honours_xdg_data_home() {
        let _guard = REPLAY_QUEUE_ENV_LOCK.lock().await;
        let prev = std::env::var("XDG_DATA_HOME").ok();
        // SAFETY: REPLAY_QUEUE_ENV_LOCK serialises every env-mutating
        // test in this binary so the read below cannot race a parallel
        // set_var from a sibling test.
        unsafe { std::env::set_var("XDG_DATA_HOME", "/tmp/oap-test-data") };
        let path = replay_queue_dir();
        assert_eq!(
            path,
            PathBuf::from("/tmp/oap-test-data")
                .join("oap")
                .join("factory-run-events")
        );
        match prev {
            Some(v) => unsafe { std::env::set_var("XDG_DATA_HOME", v) },
            None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn enqueue_then_count_matches_appended_lines() {
        let _guard = REPLAY_QUEUE_ENV_LOCK.lock().await;
        let tmp = tempfile::tempdir().unwrap();
        // SAFETY: REPLAY_QUEUE_ENV_LOCK above prevents a parallel set_var.
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        let run_id = "test-run-enqueue";
        let frame = QueuedFrame::Failed {
            run_id: run_id.to_string(),
            error: "boom".into(),
        };
        for _ in 0..3 {
            enqueue_frame(run_id, &frame).await.unwrap();
        }
        assert_eq!(queue_len(run_id).await, 3);
        unsafe { std::env::remove_var("XDG_DATA_HOME") };
    }
}
