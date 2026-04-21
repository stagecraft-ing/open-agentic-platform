/**
 * Sync connection registry — workspace + client scoped, in-memory, process-local.
 *
 * Structure: workspaceId -> clientId -> Session
 *
 * This is runtime state only. It is NOT a source of truth. Membership,
 * authority, and audit all live in the Postgres control plane.
 *
 * Horizontal scale-out: when stagecraft runs >1 replica, server-originated
 * events must fan out via PubSub/Redis to all replicas; each replica then
 * consults its own local registry. The service layer is structured so the
 * registry boundary is the only place that needs to learn about that.
 */
import log from "encore.dev/log";
import type { ServerEnvelope, SyncStream } from "./types";

export interface SessionMeta {
  workspaceId: string;
  clientId: string;
  clientKind: string;
  userId: string;
  orgId: string;
  connectedAt: Date;
  lastHeartbeatAt: Date;
  /** Monotonic cursor of the last outbound event sent to this client. */
  lastSentCursor?: string;
}

export interface Session {
  meta: SessionMeta;
  stream: SyncStream;
}

// workspaceId -> clientId -> Session
const workspaces: Map<string, Map<string, Session>> = new Map();

export function register(session: Session): void {
  const { workspaceId, clientId } = session.meta;
  let clients = workspaces.get(workspaceId);
  if (!clients) {
    clients = new Map();
    workspaces.set(workspaceId, clients);
  }

  const existing = clients.get(clientId);
  if (existing) {
    // Replace any stale session for the same clientId. Close the old stream
    // best-effort so the old transport tears down cleanly.
    existing.stream.close().catch(() => undefined);
    log.info("sync: replacing stale session", { workspaceId, clientId });
  }

  clients.set(clientId, session);
  log.info("sync: session registered", {
    workspaceId,
    clientId,
    clientKind: session.meta.clientKind,
    userId: session.meta.userId,
    workspaceSessions: clients.size,
  });
}

export function unregister(workspaceId: string, clientId: string): void {
  const clients = workspaces.get(workspaceId);
  if (!clients) return;
  if (clients.delete(clientId)) {
    log.info("sync: session unregistered", {
      workspaceId,
      clientId,
      remaining: clients.size,
    });
  }
  if (clients.size === 0) workspaces.delete(workspaceId);
}

export function get(workspaceId: string, clientId: string): Session | undefined {
  return workspaces.get(workspaceId)?.get(clientId);
}

export function listWorkspace(workspaceId: string): Session[] {
  const clients = workspaces.get(workspaceId);
  return clients ? Array.from(clients.values()) : [];
}

export function workspaceCount(): number {
  return workspaces.size;
}

export function sessionCount(workspaceId?: string): number {
  if (workspaceId) return workspaces.get(workspaceId)?.size ?? 0;
  let total = 0;
  for (const clients of workspaces.values()) total += clients.size;
  return total;
}

/**
 * Send to a single client in a workspace. Returns true if sent, false if the
 * client is not connected or the send failed (in which case the session is
 * pruned).
 */
export async function sendTo(
  workspaceId: string,
  clientId: string,
  event: ServerEnvelope,
): Promise<boolean> {
  const session = get(workspaceId, clientId);
  if (!session) return false;
  try {
    await session.stream.send(event);
    session.meta.lastSentCursor = event.meta.workspaceCursor;
    return true;
  } catch (err) {
    log.warn("sync: send failed, pruning session", {
      workspaceId,
      clientId,
      err: err instanceof Error ? err.message : String(err),
    });
    unregister(workspaceId, clientId);
    return false;
  }
}

/**
 * Broadcast to every client in a workspace. Optionally exclude one clientId
 * (useful to avoid echoing a client's own event back to it).
 */
export async function broadcastWorkspace(
  workspaceId: string,
  event: ServerEnvelope,
  opts: { excludeClientId?: string } = {},
): Promise<{ sent: number; pruned: number }> {
  const clients = workspaces.get(workspaceId);
  if (!clients || clients.size === 0) return { sent: 0, pruned: 0 };

  let sent = 0;
  let pruned = 0;
  const dead: string[] = [];

  for (const [clientId, session] of clients) {
    if (opts.excludeClientId && clientId === opts.excludeClientId) continue;
    try {
      await session.stream.send(event);
      session.meta.lastSentCursor = event.meta.workspaceCursor;
      sent++;
    } catch (err) {
      dead.push(clientId);
      log.warn("sync: broadcast send failed", {
        workspaceId,
        clientId,
        err: err instanceof Error ? err.message : String(err),
      });
    }
  }

  for (const clientId of dead) {
    unregister(workspaceId, clientId);
    pruned++;
  }

  return { sent, pruned };
}

/** Test-only helper — wipes the registry between tests. */
export function __resetForTests(): void {
  workspaces.clear();
}
