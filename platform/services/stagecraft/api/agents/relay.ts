/**
 * Spec 123 — Agent catalog sync relay (outbound only, org-scoped).
 *
 * Two outbound paths:
 *
 *   publishAgent / retireAgent (catalog.ts)
 *     -> publishAgentCatalogUpdated(row)
 *        -> dispatchServerEvent(orgId, agent.catalog.updated)            // broadcast to org
 *
 *   handshake (duplex.ts)
 *     -> sendAgentCatalogSnapshot(orgId, clientId)                      // targeted
 *        -> sendTargetedServerEvent(orgId, clientId, agent.catalog.snapshot)
 *
 * The snapshot is a directory (hashes only, no bodies) of every published
 * agent in the org. Desktops pull full bodies for cache misses via
 * `agent.catalog.fetch_request` — served by `serveAgentCatalogFetch` in
 * sync/service.ts. This module deliberately owns outbound only.
 */
import log from "encore.dev/log";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  agentCatalog,
  projectAgentBindings,
  projects,
  type AgentCatalogStatus,
} from "../db/schema";
import {
  dispatchServerEvent,
  sendTargetedServerEvent,
} from "../sync/service";
import type {
  AgentCatalogSnapshotEntry,
  ProjectAgentBindingSnapshotEntry,
  ServerAgentCatalogUpdated,
  ServerAgentCatalogSnapshot,
  ServerProjectAgentBindingSnapshot,
  ServerProjectAgentBindingUpdated,
} from "../sync/types";
import type { CatalogFrontmatter } from "./frontmatter";

type AgentRow = typeof agentCatalog.$inferSelect;
type BindingRow = typeof projectAgentBindings.$inferSelect;

// ---------------------------------------------------------------------------
// Envelope construction
// ---------------------------------------------------------------------------

