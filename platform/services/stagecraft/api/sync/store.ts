/**
 * Outbox / Inbox store interfaces.
 *
 * The current implementation is in-memory and best-effort. The interfaces
 * below exist specifically so durable Postgres-backed storage can be swapped
 * in without touching the streaming endpoint or the service layer.
 *
 * NOTE: none of the methods here persist across stagecraft restarts.
 */
import log from "encore.dev/log";
import type { ClientEnvelope, ServerEnvelope } from "./types";

// ---------------------------------------------------------------------------
// Inbox — client-originated events recorded by the server
// ---------------------------------------------------------------------------

export type InboundStatus = "accepted" | "rejected";

export interface InboundRecord {
  workspaceId: string;
  clientId: string;
  event: ClientEnvelope;
  status: InboundStatus;
  receivedAt: Date;
  rejectionReason?: string;
}

export interface InboxStore {
  /** Record an inbound client event (accepted or rejected). */
  recordInbound(record: InboundRecord): Promise<void>;
  /** Inspect recent records (debug / tests). */
  listRecent(limit?: number): Promise<InboundRecord[]>;
}

// ---------------------------------------------------------------------------
// Outbox — server-originated events queued for a workspace
// ---------------------------------------------------------------------------

export interface OutboundRecord {
  workspaceId: string;
  event: ServerEnvelope;
  createdAt: Date;
  /** Client IDs that have ACKed this event. */
  ackedBy: Set<string>;
}

export interface OutboxStore {
  /** Record a server event (for possible replay on reconnect). */
  recordOutbound(record: OutboundRecord): Promise<void>;
  /** Load pending events for a client since a cursor. */
  loadPendingForClient(
    workspaceId: string,
    clientId: string,
    sinceCursor?: string,
  ): Promise<ServerEnvelope[]>;
  /** Mark that a client ACKed a specific event. */
  markAcked(
    workspaceId: string,
    serverEventId: string,
    clientId: string,
  ): Promise<void>;
}

// ---------------------------------------------------------------------------
// Cursor issuer — monotonic per workspace
// ---------------------------------------------------------------------------

export interface CursorIssuer {
  next(workspaceId: string): string;
  peek(workspaceId: string): string | undefined;
}

// ---------------------------------------------------------------------------
// In-memory implementations (MVP)
// ---------------------------------------------------------------------------

const MAX_INBOX_HISTORY = 1_000;
const MAX_OUTBOX_PER_WORKSPACE = 500;

class InMemoryInbox implements InboxStore {
  private readonly ring: InboundRecord[] = [];

  async recordInbound(record: InboundRecord): Promise<void> {
    this.ring.push(record);
    if (this.ring.length > MAX_INBOX_HISTORY) {
      this.ring.splice(0, this.ring.length - MAX_INBOX_HISTORY);
    }
  }

  async listRecent(limit = 50): Promise<InboundRecord[]> {
    return this.ring.slice(-limit);
  }

  __reset(): void {
    this.ring.length = 0;
  }
}

class InMemoryOutbox implements OutboxStore {
  // workspaceId -> ordered records
  private readonly byWorkspace: Map<string, OutboundRecord[]> = new Map();

  async recordOutbound(record: OutboundRecord): Promise<void> {
    let bucket = this.byWorkspace.get(record.workspaceId);
    if (!bucket) {
      bucket = [];
      this.byWorkspace.set(record.workspaceId, bucket);
    }
    bucket.push(record);
    if (bucket.length > MAX_OUTBOX_PER_WORKSPACE) {
      bucket.splice(0, bucket.length - MAX_OUTBOX_PER_WORKSPACE);
    }
  }

  async loadPendingForClient(
    workspaceId: string,
    clientId: string,
    sinceCursor?: string,
  ): Promise<ServerEnvelope[]> {
    const bucket = this.byWorkspace.get(workspaceId);
    if (!bucket) return [];

    // Find the starting index: first record whose cursor is strictly > sinceCursor.
    let startIdx = 0;
    if (sinceCursor !== undefined) {
      const idx = bucket.findIndex((r) => r.event.meta.workspaceCursor === sinceCursor);
      startIdx = idx >= 0 ? idx + 1 : 0;
    }

    const pending: ServerEnvelope[] = [];
    for (let i = startIdx; i < bucket.length; i++) {
      const rec = bucket[i];
      if (!rec.ackedBy.has(clientId)) pending.push(rec.event);
    }
    return pending;
  }

  async markAcked(
    workspaceId: string,
    serverEventId: string,
    clientId: string,
  ): Promise<void> {
    const bucket = this.byWorkspace.get(workspaceId);
    if (!bucket) return;
    for (const rec of bucket) {
      if (rec.event.meta.eventId === serverEventId) {
        rec.ackedBy.add(clientId);
        return;
      }
    }
    log.debug("sync: ack for unknown server event", {
      workspaceId,
      serverEventId,
      clientId,
    });
  }

  __reset(): void {
    this.byWorkspace.clear();
  }
}

class MonotonicCursorIssuer implements CursorIssuer {
  private readonly counters: Map<string, bigint> = new Map();

  next(workspaceId: string): string {
    const prev = this.counters.get(workspaceId) ?? 0n;
    const curr = prev + 1n;
    this.counters.set(workspaceId, curr);
    // Pad so lexicographic ordering matches numeric ordering up to 10^18.
    return curr.toString().padStart(19, "0");
  }

  peek(workspaceId: string): string | undefined {
    const v = this.counters.get(workspaceId);
    return v === undefined ? undefined : v.toString().padStart(19, "0");
  }

  __reset(): void {
    this.counters.clear();
  }
}

// ---------------------------------------------------------------------------
// Singletons (MVP). Swap these for durable stores in a follow-up spec.
// ---------------------------------------------------------------------------

export const inbox: InboxStore & { __reset(): void } = new InMemoryInbox();
export const outbox: OutboxStore & { __reset(): void } = new InMemoryOutbox();
export const cursors: CursorIssuer & { __reset(): void } =
  new MonotonicCursorIssuer();

/** Test-only helper — clears all in-memory state. */
export function __resetStoresForTests(): void {
  inbox.__reset();
  outbox.__reset();
  cursors.__reset();
}
