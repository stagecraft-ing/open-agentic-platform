//! Duplex sync consumer for the stagecraft control plane (spec 110 Phase 2).
//!
//! Opens the authenticated `/api/sync/duplex` WebSocket, performs the
//! handshake via query parameters (the Encore stream convention), and runs
//! a resilient read/write loop:
//!
//!   - Receives `ServerEnvelope` frames and routes them through a
//!     registration-based dispatch table. Unknown kinds log and no-op so
//!     this bootstraps safely ahead of spec 110 §4 and spec 111.
//!   - Answers `sync.heartbeat` frames with matching client heartbeats and
//!     records the last observed workspace cursor.
//!   - Auto-reconnects with exponential backoff on disconnect; passes the
//!     last observed cursor back as `lastServerCursor` on reconnect so the
//!     server can detect gaps (see 087 §5.3, duplex.ts).
//!
//! Authority invariant (087 §5.3): the desktop MUST NOT forge
//! `ServerEnvelope` frames. This module is read/ack/dispatch only. Outbound
//! traffic is limited to `sync.heartbeat`, `sync.ack`, and `sync.resync_request`;
//! progress envelopes like `execution.status` live on the StagecraftClient
//! HTTP path today and will migrate to this stream in a later phase.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// The duplex protocol version this client speaks. Must match
/// `ENVELOPE_SCHEMA_VERSION` in `platform/services/stagecraft/api/sync/types.ts`.
pub const ENVELOPE_SCHEMA_VERSION: u8 = 1;

// ---------------------------------------------------------------------------
// Wire-level envelope types (mirror the typescript wire shapes)
// ---------------------------------------------------------------------------

/// Envelope meta carried by every frame on the stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeMeta {
    /// Schema version — strict equality with [`ENVELOPE_SCHEMA_VERSION`].
    pub v: u8,
    pub event_id: String,
    pub sent_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
}

/// Meta on server-originated envelopes — extends [`EnvelopeMeta`] with the
/// workspace cursor the server assigned.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerMeta {
    pub v: u8,
    pub event_id: String,
    pub sent_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causation_id: Option<String>,
    pub org_cursor: String,
    pub org_id: String,
}

/// Flat counterpart of `ServerEnvelopeWire` in
/// `platform/services/stagecraft/api/sync/types.ts`.
///
/// All payload fields are optional because a single concrete frame only
/// populates the subset relevant to its `kind`. Callers narrow by reading
/// `kind` first.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerEnvelopeWire {
    pub kind: String,
    pub meta: ServerMeta,
    #[serde(default)]
    pub policy_bundle_id: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub change: Option<String>,
    #[serde(default)]
    pub details: Option<Value>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub environment_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub pipeline_id: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub stage_id: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub client_event_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub server_started_at: Option<String>,
    #[serde(default)]
    pub cursor_gap: Option<bool>,
    // spec 110 §2.1 — factory.run.request fields
    #[serde(default)]
    pub adapter: Option<String>,
    #[serde(default)]
    pub actor_user_id: Option<String>,
    #[serde(default)]
    pub knowledge: Option<Vec<KnowledgeBundle>>,
    #[serde(default)]
    pub business_docs: Option<Vec<EnvelopeBusinessDoc>>,
    #[serde(default)]
    pub requested_at: Option<String>,
    #[serde(default)]
    pub deadline_at: Option<String>,
    // spec 111 §2.3 — agent.catalog.updated / agent.catalog.snapshot fields.
    // Bodies and frontmatter are decoded as `serde_json::Value` because the
    // `CatalogFrontmatter` TS type is an `UnifiedFrontmatter & { [k]: unknown }`
    // union whose `extra` flatten keys are opaque to the Rust decoder; the
    // desktop cache preserves them through the JSONB round-trip.
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub frontmatter: Option<Value>,
    #[serde(default)]
    pub body_markdown: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub entries: Option<Vec<AgentCatalogSnapshotEntry>>,
    #[serde(default)]
    pub generated_at: Option<String>,
    // spec 112 §7 — project.catalog.upsert fields.
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub org_id: Option<String>,
    #[serde(default)]
    pub factory_adapter_id: Option<String>,
    #[serde(default)]
    pub detection_level: Option<String>,
    #[serde(default)]
    pub repo: Option<ProjectCatalogRepo>,
    #[serde(default)]
    pub opc_deep_link: Option<String>,
    #[serde(default)]
    pub tombstone: Option<bool>,
}

/// Mirror of the {@link ServerProjectCatalogUpsert} `repo` sub-object
/// from stagecraft's `api/sync/types.ts` (spec 112 §7).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCatalogRepo {
    pub github_org: String,
    pub repo_name: String,
    pub default_branch: String,
    pub clone_url: String,
    pub html_url: String,
}

