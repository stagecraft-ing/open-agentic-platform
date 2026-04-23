/**
 * Outbox / Inbox Sync Protocol — typed envelopes.
 *
 * Authority boundaries (per spec 087):
 *   - Stagecraft is authoritative for identity, workspace, policy, grants,
 *     deployment/governance state, audit envelopes.
 *   - Desktop/OPC is authoritative for local execution progress, checkpoints,
 *     local agent/tool runs, local runtime observations.
 *
 * Sync is application/event-layer, NOT database replication.
 *
 * Message flow:
 *   client -> server : ClientEnvelope   (inbox path)
 *   server -> client : ServerEnvelope   (outbox path)
 */
import type { StreamInOut } from "encore.dev/api";
import type { CatalogFrontmatter } from "../agents/frontmatter";

// ---------------------------------------------------------------------------
// Handshake
// ---------------------------------------------------------------------------

/** Sent by the client exactly once when opening the duplex stream. */
export interface SyncHandshake {
  /** Caller-generated UUID identifying this connection instance. */
  clientId: string;
  /** What kind of client is connecting. */
  clientKind: "desktop-opc" | "web-ui" | "agent-runner" | "unknown";
  /** Client software version — informational, for compat/debug. */
  clientVersion?: string;
  /**
   * Last server cursor the client observed, if any. The service MAY honour
   * this to replay missed messages once a durable outbox is wired in.
   */
  lastServerCursor?: string;
  /** Optional capabilities declared by the client. */
  capabilities?: string[];
}

// ---------------------------------------------------------------------------
// Common envelope metadata
// ---------------------------------------------------------------------------

/**
 * Envelope schema version.
 *
 * Spec 087 §5.3 FR-SYNC-003: every envelope MUST carry a schema version. The
 * current protocol is version 1; the guard in `isClientEnvelope` rejects any
 * other value. Bumping this is a wire-format change and requires extending
 * both the TypeScript literal and the runtime guard in lock-step.
 */
export type EnvelopeSchemaVersion = 1;
export const ENVELOPE_SCHEMA_VERSION: EnvelopeSchemaVersion = 1;

export interface EnvelopeMeta {
  /** Schema version — required; strict equality enforced at the boundary. */
  v: EnvelopeSchemaVersion;
  /** Unique event ID — UUID, set by sender. Used for ACK/NACK correlation. */
  eventId: string;
  /** ISO-8601 timestamp, set by sender. */
  sentAt: string;
  /** Optional correlation ID linking a response back to a request. */
  correlationId?: string;
  /** Optional causation ID linking this event to the event that produced it. */
  causationId?: string;
}

// ---------------------------------------------------------------------------
// Client -> Server (INBOX) messages
// ---------------------------------------------------------------------------

/**
 * Desktop/OPC-originated events that Stagecraft should record or act on.
 * Each variant is a clearly-bounded desktop-authoritative signal — never a
 * control-plane mutation that would blur authority.
 *
 * INVARIANT (spec 087 §5.3):
 *   Extending this union is a *governance act*, not a types change. Any new
 *   variant MUST carry no control-plane authority (no policy/grant/deploy/
 *   workspace state mutation). A variant that asserts authoritative server
 *   state requires a spec amendment.
 */
export type ClientEnvelope =
  | ClientExecutionStatus
  | ClientCheckpointCreated
  | ClientArtifactEmitted
  | ClientRuntimeObserved
  | ClientAgentInvocation
  | ClientAuditCandidate
  | ClientFactoryRunAck
  | ClientAgentCatalogFetchRequest
  | ClientAck
  | ClientResyncRequest
  | ClientHeartbeat;

export interface ClientExecutionStatus {
  kind: "execution.status";
  meta: EnvelopeMeta;
  projectId: string;
  executionId: string;
  status: "started" | "progress" | "completed" | "failed" | "cancelled";
  progressPct?: number;
  message?: string;
}

export interface ClientCheckpointCreated {
  kind: "checkpoint.created";
  meta: EnvelopeMeta;
  projectId: string;
  checkpointId: string;
  label?: string;
  commitSha?: string;
}

