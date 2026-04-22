/**
 * Sync service layer.
 *
 * Boundaries:
 *   - `handleInbound`   : called by the duplex endpoint for each client event.
 *                         Validates, records, ACKs or NACKs.
 *   - `dispatchServerEvent` : called by producers (factory subscriber, policy
 *                         updates, etc) to push an authoritative event to a
 *                         workspace's connected clients.
 *   - `publishAck` / `publishNack` : helpers for ACK/NACK responses.
 *
 * This layer is the only place that talks to both the registry and the
 * outbox/inbox stores, and it is the only place that mints cursors for the
 * outbound path.
 */
import log from "encore.dev/log";
import { randomUUID } from "node:crypto";
import { eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import { agentCatalog, auditLog } from "../db/schema";
import type {
  ClientEnvelope,
  ServerEnvelope,
  ServerAck,
  ServerNack,
  ServerMeta,
  ServerAgentCatalogUpdated,
} from "./types";
import { ENVELOPE_SCHEMA_VERSION } from "./types";
import type { CatalogFrontmatter } from "../agents/frontmatter";

// Distributive Omit so Omit<UnionMember, "meta"> works variant-by-variant.
type DistributiveOmit<T, K extends keyof T> = T extends unknown
  ? Omit<T, K>
  : never;
export type ServerEnvelopeWithoutMeta = DistributiveOmit<ServerEnvelope, "meta">;
import { isClientEnvelope } from "./types";
import * as registry from "./registry";
import { inbox, outbox, cursors } from "./store";

// ---------------------------------------------------------------------------
// Inbound path
// ---------------------------------------------------------------------------

export interface InboundContext {
  workspaceId: string;
  clientId: string;
  userId: string;
  orgId: string;
}

export type InboundResult =
  | { ok: true }
  | { ok: false; reason: ServerNack["reason"]; detail?: string };

export async function handleInbound(
  ctx: InboundContext,
  raw: unknown,
): Promise<InboundResult> {
  if (!isClientEnvelope(raw)) {
    log.warn("sync: rejected malformed client envelope", {
      workspaceId: ctx.workspaceId,
      clientId: ctx.clientId,
    });
    return { ok: false, reason: "invalid", detail: "malformed envelope" };
  }

  const evt: ClientEnvelope = raw;

  // Heartbeats and ACKs get lightweight handling — no inbox write.
  if (evt.kind === "sync.heartbeat") {
    const session = registry.get(ctx.workspaceId, ctx.clientId);
    if (session) session.meta.lastHeartbeatAt = new Date();
    return { ok: true };
  }

  if (evt.kind === "sync.ack") {
    await outbox.markAcked(ctx.workspaceId, evt.serverEventId, ctx.clientId);
    return { ok: true };
  }

  if (evt.kind === "sync.resync_request") {
    await deliverResync(ctx, evt.sinceCursor);
    return { ok: true };
  }

  if (evt.kind === "agent.catalog.fetch_request") {
    // Spec 111 §2.3: reply is a targeted agent.catalog.updated. The
    // authenticated workspaceId from the session wins over whatever the
    // client declared in `evt.workspaceId` — this prevents a cross-workspace
    // probe from leaking entries through a mismatched session.
    if (evt.workspaceId !== ctx.workspaceId) {
      return {
        ok: false,
        reason: "workspace_mismatch",
        detail: "fetch_request workspaceId does not match session",
      };
    }
    const served = await serveAgentCatalogFetch(ctx, evt.agentId);
    return served;
  }

  // For all other kinds, persist + log + audit where appropriate.
  try {
    await inbox.recordInbound({
      workspaceId: ctx.workspaceId,
      clientId: ctx.clientId,
      event: evt,
      status: "accepted",
      receivedAt: new Date(),
    });

    if (evt.kind === "audit.candidate") {
      // Stagecraft remains audit authority: we normalise and commit under the
      // authenticated user. We deliberately DO NOT trust the desktop to pick
      // actor_user_id or the timestamp.
      await db.insert(auditLog).values({
        actorUserId: ctx.userId,
        action: `opc.${evt.action}`,
        targetType: evt.targetType,
        targetId: evt.targetId,
        metadata: {
          ...(evt.details ?? {}),
          clientId: ctx.clientId,
          workspaceId: ctx.workspaceId,
          clientEventId: evt.meta.eventId,
        },
      });
    }

    log.info("sync: inbound accepted", {
      workspaceId: ctx.workspaceId,
      clientId: ctx.clientId,
      kind: evt.kind,
      eventId: evt.meta.eventId,
    });
    return { ok: true };
  } catch (err) {
    log.error("sync: inbound processing failed", {
      workspaceId: ctx.workspaceId,
      clientId: ctx.clientId,
      kind: evt.kind,
      err: err instanceof Error ? err.message : String(err),
    });
    await inbox
      .recordInbound({
        workspaceId: ctx.workspaceId,
        clientId: ctx.clientId,
        event: evt,
        status: "rejected",
        receivedAt: new Date(),
        rejectionReason: "internal_error",
      })
      .catch(() => undefined);
    return { ok: false, reason: "internal_error" };
  }
}

// ---------------------------------------------------------------------------
// ACK / NACK publishing
// ---------------------------------------------------------------------------

function mintMeta(workspaceId: string, correlationId?: string): ServerMeta {
  return {
    v: ENVELOPE_SCHEMA_VERSION,
    eventId: randomUUID(),
    sentAt: new Date().toISOString(),
    correlationId,
    workspaceId,
    workspaceCursor: cursors.next(workspaceId),
  };
}

export async function publishAck(
  ctx: InboundContext,
  clientEventId: string,
): Promise<void> {
  const ack: ServerAck = {
    kind: "sync.ack",
    meta: mintMeta(ctx.workspaceId, clientEventId),
    clientEventId,
  };
  await registry.sendTo(ctx.workspaceId, ctx.clientId, ack);
}

export async function publishNack(
  ctx: InboundContext,
  clientEventId: string,
  reason: ServerNack["reason"],
  detail?: string,
): Promise<void> {
  const nack: ServerNack = {
    kind: "sync.nack",
    meta: mintMeta(ctx.workspaceId, clientEventId),
    clientEventId,
    reason,
    detail,
  };
  await registry.sendTo(ctx.workspaceId, ctx.clientId, nack);
}

// ---------------------------------------------------------------------------
// Outbound dispatch (called by producers)
// ---------------------------------------------------------------------------

/**
 * Dispatch a server-originated event to a workspace. The caller supplies the
 * event without `meta` — this function stamps it with the cursor and IDs,
 * records it in the outbox, and fans it out to connected clients.
 */
export async function dispatchServerEvent(
  workspaceId: string,
  event: ServerEnvelopeWithoutMeta,
  opts: { excludeClientId?: string; correlationId?: string } = {},
): Promise<{ eventId: string; cursor: string; delivered: number }> {
  const meta = mintMeta(workspaceId, opts.correlationId);
  // Cast is safe: we've just minted meta that satisfies ServerMeta for every variant.
  const full = { ...event, meta } as ServerEnvelope;

  await outbox.recordOutbound({
    workspaceId,
    event: full,
    createdAt: new Date(),
    ackedBy: new Set(),
  });

  const { sent } = await registry.broadcastWorkspace(workspaceId, full, {
    excludeClientId: opts.excludeClientId,
  });

  log.info("sync: server event dispatched", {
    workspaceId,
    kind: full.kind,
    eventId: meta.eventId,
    cursor: meta.workspaceCursor,
    delivered: sent,
  });

  return { eventId: meta.eventId, cursor: meta.workspaceCursor, delivered: sent };
}

/**
 * Send a server-originated event to a single connected client (instead of
 * broadcasting). Used for direct replies — e.g. the targeted
 * `agent.catalog.updated` response to an `agent.catalog.fetch_request`
 * (spec 111 §2.3). The targeted send deliberately skips the outbox: the
 * desktop already has a correlation-free path to re-request on reconnect
 * via the snapshot, so durable replay of a single-client reply is wasted.
 */
export async function sendTargetedServerEvent(
  workspaceId: string,
  clientId: string,
  event: ServerEnvelopeWithoutMeta,
  opts: { correlationId?: string } = {},
): Promise<boolean> {
  const meta = mintMeta(workspaceId, opts.correlationId);
  const full = { ...event, meta } as ServerEnvelope;
  return registry.sendTo(workspaceId, clientId, full);
}

// ---------------------------------------------------------------------------
// Agent catalog fetch request (spec 111 §2.3)
// ---------------------------------------------------------------------------

async function serveAgentCatalogFetch(
  ctx: InboundContext,
  agentId: string,
): Promise<InboundResult> {
  const [row] = await db
    .select()
    .from(agentCatalog)
    .where(eq(agentCatalog.id, agentId))
    .limit(1);

  if (!row) {
    return { ok: false, reason: "invalid", detail: "agent not found" };
  }
  if (row.workspaceId !== ctx.workspaceId) {
    // Treat as workspace mismatch to avoid leaking membership across
    // workspaces — same disposition as the declared-workspace check above.
    return {
      ok: false,
      reason: "workspace_mismatch",
      detail: "agent belongs to a different workspace",
    };
  }
  if (row.status === "draft") {
    return {
      ok: false,
      reason: "invalid",
      detail: "agent is a draft; drafts never travel the catalog wire",
    };
  }

  const event: Omit<ServerAgentCatalogUpdated, "meta"> = {
    kind: "agent.catalog.updated",
    agentId: row.id,
    name: row.name,
    version: row.version,
    status: row.status as "published" | "retired",
    contentHash: row.contentHash,
    frontmatter: row.frontmatter as CatalogFrontmatter,
    bodyMarkdown: row.bodyMarkdown,
    updatedAt: row.updatedAt.toISOString(),
  };
  await sendTargetedServerEvent(ctx.workspaceId, ctx.clientId, event);
  return { ok: true };
}

// ---------------------------------------------------------------------------
// Resync delivery
// ---------------------------------------------------------------------------

async function deliverResync(
  ctx: InboundContext,
  sinceCursor: string | undefined,
): Promise<void> {
  const pending = await outbox.loadPendingForClient(
    ctx.workspaceId,
    ctx.clientId,
    sinceCursor,
  );

  log.info("sync: delivering resync", {
    workspaceId: ctx.workspaceId,
    clientId: ctx.clientId,
    pendingCount: pending.length,
    sinceCursor,
  });

  for (const evt of pending) {
    const ok = await registry.sendTo(ctx.workspaceId, ctx.clientId, evt);
    if (!ok) break;
  }
}