/// Mirror of {@link AgentCatalogSnapshotEntry} from stagecraft's
/// `api/sync/types.ts`. The snapshot is a directory (hashes only) so the
/// desktop can diff its local cache and pull bodies lazily via
/// `agent.catalog.fetch_request` (spec 111 §2.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCatalogSnapshotEntry {
    pub agent_id: String,
    pub name: String,
    pub version: u32,
    pub status: String,
    pub content_hash: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBundle {
    pub object_id: String,
    pub filename: String,
    pub content_hash: String,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeBusinessDoc {
    pub name: String,
    pub storage_ref: String,
}

/// Set of server envelope kinds this client accepts. Guarding at the
/// boundary stops a drifted server or a hostile proxy from slipping an
/// unknown kind through serde's default decoder.
const SERVER_KINDS: &[&str] = &[
    "policy.updated",
    "grant.updated",
    "deploy.status",
    "workspace.updated",
    "project.updated",
    "factory.event",
    "factory.run.request",
    "agent.catalog.updated",
    "agent.catalog.snapshot",
    "project.catalog.upsert",
    "sync.ack",
    "sync.nack",
    "sync.resync_required",
    "sync.heartbeat",
    "sync.hello",
];

/// Mirrors `isClientEnvelope` on the stagecraft side — enforces schema
/// version and a known kind. Returns `true` when the frame is safe to
/// dispatch.
pub fn is_server_envelope(raw: &ServerEnvelopeWire) -> bool {
    raw.meta.v == ENVELOPE_SCHEMA_VERSION && SERVER_KINDS.contains(&raw.kind.as_str())
}

// ---------------------------------------------------------------------------
// Outbound frames (what the desktop can write back on the wire)
// ---------------------------------------------------------------------------

/// Outbound envelope variants the consumer knows how to emit. Richer client
/// variants (execution.status, audit.candidate) are added in later phases
/// via their own typed constructors.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OutboundFrame {
    #[serde(rename = "sync.heartbeat")]
    Heartbeat { meta: EnvelopeMeta },
    #[serde(rename = "sync.ack")]
    Ack {
        meta: EnvelopeMeta,
        #[serde(rename = "serverEventId")]
        server_event_id: String,
    },
    #[serde(rename = "sync.resync_request")]
    ResyncRequest {
        meta: EnvelopeMeta,
        #[serde(rename = "sinceCursor", skip_serializing_if = "Option::is_none")]
        since_cursor: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// Spec 110 §2.2 — desktop observation that a `factory.run.request` was
    /// received. Carries the minted tab `session_id` and the OPC instance id
    /// so stagecraft can distinguish multiple desktops competing for the same
    /// run (the first ack wins; others will receive `sync.nack`).
    #[serde(rename = "factory.run.ack")]
    FactoryRunAck {
        meta: EnvelopeMeta,
        #[serde(rename = "pipelineId")]
        pipeline_id: String,
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(rename = "opcInstanceId")]
        opc_instance_id: String,
        accepted: bool,
        #[serde(rename = "declineReason", skip_serializing_if = "Option::is_none")]
        decline_reason: Option<String>,
        #[serde(rename = "observedAt")]
        observed_at: String,
    },
    /// Spec 111 §2.3 — desktop requests the full body of an agent whose hash
    /// from the snapshot does not match its local cache. The stagecraft side
    /// replies with a targeted `agent.catalog.updated`. Reason is a small
    /// closed set so the server can log/aggregate cache-miss patterns.
    #[serde(rename = "agent.catalog.fetch_request")]
    AgentCatalogFetchRequest {
        meta: EnvelopeMeta,
        #[serde(rename = "agentId")]
        agent_id: String,
        reason: AgentCatalogFetchReason,
        #[serde(rename = "observedAt")]
        observed_at: String,
    },
}

/// Reason enum for {@link OutboundFrame::AgentCatalogFetchRequest}. Mirrors
/// the closed set in stagecraft's `ClientAgentCatalogFetchRequest.reason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCatalogFetchReason {
    CacheMiss,
    HashMismatch,
    ManualRefresh,
}

// ---------------------------------------------------------------------------
// Handler trait + dispatch table
// ---------------------------------------------------------------------------

/// Handler for a single server envelope kind. Spec 110 §10 requires the
/// dispatch surface be extensible enough that spec 111's
/// `agent.catalog.updated` can register without refactoring the consumer.
pub trait EnvelopeHandler: Send + Sync {
    fn handle(&self, envelope: &ServerEnvelopeWire);
}

/// Boxed handler for a single function. Convenience for the bootstrap.
pub struct FnHandler<F: Fn(&ServerEnvelopeWire) + Send + Sync + 'static>(pub F);

impl<F> EnvelopeHandler for FnHandler<F>
where
    F: Fn(&ServerEnvelopeWire) + Send + Sync + 'static,
{
    fn handle(&self, envelope: &ServerEnvelopeWire) {
        (self.0)(envelope)
    }
}

/// Thread-safe registry keyed by `kind`. Follows the pattern spec 110 §10
/// calls out: `HashMap<&'static str, Arc<dyn EnvelopeHandler>>` or equivalent.
#[derive(Default)]
pub struct DispatchTable {
    inner: RwLock<HashMap<String, Arc<dyn EnvelopeHandler>>>,
}