export interface ClientArtifactEmitted {
  kind: "artifact.emitted";
  meta: EnvelopeMeta;
  projectId: string;
  executionId?: string;
  artifactType: string;
  contentHash: string;
  sizeBytes?: number;
  storageRef?: string;
}

export interface ClientRuntimeObserved {
  kind: "runtime.observed";
  meta: EnvelopeMeta;
  projectId?: string;
  observation:
    | "degraded"
    | "recovered"
    | "disk_pressure"
    | "network_loss"
    | "online";
  detail?: string;
}

export interface ClientAgentInvocation {
  kind: "agent.invocation";
  meta: EnvelopeMeta;
  projectId: string;
  agentId: string;
  toolCalls: number;
  durationMs: number;
  outcome: "ok" | "error" | "policy_denied";
  errorMessage?: string;
}

/**
 * Candidate audit events from the desktop. These are NOT final audit records —
 * Stagecraft retains the right to reject, normalise or enrich them before
 * committing. This preserves Stagecraft as the audit authority.
 */
export interface ClientAuditCandidate {
  kind: "audit.candidate";
  meta: EnvelopeMeta;
  action: string;
  targetType: string;
  targetId: string;
  details?: Record<string, unknown>;
}

/**
 * Desktop observation that a `factory.run.request` was received (spec 110 §2.2).
 *
 * This is an OBSERVATION of local intent — it does not assert server state,
 * so it fits within the 087 §5.3 extension rule for `ClientEnvelope`. The
 * `accepted: false` path is how policy/local-state declines surface back to
 * stagecraft.
 */
export interface ClientFactoryRunAck {
  kind: "factory.run.ack";
  meta: EnvelopeMeta;
  /** Correlates to the `pipelineId` on the triggering `factory.run.request`. */
  pipelineId: string;
  /** Tab/execution session that accepted the request (spec 110 §2.4). */
  sessionId: string;
  /** Stable identifier for the OPC process that observed the request. */
  opcInstanceId: string;
  /** False when the desktop saw the request but declined it. */
  accepted: boolean;
  /** Present when `accepted === false`. Free-form, user-surfacable. */
  declineReason?: string;
  /** ISO-8601 timestamp of the observation. */
  observedAt: string;
}

/**
 * Desktop pulls the full body of an agent definition after a
 * `agent.catalog.snapshot` shows a `content_hash` that disagrees with its
 * local cache — or on explicit manual refresh (spec 111 §2.3). The server
 * replies with a targeted `agent.catalog.updated` carrying the full
 * frontmatter + body.
 *
 * This is an observation of local cache state, not a control-plane mutation,
 * so it fits within the 087 §5.3 ClientEnvelope extension rule.
 */
export interface ClientAgentCatalogFetchRequest {
  kind: "agent.catalog.fetch_request";
  meta: EnvelopeMeta;
  /** The workspace the desktop thinks owns the agent — used to cross-check
   *  against the session's authenticated workspaceId. */
  workspaceId: string;
  /** Remote id returned in the snapshot entry. */
  agentId: string;
  reason: "cache_miss" | "hash_mismatch" | "manual_refresh";
  /** ISO-8601 of when the desktop noticed the delta. */
  observedAt: string;
}

/** Client acknowledging a previously-received server event. */
export interface ClientAck {
  kind: "sync.ack";
  meta: EnvelopeMeta;
  /** The server event being acknowledged. */
  serverEventId: string;
}

/**
 * Client requesting a full resync — e.g. after detecting a gap in server
 * cursors, or on reconnect.
 */
export interface ClientResyncRequest {
  kind: "sync.resync_request";
  meta: EnvelopeMeta;
  sinceCursor?: string;
  reason?: string;
}

export interface ClientHeartbeat {
  kind: "sync.heartbeat";
  meta: EnvelopeMeta;
}

// ---------------------------------------------------------------------------
// Server -> Client (OUTBOX) messages
// ---------------------------------------------------------------------------

/**
 * Stagecraft-originated events describing authoritative state changes.
 * Also carries ACK/NACK for inbound client events.
 */
