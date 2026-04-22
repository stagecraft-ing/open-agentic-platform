/**
 * Spec 111 Phase 3 — Agent catalog sync relay (outbound only).
 *
 * Two outbound paths:
 *
 *   publishAgent / retireAgent (catalog.ts)
 *     -> publishAgentCatalogUpdated(row)
 *        -> dispatchServerEvent(workspaceId, agent.catalog.updated)      // broadcast
 *
 *   handshake (duplex.ts)
 *     -> sendAgentCatalogSnapshot(workspaceId, clientId)                 // targeted
 *        -> sendTargetedServerEvent(workspaceId, clientId, agent.catalog.snapshot)
 *
 * The snapshot is a directory (hashes only, no bodies). Desktops pull full
 * bodies for cache misses via `agent.catalog.fetch_request` — served by
 * `serveAgentCatalogFetch` in sync/service.ts (layered there to keep the
 * inbound handler close to handleInbound, matching the audit-candidate
 * pattern). This module deliberately owns outbound only.
 */
import log from "encore.dev/log";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  agentCatalog,
  type AgentCatalogStatus,
} from "../db/schema";
import {
  dispatchServerEvent,
  sendTargetedServerEvent,
} from "../sync/service";
import type {
  AgentCatalogSnapshotEntry,
  ServerAgentCatalogUpdated,
  ServerAgentCatalogSnapshot,
} from "../sync/types";
import type { CatalogFrontmatter } from "./frontmatter";

type AgentRow = typeof agentCatalog.$inferSelect;

// ---------------------------------------------------------------------------
// Envelope construction
// ---------------------------------------------------------------------------

function buildUpdatedEvent(
  row: AgentRow,
): Omit<ServerAgentCatalogUpdated, "meta"> {
  // Spec 111 §2.3: only published/retired rows travel the wire. The caller
  // is responsible for never passing a draft; we assert here so a caller
  // bug surfaces as a loud log rather than a silent cache corruption.
  if (row.status !== "published" && row.status !== "retired") {
    log.error("agent.catalog: refusing to relay non-terminal status", {
      agentId: row.id,
      status: row.status,
    });
    throw new Error(
      `cannot relay agent with status=${row.status}; only published|retired are valid`,
    );
  }
  return {
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
}

// ---------------------------------------------------------------------------
// Outbound: publish / retire broadcasts
// ---------------------------------------------------------------------------

export interface PublishAgentCatalogUpdatedResult {
  eventId: string;
  cursor: string;
  delivered: number;
}

/**
 * Broadcast a published or retired agent to every OPC connected to the
 * workspace. Called from `publishAgent` and `retireAgent` after their DB
 * transactions commit — order matters, so a failed broadcast never leaves
 * the catalog and the wire out of step.
 */
export async function publishAgentCatalogUpdated(
  row: AgentRow,
): Promise<PublishAgentCatalogUpdatedResult> {
  const event = buildUpdatedEvent(row);
  const result = await dispatchServerEvent(row.workspaceId, event);
  log.info("agent.catalog: updated broadcast dispatched", {
    workspaceId: row.workspaceId,
    agentId: row.id,
    name: row.name,
    version: row.version,
    status: row.status,
    delivered: result.delivered,
  });
  return result;
}

// ---------------------------------------------------------------------------
// Snapshot construction (directory of currently-published entries)
// ---------------------------------------------------------------------------

/**
 * Build the directory of currently-published agents for a workspace. Returns
 * hashes only — the bodies are pulled lazily by the desktop via
 * `agent.catalog.fetch_request` (spec 111 §2.3). Retired agents are excluded
 * so the desktop infers removal from absence, which matches §2.4.
 */
export async function buildAgentCatalogSnapshotEntries(
  workspaceId: string,
): Promise<AgentCatalogSnapshotEntry[]> {
  const rows = await db
    .select({
      id: agentCatalog.id,
      name: agentCatalog.name,
      version: agentCatalog.version,
      status: agentCatalog.status,
      contentHash: agentCatalog.contentHash,
      updatedAt: agentCatalog.updatedAt,
    })
    .from(agentCatalog)
    .where(
      and(
        eq(agentCatalog.workspaceId, workspaceId),
        eq(agentCatalog.status, "published" as AgentCatalogStatus),
      ),
    );

  return rows.map((r) => ({
    agentId: r.id,
    name: r.name,
    version: r.version,
    status: r.status as "published",
    contentHash: r.contentHash,
    updatedAt: r.updatedAt.toISOString(),
  }));
}

/**
 * Send a targeted `agent.catalog.snapshot` to a single client. Called from
 * the duplex handshake after `sync.hello` so a reconnecting desktop can
 * diff against its local cache before any catalog deltas stream in.
 */
export async function sendAgentCatalogSnapshot(
  workspaceId: string,
  clientId: string,
): Promise<boolean> {
  const entries = await buildAgentCatalogSnapshotEntries(workspaceId);
  const event: Omit<ServerAgentCatalogSnapshot, "meta"> = {
    kind: "agent.catalog.snapshot",
    entries,
    generatedAt: new Date().toISOString(),
  };
  const sent = await sendTargetedServerEvent(workspaceId, clientId, event);
  log.info("agent.catalog: snapshot sent on handshake", {
    workspaceId,
    clientId,
    entryCount: entries.length,
    delivered: sent,
  });
  return sent;
}