impl DispatchTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for a given envelope kind. Replaces any existing
    /// handler for the same kind.
    pub fn register(&self, kind: &str, handler: Arc<dyn EnvelopeHandler>) {
        if let Ok(mut g) = self.inner.write() {
            g.insert(kind.to_string(), handler);
        }
    }

    /// Lookup the handler for a kind, if one is registered.
    pub fn get(&self, kind: &str) -> Option<Arc<dyn EnvelopeHandler>> {
        self.inner.read().ok().and_then(|g| g.get(kind).cloned())
    }

    /// Test-only: list registered kinds for assertions.
    #[cfg(test)]
    pub fn kinds(&self) -> Vec<String> {
        self.inner
            .read()
            .map(|g| g.keys().cloned().collect())
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Client state
// ---------------------------------------------------------------------------

/// Configuration for the duplex consumer.
#[derive(Debug, Clone)]
pub struct SyncClientConfig {
    /// Stagecraft HTTP base URL (e.g. `https://stagecraft.ing`). Converted
    /// to ws:// or wss:// internally.
    pub base_url: String,
    /// Stable client identifier for this OPC process. Persisted across
    /// reconnects.
    pub client_id: String,
    /// Human-readable client version — informational only.
    pub client_version: Option<String>,
    /// Rauthy JWT used in the Authorization header on the handshake.
    pub auth_token: String,
}

/// Shared inner state for the duplex consumer. Held in an `Arc` so external
/// modules (e.g. the factory.run.request handler) can clone a handle and
/// post `factory.run.ack` frames without touching the Tauri state registry
/// on each call.
#[derive(Default)]
pub struct SyncClientInner {
    dispatch: Arc<DispatchTable>,
    last_cursor: Arc<RwLock<Option<String>>>,
    /// Sender for the currently-connected duplex session. `None` whenever the
    /// socket is disconnected; external callers treat a `None` as "best-effort
    /// drop" rather than blocking.
    outbound: RwLock<Option<mpsc::Sender<OutboundFrame>>>,
}

impl SyncClientInner {
    fn set_outbound(&self, tx: Option<mpsc::Sender<OutboundFrame>>) {
        if let Ok(mut g) = self.outbound.write() {
            *g = tx;
        }
    }

    fn current_outbound(&self) -> Option<mpsc::Sender<OutboundFrame>> {
        self.outbound.read().ok().and_then(|g| g.clone())
    }

    /// Emit a pre-built outbound frame if the duplex stream is connected.
    /// Returns `true` when the frame was queued on the outbound channel.
    pub async fn send(&self, frame: OutboundFrame) -> bool {
        let Some(tx) = self.current_outbound() else {
            return false;
        };
        tx.send(frame).await.is_ok()
    }

    /// Emit a typed `agent.catalog.fetch_request` frame (spec 111 §2.3).
    /// Kept behind the catalog feature flag at the desktop caller site —
    /// this function does not gate itself so tests can exercise the wire
    /// path without flipping a flag. Returns `false` if the duplex stream
    /// is not connected.
    pub async fn send_agent_catalog_fetch_request(
        &self,
        agent_id: &str,
        reason: AgentCatalogFetchReason,
    ) -> bool {
        let frame = OutboundFrame::AgentCatalogFetchRequest {
            meta: new_meta(),
            agent_id: agent_id.to_string(),
            reason,
            observed_at: chrono::Utc::now().to_rfc3339(),
        };
        self.send(frame).await
    }

    /// Emit a typed `factory.run.ack` frame (spec 110 §2.2). Returns `false`
    /// when the duplex stream is not currently connected — callers log but
    /// do not retry; the dedupe marker prevents re-ack on reconnect.
    pub async fn send_factory_run_ack(
        &self,
        pipeline_id: &str,
        session_id: &str,
        opc_instance_id: &str,
        accepted: bool,
        decline_reason: Option<String>,
    ) -> bool {
        let frame = OutboundFrame::FactoryRunAck {
            meta: new_meta(),
            pipeline_id: pipeline_id.to_string(),
            session_id: session_id.to_string(),
            opc_instance_id: opc_instance_id.to_string(),
            accepted,
            decline_reason,
            observed_at: chrono::Utc::now().to_rfc3339(),
        };
        self.send(frame).await
    }
}

/// Tauri-managed handle to the background duplex consumer.
pub struct SyncClientState {
    inner: Arc<SyncClientInner>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl Default for SyncClientState {
    fn default() -> Self {
        Self {
            inner: Arc::new(SyncClientInner::default()),
            join: Mutex::new(None),
        }
    }
}

impl SyncClientState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dispatch_table(&self) -> Arc<DispatchTable> {
        self.inner.dispatch.clone()
    }

    pub fn last_cursor(&self) -> Option<String> {
        self.inner.last_cursor.read().ok().and_then(|g| g.clone())
    }

    /// Clone the inner handle. Callers hold it across async tasks without
    /// touching `AppHandle` on every send.
    ///
    /// Named `handle()` rather than `inner()` because Tauri's `State<T>`
    /// already exposes `.inner() -> &T`, which would shadow this method on
    /// managed-state call sites.
    pub fn handle(&self) -> Arc<SyncClientInner> {
        self.inner.clone()
    }

    /// Spawn the background reconnect loop. Returns immediately. If an
    /// existing task is running the old task is aborted first.
    pub async fn spawn(&self, config: SyncClientConfig) {
        let mut guard = self.join.lock().await;
        if let Some(prev) = guard.take() {
            prev.abort();
        }
        let inner = self.inner.clone();
        let task = tokio::spawn(async move {
            run_forever(config, inner).await;
        });
        *guard = Some(task);
    }

    /// Stop the background consumer if running.
    pub async fn shutdown(&self) {
        let mut guard = self.join.lock().await;
        if let Some(task) = guard.take() {
            task.abort();
        }
    }
}

// ---------------------------------------------------------------------------
// Reconnect loop
// ---------------------------------------------------------------------------

const MIN_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(25);

async fn run_forever(config: SyncClientConfig, inner: Arc<SyncClientInner>) {
    let mut backoff = MIN_BACKOFF;
    loop {
        let cursor_snapshot = inner.last_cursor.read().ok().and_then(|g| g.clone());
        match connect_and_run(&config, cursor_snapshot, &inner).await {
            Ok(()) => {
                log::info!("sync_client: duplex stream closed cleanly — reconnecting");
                backoff = MIN_BACKOFF;
            }
            Err(err) => {
                log::warn!(
                    "sync_client: duplex stream error — reconnecting in {:?}: {err}",
                    backoff
                );
            }
        }
        // Clear the outbound channel so external callers stop enqueuing
        // frames onto a dead session while we wait to reconnect.
        inner.set_outbound(None);
        tokio::time::sleep(backoff).await;
        backoff = std::cmp::min(backoff * 2, MAX_BACKOFF);
    }
}

/// Convert a stagecraft HTTP base URL to the ws:// or wss:// duplex URL
/// with the handshake query parameters appended.
fn build_duplex_url(base_url: &str, client_id: &str, cursor: Option<&str>) -> String {
    let trimmed = base_url.trim_end_matches('/');
    let ws_base = if let Some(rest) = trimmed.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        trimmed.to_string()
    };
    let mut url = format!(
        "{ws_base}/api/sync/duplex?clientId={}&clientKind=desktop-opc",
        urlencode(client_id)
    );
    if let Some(c) = cursor {
        url.push_str("&lastServerCursor=");
        url.push_str(&urlencode(c));
    }
    url
}

