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

export interface EnvelopeMeta {
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
 */
export type ClientEnvelope =
  | ClientExecutionStatus
  | ClientCheckpointCreated
  | ClientArtifactEmitted
  | ClientRuntimeObserved
  | ClientAgentInvocation
  | ClientAuditCandidate
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

const CLIENT_KINDS: ReadonlySet<ClientEnvelopeKind> = new Set<ClientEnvelopeKind>([
  "execution.status",
  "checkpoint.created",
  "artifact.emitted",
  "runtime.observed",
  "agent.invocation",
  "audit.candidate",
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
  const m = r.meta as { eventId?: unknown; sentAt?: unknown };
  return typeof m.eventId === "string" && typeof m.sentAt === "string";
}

// ---------------------------------------------------------------------------
// Stream alias
// ---------------------------------------------------------------------------

export type SyncStream = StreamInOut<ClientEnvelope, ServerEnvelope>;
