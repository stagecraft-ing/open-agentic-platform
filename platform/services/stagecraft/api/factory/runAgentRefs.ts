// Spec 124 §4.1 / T022 — agent-reference resolver for the reservation path.
//
// Walks a `factory_processes.definition` JSONB blob for embedded
// `AgentReference` instances (the externally-tagged enum from
// `crates/factory-contracts/src/agent_reference.rs`), then resolves each
// reference to a `(org_agent_id, version, content_hash)` triple by reading
// `agent_catalog` and (when `projectId` is supplied) `project_agent_bindings`.
//
// The resolver is intentionally thin: the desktop's `agent_resolver`
// (spec 123 §8.2) does the same lookup at run time, so the platform-side
// resolver is here only to populate `factory_runs.source_shas.agents[]` at
// reservation time. T043's cross-check asserts the desktop's per-run
// resolution matches what the server recorded.
//
// AgentReference JSON shape (externally-tagged, snake_case):
//   { "by_id":          { "org_agent_id": "...", "version": 3 } }
//   { "by_name":        { "name": "stage-cd",   "version": 2 } }
//   { "by_name_latest": { "name": "stage-cd"                  } }
//
// Project-bound runs override the process's declared `by_name_latest` with
// the version pinned in `project_agent_bindings` (spec 124 §4.4 invariant
// I-B2 of spec 123). Bindings whose pinned catalog row is `retired` reject
// the reservation client- and server-side (spec 124 §4.1).

import { and, eq, sql } from "drizzle-orm";
import { db } from "../db/drizzle";
import { agentCatalog, projectAgentBindings } from "../db/schema";
import type { FactoryRunSourceShas } from "../db/schema";
import {
  walkForAgentRefs,
  type AgentRefVariant,
} from "./agentRefWalker";

// Re-export the pure walker so callers that already import from
// `runAgentRefs.ts` keep working. Tests should import the walker
// directly from `./agentRefWalker` to avoid dragging the DB client.
export { walkForAgentRefs };
export type { AgentRefVariant };

/** Spec 122 / spec 123 — minimal agent-identity triple. snake_case keys
 *  are the JSONB shape persisted under `factory_runs.source_shas.agents[]`. */
export interface AgentTriple {
  org_agent_id: string;
  version: number;
  content_hash: string;
}

/**
 * The reservation rejects when a project's binding points at a retired
 * catalog row (spec 123 invariant I-B3). Mapped to a typed
 * `FactoryError::RetiredAgent` on the desktop and to
 * `APIError.failedPrecondition` on the platform. Carrying the triple plus
 * the agent name is what the UI needs to deep-link the user to the
 * project's binding management page.
 */
export class RetiredAgentError extends Error {
  constructor(
    public readonly agentName: string,
    public readonly orgAgentId: string,
    public readonly version: number,
    public readonly projectId: string | null,
  ) {
    super(
      `agent "${agentName}" (${orgAgentId} v${version}) is retired upstream`,
    );
    this.name = "RetiredAgentError";
  }
}

/** Thrown when an `AgentReference` does not resolve against the org
 *  catalog. Surfaced as `APIError.failedPrecondition` so the user gets a
 *  clear "process references unknown agent" message rather than a generic
 *  500. */
export class AgentReferenceNotFoundError extends Error {
  constructor(public readonly summary: string) {
    super(`agent reference not resolvable: ${summary}`);
    this.name = "AgentReferenceNotFoundError";
  }
}

// ---------------------------------------------------------------------------
// DB-bound resolver
// ---------------------------------------------------------------------------

interface ResolveContext {
  orgId: string;
  /** When provided: project_agent_bindings is consulted for `by_name_latest`
   *  variants (spec 124 §4.1 / spec 123 binding-aware run identity). */
  projectId?: string | null;
  /** The `factory_processes.definition` JSONB body. */
  processDefinition: unknown;
}

interface BindingRow {
  orgAgentId: string;
  pinnedVersion: number;
  pinnedContentHash: string;
  status: string;
}

/**
 * Resolve every `AgentReference` in the process definition to a triple
 * suitable for `factory_runs.source_shas.agents[]`.
 *
 * Resolution strategy:
 *   * `by_id` — direct lookup in `agent_catalog`. Mismatched version is a
 *     hard error (the process pinned an exact version that doesn't exist).
 *   * `by_name` — `(org_id, name, version)` lookup; same as `by_id` modulo
 *     the lookup key.
 *   * `by_name_latest` — when `projectId` is set and a binding exists for
 *     `name`, prefer the binding's `pinned_version` / `pinned_content_hash`.
 *     Otherwise pick the highest `published` version in `agent_catalog`.
 *
 * Any binding whose pinned catalog row has `status = 'retired'` rejects
 * the reservation with `RetiredAgentError`.
 */