/// Minimal percent-encoder for the handshake query values. The
/// `reqwest::Url` crate would work but we want to avoid a fresh dep when
/// these inputs are UUID-shaped.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn connect_and_run(
    config: &SyncClientConfig,
    cursor: Option<String>,
    inner: &Arc<SyncClientInner>,
) -> Result<(), String> {
    let url = build_duplex_url(&config.base_url, &config.client_id, cursor.as_deref());
    log::info!("sync_client: connecting to {url}");

    let mut req = url
        .into_client_request()
        .map_err(|e| format!("build handshake request: {e}"))?;
    req.headers_mut().insert(
        "Authorization",
        format!("Bearer {}", config.auth_token)
            .parse()
            .map_err(|e| format!("bad auth header: {e}"))?,
    );

    let (stream, _response) = tokio_tungstenite::connect_async(req)
        .await
        .map_err(|e| format!("connect: {e}"))?;

    log::info!(
        "sync_client: duplex connected (client_id={})",
        config.client_id
    );

    run_duplex_session(stream, inner).await
}

async fn run_duplex_session(
    stream: WsStream,
    inner: &Arc<SyncClientInner>,
) -> Result<(), String> {
    let (mut sink, mut source) = stream.split();
    let (out_tx, mut out_rx) = mpsc::channel::<OutboundFrame>(32);
    // Publish the sender so external handlers (factory.run.request, etc.)
    // can emit acks while this session is alive.
    inner.set_outbound(Some(out_tx.clone()));
    let dispatch = &inner.dispatch;
    let last_cursor = &inner.last_cursor;

    // Heartbeat producer — independent task so a slow server read doesn't
    // block outbound heartbeats.
    let hb_tx = out_tx.clone();
    let heartbeat_task = tokio::spawn(async move {
        let mut ticker = interval(HEARTBEAT_INTERVAL);
        ticker.tick().await; // consume the immediate first tick
        loop {
            ticker.tick().await;
            let frame = OutboundFrame::Heartbeat {
                meta: new_meta(),
            };
            if hb_tx.send(frame).await.is_err() {
                break;
            }
        }
    });

    // Outbound writer — drains the mpsc channel onto the socket.
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = out_rx.recv().await {
            let json = match serde_json::to_string(&frame) {
                Ok(j) => j,
                Err(e) => {
                    log::warn!("sync_client: serialize outbound: {e}");
                    continue;
                }
            };
            if let Err(e) = sink.send(Message::Text(json.into())).await {
                log::warn!("sync_client: outbound send failed: {e}");
                break;
            }
        }
    });

    // Inbound reader — blocks on the socket and dispatches.
    let read_result = async {
        while let Some(frame) = source.next().await {
            let msg = frame.map_err(|e| format!("read: {e}"))?;
            match msg {
                Message::Text(text) => {
                    handle_text_frame(&text, dispatch, last_cursor, &out_tx).await;
                }
                Message::Binary(bytes) => {
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => {
                            handle_text_frame(text, dispatch, last_cursor, &out_tx).await;
                        }
                        Err(_) => log::warn!("sync_client: non-utf8 binary frame ignored"),
                    }
                }
                Message::Ping(_) | Message::Pong(_) => {}
                Message::Close(_) => {
                    log::info!("sync_client: server closed the duplex stream");
                    return Ok(());
                }
                Message::Frame(_) => {}
            }
        }
        Ok(())
    }
    .await;

    heartbeat_task.abort();
    inner.set_outbound(None);
    drop(out_tx);
    let _ = writer_task.await;
    read_result
}