export type ServerEnvelope =
  | ServerPolicyUpdated
  | ServerGrantUpdated
  | ServerDeployStatus
  | ServerWorkspaceUpdated
  | ServerProjectUpdated
  | ServerFactoryEvent
  | ServerFactoryRunRequest
  | ServerAgentCatalogUpdated
  | ServerAgentCatalogSnapshot
  | ServerProjectCatalogUpsert
  | ServerAck
  | ServerNack
  | ServerResyncRequired
  | ServerHeartbeat
  | ServerHello;

export interface ServerMeta extends EnvelopeMeta {
  /**
   * Monotonic cursor issued by the server for outbound events within a
   * workspace. Clients MAY persist this and pass it back as
   * `SyncHandshake.lastServerCursor` on reconnect.
   *
   * This is best-effort in the in-memory implementation; a durable store
   * is required before clients can safely rely on it for replay.
   */
  workspaceCursor: string;
  workspaceId: string;
}

export interface ServerPolicyUpdated {
  kind: "policy.updated";
  meta: ServerMeta;
  policyBundleId: string;
  summary?: string;
}

export interface ServerGrantUpdated {
  kind: "grant.updated";
  meta: ServerMeta;
  userId: string;
  change: "granted" | "revoked" | "modified";
  details?: Record<string, unknown>;
}

export interface ServerDeployStatus {
  kind: "deploy.status";
  meta: ServerMeta;
  projectId: string;
  environmentId: string;
  status: "queued" | "running" | "succeeded" | "failed" | "rolled_back";
  detail?: string;
}

export interface ServerWorkspaceUpdated {
  kind: "workspace.updated";
  meta: ServerMeta;
  change: "renamed" | "members_changed" | "settings_changed";
  details?: Record<string, unknown>;
}

export interface ServerProjectUpdated {
  kind: "project.updated";
  meta: ServerMeta;
  projectId: string;
  change: "created" | "updated" | "deleted" | "repo_linked";
  details?: Record<string, unknown>;
}

/**
 * Factory pipeline events. Carried through the outbox so OPC and web clients
 * observe the same authoritative stream.
 */
export interface ServerFactoryEvent {
  kind: "factory.event";
  meta: ServerMeta;
  pipelineId: string;
  projectId: string;
  eventType: string;
  stageId?: string;
  actor?: string;
  details?: Record<string, unknown>;
}

/**
 * Knowledge-bundle reference carried on a `factory.run.request` (spec 110 §2.3).
 *
 * The desktop resolves each entry against a content-addressable cache at
 * `$OPC_CACHE_DIR/knowledge/<contentHash>` before passing local paths to the
 * factory engine; the hash is the trust boundary (mismatch ⇒ run fails).
 */
export interface KnowledgeBundle {
  /** Knowledge-object UUID on stagecraft. */
  objectId: string;
  /** Suggested local filename (preserves extension for the engine). */
  filename: string;
  /** sha-256 of the object body — authoritative cache key. */
  contentHash: string;
  /** Presigned URL with a short TTL (15 min); regenerated on resync. */
  downloadUrl: string;
}

/**
 * Business-doc reference carried on a `factory.run.request` (spec 110 §2.1).
 *
 * Distinct from the file-local `BusinessDocRef` in `api/factory/factory.ts`
 * (which uses snake_case `storage_ref` to match HTTP shape); this is the
 * wire-level camelCase form used on the envelope.
 */
export interface EnvelopeBusinessDoc {
  name: string;
  storageRef: string;
}

/**
 * Stagecraft directs a connected OPC to start a locally-executed factory run
 * (spec 110 §2.1).
 *
 * Authority invariant (087 §5.3): this IS a control-plane directive. The
 * request originates from an authenticated stagecraft user; the desktop
 * enforces its local policy bundle before acting and replies with
 * `factory.run.ack` (`accepted: false`) when it declines. One OPC accepts;
 * others receive `sync.nack` for the same `meta.eventId`.
 */