export async function resolveProcessAgentRefs(
  ctx: ResolveContext,
): Promise<AgentTriple[]> {
  const variants = walkForAgentRefs(ctx.processDefinition);
  if (variants.length === 0) {
    return [];
  }

  // Pre-load project bindings (one query for the whole walk).
  const bindingsByName = new Map<string, BindingRow>();
  if (ctx.projectId) {
    const rows = await db
      .select({
        orgAgentId: projectAgentBindings.orgAgentId,
        pinnedVersion: projectAgentBindings.pinnedVersion,
        pinnedContentHash: projectAgentBindings.pinnedContentHash,
        agentName: agentCatalog.name,
        agentStatus: agentCatalog.status,
      })
      .from(projectAgentBindings)
      .innerJoin(
        agentCatalog,
        and(
          eq(agentCatalog.id, projectAgentBindings.orgAgentId),
          eq(agentCatalog.version, projectAgentBindings.pinnedVersion),
        ),
      )
      .where(eq(projectAgentBindings.projectId, ctx.projectId));
    for (const row of rows) {
      bindingsByName.set(row.agentName, {
        orgAgentId: row.orgAgentId,
        pinnedVersion: row.pinnedVersion,
        pinnedContentHash: row.pinnedContentHash,
        status: row.agentStatus,
      });
    }
  }

  const triples: AgentTriple[] = [];
  for (const variant of variants) {
    if ("by_id" in variant) {
      const wanted = variant.by_id;
      const [row] = await db
        .select()
        .from(agentCatalog)
        .where(
          and(
            eq(agentCatalog.id, wanted.org_agent_id),
            eq(agentCatalog.orgId, ctx.orgId),
            eq(agentCatalog.version, wanted.version),
          ),
        )
        .limit(1);
      if (!row) {
        throw new AgentReferenceNotFoundError(
          `by_id ${wanted.org_agent_id} v${wanted.version}`,
        );
      }
      if (row.status === "retired") {
        throw new RetiredAgentError(
          row.name,
          row.id,
          row.version,
          ctx.projectId ?? null,
        );
      }
      triples.push({
        org_agent_id: row.id,
        version: row.version,
        content_hash: row.contentHash,
      });
      continue;
    }
    if ("by_name" in variant) {
      const wanted = variant.by_name;
      const [row] = await db
        .select()
        .from(agentCatalog)
        .where(
          and(
            eq(agentCatalog.name, wanted.name),
            eq(agentCatalog.orgId, ctx.orgId),
            eq(agentCatalog.version, wanted.version),
          ),
        )
        .limit(1);
      if (!row) {
        throw new AgentReferenceNotFoundError(
          `by_name ${wanted.name} v${wanted.version}`,
        );
      }
      if (row.status === "retired") {
        throw new RetiredAgentError(
          row.name,
          row.id,
          row.version,
          ctx.projectId ?? null,
        );
      }
      triples.push({
        org_agent_id: row.id,
        version: row.version,
        content_hash: row.contentHash,
      });
      continue;
    }
    // by_name_latest
    const name = variant.by_name_latest.name;
    const binding = bindingsByName.get(name);
    if (binding) {
      // Project-bound resolution. Reject if the pinned catalog row is
      // retired upstream (spec 123 I-B3 / spec 124 §4.1).
      if (binding.status === "retired") {
        throw new RetiredAgentError(
          name,
          binding.orgAgentId,
          binding.pinnedVersion,
          ctx.projectId ?? null,
        );
      }
      triples.push({
        org_agent_id: binding.orgAgentId,
        version: binding.pinnedVersion,
        content_hash: binding.pinnedContentHash,
      });
      continue;
    }
    // Ad-hoc resolution — pick the highest published version.
    const rows = await db
      .select()
      .from(agentCatalog)
      .where(
        and(
          eq(agentCatalog.orgId, ctx.orgId),
          eq(agentCatalog.name, name),
          eq(agentCatalog.status, "published"),
        ),
      )
      .orderBy(sql`${agentCatalog.version} DESC`)
      .limit(1);
    const top = rows[0];
    if (!top) {
      throw new AgentReferenceNotFoundError(
        `by_name_latest ${name} (no published versions)`,
      );
    }
    triples.push({
      org_agent_id: top.id,
      version: top.version,
      content_hash: top.contentHash,
    });
  }
  return triples;
}

/** Convenience: build the `source_shas` JSONB body for a reservation. */
export function buildSourceShas(args: {
  adapterSha: string;
  processSha: string;
  contracts: Record<string, string>;
  agents: AgentTriple[];
}): FactoryRunSourceShas {
  return {
    adapter: args.adapterSha,
    process: args.processSha,
    contracts: args.contracts,
    agents: args.agents,
  };
}