async fn handle_text_frame(
    text: &str,
    dispatch: &Arc<DispatchTable>,
    last_cursor: &Arc<RwLock<Option<String>>>,
    out_tx: &mpsc::Sender<OutboundFrame>,
) {
    let envelope: ServerEnvelopeWire = match serde_json::from_str(text) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("sync_client: malformed envelope ({e}) — ignored");
            return;
        }
    };

    if !is_server_envelope(&envelope) {
        log::warn!(
            "sync_client: rejected envelope with unknown kind or bad schema: kind={} v={}",
            envelope.kind,
            envelope.meta.v
        );
        return;
    }

    // Update the last observed org cursor so we can resume on reconnect.
    if !envelope.meta.org_cursor.is_empty()
        && let Ok(mut g) = last_cursor.write()
    {
        *g = Some(envelope.meta.org_cursor.clone());
    }

    match envelope.kind.as_str() {
        "sync.heartbeat" => {
            // Server-side heartbeat. Our own heartbeat task handles the
            // outbound side; nothing more to do here.
        }
        "sync.resync_required" => {
            log::info!(
                "sync_client: server requested resync (reason={:?})",
                envelope.reason
            );
            let cursor = last_cursor.read().ok().and_then(|g| g.clone());
            let _ = out_tx
                .send(OutboundFrame::ResyncRequest {
                    meta: new_meta(),
                    since_cursor: cursor,
                    reason: Some("server_requested".to_string()),
                })
                .await;
        }
        "sync.hello" => {
            log::info!(
                "sync_client: duplex hello received (session_id={:?}, cursor_gap={:?})",
                envelope.session_id,
                envelope.cursor_gap
            );
        }
        "sync.ack" | "sync.nack" => {
            // No inbox to reconcile yet — tracked in a later phase.
        }
        _ => {
            // Ack authoritative events before dispatching so a slow handler
            // doesn't stall the server's outbox tracking.
            let event_id = envelope.meta.event_id.clone();
            let _ = out_tx
                .send(OutboundFrame::Ack {
                    meta: new_meta(),
                    server_event_id: event_id,
                })
                .await;

            if let Some(handler) = dispatch.get(&envelope.kind) {
                handler.handle(&envelope);
            } else {
                log::info!(
                    "sync_client: received {} — no handler registered",
                    envelope.kind
                );
            }
        }
    }
}