export interface ServerFactoryRunRequest {
  kind: "factory.run.request";
  meta: ServerMeta;
  projectId: string;
  pipelineId: string;
  /** One of the KNOWN_ADAPTERS registered with the factory engine. */
  adapter: string;
  /** User who clicked Initialize (or invoked `oap-ctl run factory`). */
  actorUserId: string;
  /** Workspace knowledge objects attached to this run. */
  knowledge: KnowledgeBundle[];
  /** Explicit per-request doc uploads. Same shape as spec 108. */
  businessDocs: EnvelopeBusinessDoc[];
  /** Policy bundle id compiled server-side for this workspace. */
  policyBundleId: string;
  /** ISO-8601 when stagecraft dispatched the request. */
  requestedAt: string;
  /** ISO-8601 after which stagecraft will mark the pipeline `abandoned`. */
  deadlineAt: string;
}

/**
 * Entry shape for {@link ServerAgentCatalogSnapshot}. The snapshot is a
 * directory — names, versions, and content hashes only. Desktops compare
 * each `contentHash` against their local cache and pull missing bodies via
 * {@link ClientAgentCatalogFetchRequest} (spec 111 §2.3).
 */
export interface AgentCatalogSnapshotEntry {
  agentId: string;
  name: string;
  version: number;
  status: "published" | "retired";
  contentHash: string;
  updatedAt: string;
}

/**
 * Stagecraft announces that an agent definition was published or retired
 * (spec 111 §2.3). Carries the full frontmatter + body so connected OPCs
 * can update their local caches in one round-trip. Also used as the
 * targeted reply to a {@link ClientAgentCatalogFetchRequest}.
 */
export interface ServerAgentCatalogUpdated {
  kind: "agent.catalog.updated";
  meta: ServerMeta;
  /** Remote id — stable across versions within a (workspace, name) pair. */
  agentId: string;
  /** Catalog key (kebab-case, unique per workspace). */
  name: string;
  /** Monotonic per (workspace, name). */
  version: number;
  /** `published` puts the agent into the active catalog;
   *  `retired` removes it. Drafts never travel the wire. */
  status: "published" | "retired";
  /** sha-256 over canonical JSON of frontmatter + body (spec 111 §6). */
  contentHash: string;
  /** UnifiedFrontmatter mirrored from crates/agent-frontmatter. */
  frontmatter: CatalogFrontmatter;
  /** The agent's system prompt body. */
  bodyMarkdown: string;
  /** ISO-8601 of the underlying row's updated_at. */
  updatedAt: string;
}

/**
 * Full catalog replay on handshake or explicit resync (spec 111 §2.3). The
 * snapshot is intentionally a directory — `entries` carry hashes only, not
 * bodies. Desktops diff against their local cache and pull individual
 * bodies via {@link ClientAgentCatalogFetchRequest} — this keeps reconnect
 * storms bounded for workspaces with many large-prompt agents.
 */
export interface ServerAgentCatalogSnapshot {
  kind: "agent.catalog.snapshot";
  meta: ServerMeta;
  /** Currently-published entries for the session's workspace. Retired
   *  agents are excluded; the desktop infers removal from absence. */
  entries: AgentCatalogSnapshotEntry[];
  /** ISO-8601 of when the snapshot was built. Informational. */
  generatedAt: string;
}

/**
 * Stagecraft announces that a workspace project was created/updated/deleted
 * (spec 112 §7). Reuses the spec 111 sync pattern: one wire message carries
 * everything OPC needs to surface the row in its "Projects" panel — no
 * secondary round-trip. Deletions are expressed by the `tombstone` flag so
 * desktops can prune local state without reconciling absence.
 */
export interface ServerProjectCatalogUpsert {
  kind: "project.catalog.upsert";
  meta: ServerMeta;
  /** UUID of the workspace projects row. */
  projectId: string;
  workspaceId: string;
  /** Human-friendly display name and URL slug. */
  name: string;
  slug: string;
  description: string;
  /** Factory adapter row id this project is bound to; null for pre-112 projects. */
  factoryAdapterId: string | null;
  /** One of the factory-project-detect levels, as inferred by stagecraft at create/import time. */
  detectionLevel:
    | "not_factory"
    | "scaffold_only"
    | "legacy_produced"
    | "acp_produced"
    | null;
  /** Primary repo metadata — the desktop uses this to clone on first open. */
  repo: {
    githubOrg: string;
    repoName: string;
    defaultBranch: string;
    cloneUrl: string;
    htmlUrl: string;
  } | null;
  /** Canonical oap:// deep link for this project. */
  oapDeepLink: string;
  /** Marks the project as deleted — desktops drop it from local state. */
  tombstone: boolean;
  /** ISO-8601 of the underlying row's updated_at. */
  updatedAt: string;
}