function buildUpdatedEvent(
  row: AgentRow,
): Omit<ServerAgentCatalogUpdated, "meta"> {
  // Spec 111 §2.3: only published/retired rows travel the wire.
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
    orgId: row.orgId,
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
 * agent's org. Called from `publishAgent` and `retireAgent` after their
 * DB transactions commit — order matters, so a failed broadcast never
 * leaves the catalog and the wire out of step.
 */
export async function publishAgentCatalogUpdated(
  row: AgentRow,
): Promise<PublishAgentCatalogUpdatedResult> {
  const event = buildUpdatedEvent(row);
  const result = await dispatchServerEvent(row.orgId, event);
  log.info("agent.catalog: updated broadcast dispatched", {
    orgId: row.orgId,
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
 * Build the directory of currently-published agents in an org. Returns
 * hashes only — the bodies are pulled lazily by the desktop via
 * `agent.catalog.fetch_request` (spec 111 §2.3). Retired agents are
 * excluded so the desktop infers removal from absence.
 */
export async function buildAgentCatalogSnapshotEntries(
  orgId: string,
): Promise<AgentCatalogSnapshotEntry[]> {
  const rows = await db
    .select({
      id: agentCatalog.id,
      orgId: agentCatalog.orgId,
      name: agentCatalog.name,
      version: agentCatalog.version,
      status: agentCatalog.status,
      contentHash: agentCatalog.contentHash,
      updatedAt: agentCatalog.updatedAt,
    })
    .from(agentCatalog)
    .where(
      and(
        eq(agentCatalog.orgId, orgId),
        eq(agentCatalog.status, "published" as AgentCatalogStatus),
      ),
    );

  return rows.map((r) => ({
    agentId: r.id,
    orgId: r.orgId,
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
  orgId: string,
  clientId: string,
): Promise<boolean> {
  const entries = await buildAgentCatalogSnapshotEntries(orgId);
  const event: Omit<ServerAgentCatalogSnapshot, "meta"> = {
    kind: "agent.catalog.snapshot",
    entries,
    generatedAt: new Date().toISOString(),
  };
  const sent = await sendTargetedServerEvent(orgId, clientId, event);
  log.info("agent.catalog: snapshot sent on handshake", {
    orgId,
    clientId,
    entryCount: entries.length,
    delivered: sent,
  });
  return sent;
}

// ---------------------------------------------------------------------------
// Spec 123 §7.2 — Project agent binding broadcasts
// ---------------------------------------------------------------------------

/**
 * Broadcast a binding mutation to every OPC connected to the project's org.
 * Desktop-side filters by `projectId` to apply only to the project the user
 * has active. The fan-out keys on org because that's the duplex session
 * scope; per-project routing is the desktop's responsibility.
 */
export async function publishProjectAgentBindingUpdated(args: {
  orgId: string;
  projectId: string;
  binding: BindingRow;
  agentName: string;
  action: "bound" | "rebound" | "unbound";
}): Promise<{ eventId: string; cursor: string; delivered: number }> {
  const event: Omit<ServerProjectAgentBindingUpdated, "meta"> = {
    kind: "project.agent_binding.updated",
    orgId: args.orgId,
    projectId: args.projectId,
    bindingId: args.binding.id,
    orgAgentId: args.binding.orgAgentId,
    agentName: args.agentName,
    pinnedVersion: args.binding.pinnedVersion,
    pinnedContentHash: args.binding.pinnedContentHash,
    action: args.action,
    boundAt: args.binding.boundAt.toISOString(),
  };
  const result = await dispatchServerEvent(args.orgId, event);
  log.info("agent.binding: updated broadcast dispatched", {
    orgId: args.orgId,
    projectId: args.projectId,
    bindingId: args.binding.id,
    action: args.action,
    delivered: result.delivered,
  });
  return result;
}

/**
 * Build the directory of bindings for one project. Carries no body or
 * frontmatter — the desktop joins entries against the org-wide catalog
 * snapshot to materialise the active agent set.
 */
export async function buildProjectAgentBindingSnapshotEntries(
  projectId: string,
): Promise<ProjectAgentBindingSnapshotEntry[]> {
  const rows = await db
    .select({
      bindingId: projectAgentBindings.id,
      orgAgentId: projectAgentBindings.orgAgentId,
      pinnedVersion: projectAgentBindings.pinnedVersion,
      pinnedContentHash: projectAgentBindings.pinnedContentHash,
      agentName: agentCatalog.name,
    })
    .from(projectAgentBindings)
    .innerJoin(
      agentCatalog,
      eq(agentCatalog.id, projectAgentBindings.orgAgentId),
    )
    .where(eq(projectAgentBindings.projectId, projectId));

  return rows.map((r) => ({
    bindingId: r.bindingId,
    orgAgentId: r.orgAgentId,
    agentName: r.agentName,
    pinnedVersion: r.pinnedVersion,
    pinnedContentHash: r.pinnedContentHash,
  }));
}

/**
 * Send the per-project binding snapshot to one connected client. Called
 * once per project the user has access to, immediately after the catalog
 * snapshot, on handshake or explicit resync.
 */
export async function sendProjectAgentBindingSnapshot(
  orgId: string,
  projectId: string,
  clientId: string,
): Promise<boolean> {
  const bindings = await buildProjectAgentBindingSnapshotEntries(projectId);
  const event: Omit<ServerProjectAgentBindingSnapshot, "meta"> = {
    kind: "project.agent_binding.snapshot",
    orgId,
    projectId,
    bindings,
    generatedAt: new Date().toISOString(),
  };
  const sent = await sendTargetedServerEvent(orgId, clientId, event);
  log.info("agent.binding: snapshot sent on handshake", {
    orgId,
    projectId,
    clientId,
    bindingCount: bindings.length,
    delivered: sent,
  });
  return sent;
}

/**
 * List every project an OPC session needs binding snapshots for. Returns
 * project ids in the connected user's org. Called from the duplex
 * handshake; the resync path uses the same lookup.
 */
export async function listProjectIdsForOrg(orgId: string): Promise<string[]> {
  const rows = await db
    .select({ id: projects.id })
    .from(projects)
    .where(eq(projects.orgId, orgId));
  return rows.map((r) => r.id);
}