fn new_meta() -> EnvelopeMeta {
    EnvelopeMeta {
        v: ENVELOPE_SCHEMA_VERSION,
        event_id: uuid::Uuid::new_v4().to_string(),
        sent_at: chrono::Utc::now().to_rfc3339(),
        correlation_id: None,
        causation_id: None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(v: u8, cursor: &str) -> ServerMeta {
        ServerMeta {
            v,
            event_id: "evt-1".into(),
            sent_at: "2026-04-21T00:00:00Z".into(),
            correlation_id: None,
            causation_id: None,
            org_cursor: cursor.into(),
            org_id: "org-1".into(),
        }
    }

    fn empty_envelope(kind: &str, v: u8) -> ServerEnvelopeWire {
        ServerEnvelopeWire {
            kind: kind.into(),
            meta: meta(v, "c1"),
            policy_bundle_id: None,
            summary: None,
            user_id: None,
            change: None,
            details: None,
            project_id: None,
            environment_id: None,
            status: None,
            detail: None,
            pipeline_id: None,
            event_type: None,
            stage_id: None,
            actor: None,
            client_event_id: None,
            reason: None,
            session_id: None,
            server_started_at: None,
            cursor_gap: None,
            adapter: None,
            actor_user_id: None,
            knowledge: None,
            business_docs: None,
            requested_at: None,
            deadline_at: None,
            agent_id: None,
            name: None,
            version: None,
            content_hash: None,
            frontmatter: None,
            body_markdown: None,
            updated_at: None,
            entries: None,
            generated_at: None,
            slug: None,
            description: None,
            org_id: None,
            factory_adapter_id: None,
            detection_level: None,
            repo: None,
            opc_deep_link: None,
            tombstone: None,
        }
    }

    #[test]
    fn accepts_known_kinds_at_v1() {
        for kind in [
            "factory.run.request",
            "factory.event",
            "sync.hello",
            "sync.heartbeat",
            "policy.updated",
            "project.catalog.upsert",
        ] {
            assert!(
                is_server_envelope(&empty_envelope(kind, 1)),
                "kind {kind} should pass the guard",
            );
        }
    }

    #[test]
    fn project_catalog_upsert_deserializes_from_wire_json() {
        // Mirrors the shape stagecraft emits per spec 112 §7 — repo block
        // with camelCase fields and optional detectionLevel.
        let raw = r#"{
          "kind": "project.catalog.upsert",
          "meta": {
            "v": 1,
            "eventId": "e1",
            "sentAt": "2026-04-23T00:00:00Z",
            "orgCursor": "c-1",
            "orgId": "org-1"
          },
          "projectId": "p-1",
          "orgId": "org-1",
          "name": "Portal",
          "slug": "portal",
          "description": "desc",
          "factoryAdapterId": "adap-1",
          "detectionLevel": "scaffold_only",
          "repo": {
            "githubOrg": "acme",
            "repoName": "portal",
            "defaultBranch": "main",
            "cloneUrl": "https://github.com/acme/portal.git",
            "htmlUrl": "https://github.com/acme/portal"
          },
          "opcDeepLink": "opc://project/open?project_id=p-1&url=https%3A%2F%2Fgithub.com%2Facme%2Fportal.git&level=scaffold_only",
          "tombstone": false,
          "updatedAt": "2026-04-23T00:00:01Z"
        }"#;
        let env: ServerEnvelopeWire = serde_json::from_str(raw).expect("parses");
        assert!(is_server_envelope(&env));
        assert_eq!(env.kind, "project.catalog.upsert");
        assert_eq!(env.project_id.as_deref(), Some("p-1"));
        assert_eq!(env.name.as_deref(), Some("Portal"));
        assert_eq!(env.slug.as_deref(), Some("portal"));
        assert_eq!(env.detection_level.as_deref(), Some("scaffold_only"));
        assert_eq!(env.tombstone, Some(false));
        let repo = env.repo.expect("repo present");
        assert_eq!(repo.github_org, "acme");
        assert_eq!(repo.repo_name, "portal");
        assert_eq!(repo.default_branch, "main");
    }

    #[test]
    fn rejects_unknown_kind() {
        assert!(!is_server_envelope(&empty_envelope("totally.made.up", 1)));
    }

    #[test]
    fn rejects_wrong_schema_version() {
        assert!(!is_server_envelope(&empty_envelope("sync.hello", 2)));
    }

    #[test]
    fn factory_run_request_deserializes_from_wire_json() {
        // Sample mirrors what stagecraft sends — camelCase field names, with
        // knowledge and businessDocs arrays.
        let raw = r#"{
          "kind": "factory.run.request",
          "meta": {
            "v": 1,
            "eventId": "e1",
            "sentAt": "2026-04-21T00:00:00Z",
            "orgCursor": "cur-42",
            "orgId": "org-1"
          },
          "projectId": "p1",
          "pipelineId": "pl-1",
          "adapter": "rest",
          "actorUserId": "u1",
          "knowledge": [
            {
              "objectId": "k1",
              "filename": "spec.md",
              "contentHash": "abc",
              "downloadUrl": "https://example/k1"
            }
          ],
          "businessDocs": [{"name": "doc", "storageRef": "s3://x"}],
          "policyBundleId": "pb-1",
          "requestedAt": "2026-04-21T00:00:01Z",
          "deadlineAt": "2026-04-21T01:00:00Z"
        }"#;
        let env: ServerEnvelopeWire = serde_json::from_str(raw).expect("deserialize");
        assert!(is_server_envelope(&env));
        assert_eq!(env.pipeline_id.as_deref(), Some("pl-1"));
        assert_eq!(env.adapter.as_deref(), Some("rest"));
        assert_eq!(env.knowledge.as_ref().unwrap().len(), 1);
        assert_eq!(
            env.knowledge.as_ref().unwrap()[0].content_hash,
            "abc".to_string()
        );
        assert_eq!(env.meta.org_cursor, "cur-42");
    }

    #[test]
    fn dispatch_table_registers_and_dispatches() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let table = DispatchTable::new();
        table.register(
            "factory.run.request",
            Arc::new(FnHandler(move |_env| {
                c.fetch_add(1, Ordering::SeqCst);
            })),
        );
        assert!(table.kinds().contains(&"factory.run.request".to_string()));

        let handler = table
            .get("factory.run.request")
            .expect("handler should be registered");
        handler.handle(&empty_envelope("factory.run.request", 1));
        handler.handle(&empty_envelope("factory.run.request", 1));
        assert_eq!(count.load(Ordering::SeqCst), 2);

        assert!(table.get("unknown.kind").is_none());
    }

    #[test]
    fn dispatch_table_replaces_existing_handler() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let hits = Arc::new(AtomicUsize::new(0));
        let h2 = hits.clone();
        let table = DispatchTable::new();
        table.register(
            "factory.event",
            Arc::new(FnHandler(|_env| {
                panic!("old handler should have been replaced");
            })),
        );
        table.register(
            "factory.event",
            Arc::new(FnHandler(move |_env| {
                h2.fetch_add(1, Ordering::SeqCst);
            })),
        );
        table
            .get("factory.event")
            .unwrap()
            .handle(&empty_envelope("factory.event", 1));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn build_duplex_url_handles_https_http_and_cursor() {
        assert_eq!(
            build_duplex_url("https://stagecraft.ing/", "cid-1", None),
            "wss://stagecraft.ing/api/sync/duplex?clientId=cid-1&clientKind=desktop-opc"
        );
        assert_eq!(
            build_duplex_url("http://localhost:4000", "cid-1", Some("cur/42")),
            "ws://localhost:4000/api/sync/duplex?clientId=cid-1&clientKind=desktop-opc&lastServerCursor=cur%2F42"
        );
    }

    #[test]
    fn urlencode_escapes_reserved_chars() {
        assert_eq!(urlencode("abc-123.~_"), "abc-123.~_");
        assert_eq!(urlencode("a b/c?d"), "a%20b%2Fc%3Fd");
    }

    #[test]
    fn malformed_json_is_dropped_without_panic() {
        // Regression: a stray non-envelope frame must not crash the reader.
        let env: Result<ServerEnvelopeWire, _> = serde_json::from_str("{\"kind\":123}");
        assert!(env.is_err());
    }

    #[test]
    fn factory_run_ack_serializes_to_camelcase_wire_shape() {
        // Spec 110 §2.2: the wire shape must match stagecraft's
        // ClientFactoryRunAck exactly — camelCase keys, the right `kind`,
        // and optional fields omitted when unset.
        let frame = OutboundFrame::FactoryRunAck {
            meta: EnvelopeMeta {
                v: ENVELOPE_SCHEMA_VERSION,
                event_id: "e1".into(),
                sent_at: "2026-04-21T00:00:00Z".into(),
                correlation_id: None,
                causation_id: None,
            },
            pipeline_id: "pl-1".into(),
            session_id: "s-1".into(),
            opc_instance_id: "opc-1".into(),
            accepted: true,
            decline_reason: None,
            observed_at: "2026-04-21T00:00:01Z".into(),
        };
        let json = serde_json::to_value(&frame).unwrap();
        assert_eq!(json["kind"], "factory.run.ack");
        assert_eq!(json["pipelineId"], "pl-1");
        assert_eq!(json["sessionId"], "s-1");
        assert_eq!(json["opcInstanceId"], "opc-1");
        assert_eq!(json["accepted"], true);
        assert!(
            json.get("declineReason").is_none(),
            "declineReason must be omitted when None"
        );
        assert_eq!(json["observedAt"], "2026-04-21T00:00:01Z");
        assert_eq!(json["meta"]["v"], 1);
        assert_eq!(json["meta"]["eventId"], "e1");
    }

    #[test]
    fn factory_run_ack_include_decline_reason_when_rejected() {
        let frame = OutboundFrame::FactoryRunAck {
            meta: EnvelopeMeta {
                v: ENVELOPE_SCHEMA_VERSION,
                event_id: "e1".into(),
                sent_at: "2026-04-21T00:00:00Z".into(),
                correlation_id: None,
                causation_id: None,
            },
            pipeline_id: "pl-1".into(),
            session_id: "s-1".into(),
            opc_instance_id: "opc-1".into(),
            accepted: false,
            decline_reason: Some("knowledge_hash_mismatch".into()),
            observed_at: "2026-04-21T00:00:01Z".into(),
        };
        let json = serde_json::to_value(&frame).unwrap();
        assert_eq!(json["accepted"], false);
        assert_eq!(json["declineReason"], "knowledge_hash_mismatch");
    }

    // spec 111 §2.3 — agent.catalog.{updated,snapshot} must be recognised as
    // known SERVER→CLIENT kinds so the duplex consumer doesn't drop them.
    #[test]
    fn accepts_agent_catalog_kinds_at_v1() {
        for kind in ["agent.catalog.updated", "agent.catalog.snapshot"] {
            assert!(
                is_server_envelope(&empty_envelope(kind, 1)),
                "kind {kind} should pass the guard",
            );
        }
    }

    #[test]
    fn agent_catalog_updated_deserializes_from_wire_json() {
        // Triple-# raw delimiter so the JSON body "# body" (which contains a
        // `"#` sequence) doesn't terminate the Rust raw literal early.
        let raw = r###"{
          "kind": "agent.catalog.updated",
          "meta": {
            "v": 1,
            "eventId": "e-ag",
            "sentAt": "2026-04-22T00:00:00Z",
            "orgCursor": "cur-1",
            "orgId": "org-1"
          },
          "agentId": "a-1",
          "name": "triage",
          "version": 2,
          "status": "published",
          "contentHash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          "frontmatter": {"name": "triage", "extra": {"k": "v"}},
          "bodyMarkdown": "# body",
          "updatedAt": "2026-04-22T00:05:00Z"
        }"###;
        let env: ServerEnvelopeWire = serde_json::from_str(raw).expect("deserialize");
        assert!(is_server_envelope(&env));
        assert_eq!(env.agent_id.as_deref(), Some("a-1"));
        assert_eq!(env.name.as_deref(), Some("triage"));
        assert_eq!(env.version, Some(2));
        assert_eq!(env.status.as_deref(), Some("published"));
        assert_eq!(env.body_markdown.as_deref(), Some("# body"));
        // Frontmatter is decoded as serde_json::Value so the extra flatten
        // keys round-trip opaquely on the desktop side.
        assert_eq!(
            env.frontmatter.as_ref().and_then(|v| v.get("name")),
            Some(&Value::String("triage".into()))
        );
    }

    #[test]
    fn agent_catalog_snapshot_deserializes_directory_entries() {
        let raw = r#"{
          "kind": "agent.catalog.snapshot",
          "meta": {
            "v": 1,
            "eventId": "e-snap",
            "sentAt": "2026-04-22T00:00:00Z",
            "orgCursor": "cur-2",
            "orgId": "org-1"
          },
          "entries": [
            {
              "agentId": "a-1",
              "name": "triage",
              "version": 2,
              "status": "published",
              "contentHash": "aaaa",
              "updatedAt": "2026-04-22T00:05:00Z"
            }
          ],
          "generatedAt": "2026-04-22T00:06:00Z"
        }"#;
        let env: ServerEnvelopeWire = serde_json::from_str(raw).expect("deserialize");
        assert!(is_server_envelope(&env));
        let entries = env.entries.as_ref().expect("entries present");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].agent_id, "a-1");
        assert_eq!(entries[0].status, "published");
        assert_eq!(env.generated_at.as_deref(), Some("2026-04-22T00:06:00Z"));
    }

    #[test]
    fn agent_catalog_fetch_request_serializes_to_camelcase_wire_shape() {
        // Spec 111 §2.3 — reason is a closed set; verify the snake_case
        // serde rename produces the expected wire strings.
        let frame = OutboundFrame::AgentCatalogFetchRequest {
            meta: EnvelopeMeta {
                v: ENVELOPE_SCHEMA_VERSION,
                event_id: "e1".into(),
                sent_at: "2026-04-22T00:00:00Z".into(),
                correlation_id: None,
                causation_id: None,
            },
            agent_id: "a-1".into(),
            reason: AgentCatalogFetchReason::HashMismatch,
            observed_at: "2026-04-22T00:00:01Z".into(),
        };
        let json = serde_json::to_value(&frame).unwrap();
        assert_eq!(json["kind"], "agent.catalog.fetch_request");
        assert_eq!(json["agentId"], "a-1");
        assert_eq!(json["reason"], "hash_mismatch");
        assert_eq!(json["observedAt"], "2026-04-22T00:00:01Z");
        assert_eq!(json["meta"]["v"], 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn send_agent_catalog_fetch_request_emits_frame_when_connected() {
        let inner = Arc::new(SyncClientInner::default());
        let (tx, mut rx) = mpsc::channel::<OutboundFrame>(8);
        inner.set_outbound(Some(tx));

        let sent = inner
            .send_agent_catalog_fetch_request(
                "a-1",
                AgentCatalogFetchReason::CacheMiss,
            )
            .await;
        assert!(sent);

        let frame = rx.recv().await.expect("frame on channel");
        match frame {
            OutboundFrame::AgentCatalogFetchRequest {
                agent_id,
                reason,
                ..
            } => {
                assert_eq!(agent_id, "a-1");
                assert!(matches!(reason, AgentCatalogFetchReason::CacheMiss));
            }
            other => panic!("unexpected frame: {other:?}"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn send_without_active_session_returns_false() {
        // External handlers call send() before the duplex stream connects.
        // The contract is best-effort drop — return false, never block.
        let inner = Arc::new(SyncClientInner::default());
        let sent = inner
            .send_factory_run_ack("pl", "sid", "opc", true, None)
            .await;
        assert!(!sent);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn send_with_active_session_enqueues_on_channel() {
        let inner = Arc::new(SyncClientInner::default());
        let (tx, mut rx) = mpsc::channel::<OutboundFrame>(8);
        inner.set_outbound(Some(tx));

        let sent = inner
            .send_factory_run_ack("pl", "sid", "opc", true, None)
            .await;
        assert!(sent);

        let frame = rx.recv().await.expect("frame on channel");
        match frame {
            OutboundFrame::FactoryRunAck {
                pipeline_id,
                session_id,
                opc_instance_id,
                accepted,
                ..
            } => {
                assert_eq!(pipeline_id, "pl");
                assert_eq!(session_id, "sid");
                assert_eq!(opc_instance_id, "opc");
                assert!(accepted);
            }
            other => panic!("unexpected frame: {other:?}"),
        }
    }
}