/** Server accepted a client event and recorded it. */
export interface ServerAck {
  kind: "sync.ack";
  meta: ServerMeta;
  /** The client event being acknowledged. */
  clientEventId: string;
}

/** Server rejected a client event — validation/auth/workspace mismatch. */
export interface ServerNack {
  kind: "sync.nack";
  meta: ServerMeta;
  clientEventId: string;
  reason: "invalid" | "unauthorized" | "workspace_mismatch" | "internal_error";
  detail?: string;
}

/** Server informs the client it should perform a full resync. */
export interface ServerResyncRequired {
  kind: "sync.resync_required";
  meta: ServerMeta;
  reason: "cursor_gap" | "stale_cursor" | "server_restart";
}

export interface ServerHeartbeat {
  kind: "sync.heartbeat";
  meta: ServerMeta;
}

/** First message sent by server after handshake — carries session info. */
export interface ServerHello {
  kind: "sync.hello";
  meta: ServerMeta;
  sessionId: string;
  serverStartedAt: string;
  /** Any cursor gap the server detected vs the handshake cursor. */
  cursorGap?: boolean;
}

// ---------------------------------------------------------------------------
// Type guards / discriminators
// ---------------------------------------------------------------------------

export type ClientEnvelopeKind = ClientEnvelope["kind"];
export type ServerEnvelopeKind = ServerEnvelope["kind"];

// ---------------------------------------------------------------------------
// Wire-level interfaces for the Encore streaming boundary
// ---------------------------------------------------------------------------
//
// Encore.ts's schema parser cannot walk a union-typed alias at an API boundary
// (it expects a named interface). To keep the rich discriminated unions for
// internal narrowing, we expose flat "fat" interfaces that enumerate every
// possible field with optional typing. On the wire the JSON is identical —
// optional keys are simply omitted for variants that don't use them.
//
// INVARIANT: every `ClientEnvelope` / `ServerEnvelope` variant must be
// structurally assignable to its wire counterpart. The compile-time
// assertions at the bottom of this block pin that — adding a variant without
// widening the wire interface fails tsc.

/** Flat counterpart of {@link ClientEnvelope} for the Encore stream boundary. */
export interface ClientEnvelopeWire {
  // Kinds are inlined rather than referencing `ClientEnvelopeKind`; Encore's
  // schema parser cannot evaluate indexed-access types over a union alias.
  kind:
    | "execution.status"
    | "checkpoint.created"
    | "artifact.emitted"
    | "runtime.observed"
    | "agent.invocation"
    | "audit.candidate"
    | "factory.run.ack"
    | "agent.catalog.fetch_request"
    | "sync.ack"
    | "sync.resync_request"
    | "sync.heartbeat";
  meta: EnvelopeMeta;
  projectId?: string;
  executionId?: string;
  status?: "started" | "progress" | "completed" | "failed" | "cancelled";
  progressPct?: number;
  message?: string;
  checkpointId?: string;
  label?: string;
  commitSha?: string;
  artifactType?: string;
  contentHash?: string;
  sizeBytes?: number;
  storageRef?: string;
  observation?:
    | "degraded"
    | "recovered"
    | "disk_pressure"
    | "network_loss"
    | "online";
  detail?: string;
  agentId?: string;
  toolCalls?: number;
  durationMs?: number;
  outcome?: "ok" | "error" | "policy_denied";
  errorMessage?: string;
  action?: string;
  targetType?: string;
  targetId?: string;
  details?: Record<string, unknown>;
  serverEventId?: string;
  sinceCursor?: string;
  reason?: string;
  // spec 110 §2.2 — factory.run.ack fields
  pipelineId?: string;
  sessionId?: string;
  opcInstanceId?: string;
  accepted?: boolean;
  declineReason?: string;
  observedAt?: string;
  // spec 111 §2.3 — agent.catalog.fetch_request fields (agentId is already
  // declared above for agent.invocation; reason is already the open string).
  workspaceId?: string;
}

/** Flat counterpart of {@link ServerEnvelope} for the Encore stream boundary. */
export interface ServerEnvelopeWire {
  kind:
    | "policy.updated"
    | "grant.updated"
    | "deploy.status"
    | "workspace.updated"
    | "project.updated"
    | "factory.event"
    | "factory.run.request"
    | "agent.catalog.updated"
    | "agent.catalog.snapshot"
    | "sync.ack"
    | "sync.nack"
    | "sync.resync_required"
    | "sync.heartbeat"
    | "sync.hello";
  meta: ServerMeta;
  policyBundleId?: string;
  summary?: string;
  userId?: string;
  change?:
    | "granted"
    | "revoked"
    | "modified"
    | "renamed"
    | "members_changed"
    | "settings_changed"
    | "created"
    | "updated"
    | "deleted"
    | "repo_linked";
  details?: Record<string, unknown>;
  projectId?: string;
  environmentId?: string;
  status?:
    | "queued"
    | "running"
    | "succeeded"
    | "failed"
    | "rolled_back"
    | "published"
    | "retired";
  detail?: string;
  pipelineId?: string;
  eventType?: string;
  stageId?: string;
  actor?: string;
  clientEventId?: string;
  reason?:
    | "invalid"
    | "unauthorized"
    | "workspace_mismatch"
    | "internal_error"
    | "cursor_gap"
    | "stale_cursor"
    | "server_restart";
  sessionId?: string;
  serverStartedAt?: string;
  cursorGap?: boolean;
  // spec 110 §2.1 — factory.run.request fields
  adapter?: string;
  actorUserId?: string;
  knowledge?: KnowledgeBundle[];
  businessDocs?: EnvelopeBusinessDoc[];
  requestedAt?: string;
  deadlineAt?: string;
  // spec 111 §2.3 — agent.catalog.updated / agent.catalog.snapshot fields
  agentId?: string;
  name?: string;
  version?: number;
  contentHash?: string;
  frontmatter?: CatalogFrontmatter;
  bodyMarkdown?: string;
  updatedAt?: string;
  entries?: AgentCatalogSnapshotEntry[];
  generatedAt?: string;
}

// Compile-time assignability gates: every variant must fit the wire shape.
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const _clientWireAssignable: ClientEnvelopeWire = null as unknown as ClientEnvelope;
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const _serverWireAssignable: ServerEnvelopeWire = null as unknown as ServerEnvelope;

const CLIENT_KINDS: ReadonlySet<ClientEnvelopeKind> = new Set<ClientEnvelopeKind>([
  "execution.status",
  "checkpoint.created",
  "artifact.emitted",
  "runtime.observed",
  "agent.invocation",
  "audit.candidate",
  "factory.run.ack",
  "agent.catalog.fetch_request",
  "sync.ack",
  "sync.resync_request",
  "sync.heartbeat",
]);

export function isClientEnvelope(v: unknown): v is ClientEnvelope {
  if (!v || typeof v !== "object") return false;
  const r = v as { kind?: unknown; meta?: unknown };
  if (typeof r.kind !== "string") return false;
  if (!CLIENT_KINDS.has(r.kind as ClientEnvelopeKind)) return false;
  if (!r.meta || typeof r.meta !== "object") return false;
  const m = r.meta as { eventId?: unknown; sentAt?: unknown; v?: unknown };
  // Spec 087 §5.3 FR-SYNC-003: strict equality on schema version. A newer
  // client sending v=2 is rejected as "invalid" rather than silently falling
  // through a best-effort decoder.
  if (m.v !== ENVELOPE_SCHEMA_VERSION) return false;
  return typeof m.eventId === "string" && typeof m.sentAt === "string";
}

// ---------------------------------------------------------------------------
// Stream alias
// ---------------------------------------------------------------------------

export type SyncStream = StreamInOut<ClientEnvelopeWire, ServerEnvelopeWire>;
