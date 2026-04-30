/**
 * Spec 111 Phase 1 (amended by spec 119) — Project-managed Agent Catalog CRUD.
 *
 * Authoritative per-project agent definitions. Drafts mutate in place
 * (bumping `content_hash`); publication promotes a draft to the next
 * version and auto-retires the prior published row for the same
 * (project_id, name). Retirement is a status flip; no hard delete.
 *
 * Phase 1 is CRUD only — the duplex envelopes that push published state
 * to connected OPCs land in Phase 3 (see spec 111 §7 Rollout).
 *
 * Spec 119: agents are project-scoped. Every endpoint either takes a
 * `:projectId` path param or resolves the project via the row's `:id`,
 * and the caller's org is verified against `projects.org_id`.
 */

import { createHash } from "node:crypto";
import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { db } from "../db/drizzle";
import {
  agentCatalog,
  agentCatalogAudit,
  projects,
  type AgentCatalogAuditAction,
  type AgentCatalogStatus,
} from "../db/schema";
import { and, desc, eq, max, ne } from "drizzle-orm";
import type { CatalogFrontmatter } from "./frontmatter";
import { publishAgentCatalogUpdated } from "./relay";

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

// `CatalogFrontmatter` is the ts-rs-mirrored `UnifiedFrontmatter` (crate
// `agent-frontmatter`, spec 054) plus an open index signature for the
// flattened `extra` map — re-exported from `./frontmatter` so the Rust type
// stays the single source of truth (spec 111 §2.1, Phase 2).
export type { CatalogFrontmatter };

export type CatalogAgent = {
  id: string;
  project_id: string;
  name: string;
  version: number;
  status: AgentCatalogStatus;
  frontmatter: CatalogFrontmatter;
  body_markdown: string;
  content_hash: string;
  created_by: string;
  created_at: string;
  updated_at: string;
};

type CreateAgentRequest = {
  projectId: string;
  name: string;
  frontmatter: CatalogFrontmatter;
  body_markdown: string;
};
type CreateAgentResponse = { agent: CatalogAgent };

type ListAgentsRequest = { projectId: string; status?: AgentCatalogStatus };
type ListAgentsResponse = { agents: CatalogAgent[] };

type GetAgentRequest = { id: string };
type GetAgentResponse = { agent: CatalogAgent };

type PatchAgentRequest = {
  id: string;
  frontmatter?: CatalogFrontmatter;
  body_markdown?: string;
  /** Optimistic lock: rejected if the current content_hash doesn't match. */
  expected_content_hash?: string;
};
type PatchAgentResponse = { agent: CatalogAgent };

type PublishAgentRequest = { id: string };
type PublishAgentResponse = { agent: CatalogAgent; retired?: CatalogAgent };

type RetireAgentRequest = { id: string };
type RetireAgentResponse = { agent: CatalogAgent };

type ForkAgentRequest = { id: string; new_name: string };
type ForkAgentResponse = { agent: CatalogAgent };

export type CatalogAuditEntry = {
  id: string;
  agent_id: string;
  project_id: string;
  action: AgentCatalogAuditAction;
  actor_user_id: string;
  before: Record<string, unknown> | null;
  after: Record<string, unknown> | null;
  created_at: string;
};

type ListAgentAuditRequest = { id: string };
type ListAgentAuditResponse = { entries: CatalogAuditEntry[] };

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const KEBAB_CASE = /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/;

/**
 * Canonical JSON: object keys sorted recursively so the hash is stable
 * regardless of the order keys appear in the authoring payload.
 */
function canonicalise(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonicalise);
  if (value && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>);
    entries.sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));
    return entries.reduce<Record<string, unknown>>((acc, [k, v]) => {
      acc[k] = canonicalise(v);
      return acc;
    }, {});
  }
  return value;
}

/**
 * Content-addressable hash. Typed loosely on purpose: the stability
 * invariant (spec 111 §6) is about canonical JSON serialisation of arbitrary
 * object shapes, not about whether the input matches `CatalogFrontmatter`.
 * The catalog API call-sites flow a typed `CatalogFrontmatter` in through
 * this wider signature, which still accepts them via structural subtyping.
 */
export function computeContentHash(
  frontmatter: Record<string, unknown>,
  bodyMarkdown: string,
): string {
  const canon = JSON.stringify({
    frontmatter: canonicalise(frontmatter),
    body_markdown: bodyMarkdown,
  });
  return createHash("sha256").update(canon).digest("hex");
}

type AgentRow = typeof agentCatalog.$inferSelect;

function toWire(row: AgentRow): CatalogAgent {
  return {
    id: row.id,
    project_id: row.projectId,
    name: row.name,
    version: row.version,
    status: row.status,
    frontmatter: row.frontmatter as CatalogFrontmatter,
    body_markdown: row.bodyMarkdown,
    content_hash: row.contentHash,
    created_by: row.createdBy,
    created_at: row.createdAt.toISOString(),
    updated_at: row.updatedAt.toISOString(),
  };
}

/**
 * Snapshot for audit rows: drops `body_markdown` since the full body is
 * reconstructable from the corresponding agent_catalog row at replay time,
 * and the audit trail is kept light for compliance reads.
 */
function auditSnapshot(row: AgentRow): Record<string, unknown> {
  return {
    id: row.id,
    project_id: row.projectId,
    name: row.name,
    version: row.version,
    status: row.status,
    content_hash: row.contentHash,
    frontmatter: row.frontmatter,
    updated_at: row.updatedAt.toISOString(),
  };
}

async function recordAudit(
  entry: {
    agentId: string;
    projectId: string;
    action: AgentCatalogAuditAction;
    actorUserId: string;
    before?: AgentRow | null;
    after?: AgentRow | null;
  },
  tx: typeof db,
): Promise<void> {
  await tx.insert(agentCatalogAudit).values({
    agentId: entry.agentId,
    projectId: entry.projectId,
    action: entry.action,
    actorUserId: entry.actorUserId,
    before: entry.before ? auditSnapshot(entry.before) : null,
    after: entry.after ? auditSnapshot(entry.after) : null,
  });
}

/** Verify the project exists and belongs to the caller's org; throw 404 if not. */
async function verifyProjectInOrg(projectId: string, orgId: string): Promise<void> {
  const [row] = await db
    .select({ id: projects.id })
    .from(projects)
    .where(and(eq(projects.id, projectId), eq(projects.orgId, orgId)))
    .limit(1);
  if (!row) {
    throw APIError.notFound("project not found");
  }
}

function requireOrgAuth(): {
  userId: string;
  orgId: string;
  platformRole: string;
} {
  const auth = getAuthData()!;
  return {
    userId: auth.userID,
    orgId: auth.orgId,
    platformRole: auth.platformRole,
  };
}

function requirePublishRole(platformRole: string) {
  if (platformRole !== "owner" && platformRole !== "admin") {
    throw APIError.permissionDenied(
      "publishing or retiring agents requires org admin",
    );
  }
}

async function loadAgent(
  tx: typeof db,
  id: string,
  projectId: string,
): Promise<AgentRow> {
  const rows = await tx
    .select()
    .from(agentCatalog)
    .where(and(eq(agentCatalog.id, id), eq(agentCatalog.projectId, projectId)))
    .limit(1);
  if (rows.length === 0) {
    throw APIError.notFound("agent not found");
  }
  return rows[0];
}

/** Resolve an agent id → its project, asserting the project lives in the caller's org. */
async function resolveAgentProject(
  agentId: string,
  orgId: string,
): Promise<string> {
  const [row] = await db
    .select({ projectId: agentCatalog.projectId })
    .from(agentCatalog)
    .where(eq(agentCatalog.id, agentId))
    .limit(1);
  if (!row) {
    throw APIError.notFound("agent not found");
  }
  await verifyProjectInOrg(row.projectId, orgId);
  return row.projectId;
}

async function nextVersion(
  tx: typeof db,
  projectId: string,
  name: string,
): Promise<number> {
  const [row] = await tx
    .select({ max: max(agentCatalog.version) })
    .from(agentCatalog)
    .where(
      and(eq(agentCatalog.projectId, projectId), eq(agentCatalog.name, name)),
    );
  return (row?.max ?? 0) + 1;
}

// ---------------------------------------------------------------------------
// Endpoints
// ---------------------------------------------------------------------------

export const createAgent = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/projects/:projectId/agents",
  },
  async (req: CreateAgentRequest): Promise<CreateAgentResponse> => {
    if (!KEBAB_CASE.test(req.name)) {
      throw APIError.invalidArgument(
        `name must be kebab-case (matching ${KEBAB_CASE.source})`,
      );
    }
    if (!req.body_markdown || req.body_markdown.length === 0) {
      throw APIError.invalidArgument("body_markdown is required");
    }

    const { userId, orgId } = requireOrgAuth();
    await verifyProjectInOrg(req.projectId, orgId);
    const hash = computeContentHash(req.frontmatter, req.body_markdown);

    const inserted = await db.transaction(async (tx) => {
      const version = await nextVersion(
        tx as unknown as typeof db,
        req.projectId,
        req.name,
      );
      const [row] = await tx
        .insert(agentCatalog)
        .values({
          projectId: req.projectId,
          name: req.name,
          version,
          status: "draft",
          frontmatter: req.frontmatter,
          bodyMarkdown: req.body_markdown,
          contentHash: hash,
          createdBy: userId,
        })
        .returning();
      await recordAudit(
        {
          agentId: row.id,
          projectId: req.projectId,
          action: "create",
          actorUserId: userId,
          after: row,
        },
        tx as unknown as typeof db,
      );
      return row;
    });

    return { agent: toWire(inserted) };
  },
);

export const listAgents = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/projects/:projectId/agents",
  },
  async (req: ListAgentsRequest): Promise<ListAgentsResponse> => {
    const { orgId } = requireOrgAuth();
    await verifyProjectInOrg(req.projectId, orgId);

    const where = req.status
      ? and(
          eq(agentCatalog.projectId, req.projectId),
          eq(agentCatalog.status, req.status),
        )
      : eq(agentCatalog.projectId, req.projectId);

    const rows = await db
      .select()
      .from(agentCatalog)
      .where(where)
      .orderBy(desc(agentCatalog.updatedAt))
      .limit(500);

    return { agents: rows.map(toWire) };
  },
);

export const getAgent = api(
  { expose: true, auth: true, method: "GET", path: "/api/agents/:id" },
  async (req: GetAgentRequest): Promise<GetAgentResponse> => {
    const { orgId } = requireOrgAuth();
    const projectId = await resolveAgentProject(req.id, orgId);
    const row = await loadAgent(db, req.id, projectId);
    return { agent: toWire(row) };
  },
);

export const patchAgent = api(
  { expose: true, auth: true, method: "PATCH", path: "/api/agents/:id" },
  async (req: PatchAgentRequest): Promise<PatchAgentResponse> => {
    const { userId, orgId } = requireOrgAuth();
    const projectId = await resolveAgentProject(req.id, orgId);

    const updated = await db.transaction(async (tx) => {
      const existing = await loadAgent(tx as unknown as typeof db, req.id, projectId);
      if (existing.status !== "draft") {
        throw APIError.failedPrecondition(
          `only drafts may be edited (agent is ${existing.status})`,
        );
      }
      if (
        req.expected_content_hash !== undefined &&
        req.expected_content_hash !== existing.contentHash
      ) {
        throw APIError.failedPrecondition(
          "content_hash mismatch — the draft was edited by someone else",
        );
      }

      const newFrontmatter =
        req.frontmatter ?? (existing.frontmatter as CatalogFrontmatter);
      const newBody = req.body_markdown ?? existing.bodyMarkdown;
      const newHash = computeContentHash(newFrontmatter, newBody);

      const [row] = await tx
        .update(agentCatalog)
        .set({
          frontmatter: newFrontmatter,
          bodyMarkdown: newBody,
          contentHash: newHash,
          updatedAt: new Date(),
        })
        .where(eq(agentCatalog.id, existing.id))
        .returning();

      await recordAudit(
        {
          agentId: row.id,
          projectId,
          action: "edit",
          actorUserId: userId,
          before: existing,
          after: row,
        },
        tx as unknown as typeof db,
      );
      return row;
    });

    return { agent: toWire(updated) };
  },
);

export const publishAgent = api(
  { expose: true, auth: true, method: "POST", path: "/api/agents/:id/publish" },
  async (req: PublishAgentRequest): Promise<PublishAgentResponse> => {
    const { userId, orgId, platformRole } = requireOrgAuth();
    requirePublishRole(platformRole);
    const projectId = await resolveAgentProject(req.id, orgId);

    const result = await db.transaction(async (tx) => {
      const existing = await loadAgent(tx as unknown as typeof db, req.id, projectId);
      if (existing.status !== "draft") {
        throw APIError.failedPrecondition(
          `only drafts may be published (agent is ${existing.status})`,
        );
      }

      // Auto-retire any currently-published sibling for the same
      // (project, name). Keeps "published" a single active row per name.
      const priorPublished = await tx
        .select()
        .from(agentCatalog)
        .where(
          and(
            eq(agentCatalog.projectId, projectId),
            eq(agentCatalog.name, existing.name),
            eq(agentCatalog.status, "published"),
            ne(agentCatalog.id, existing.id),
          ),
        );

      let retired: AgentRow | null = null;
      for (const prior of priorPublished) {
        const [row] = await tx
          .update(agentCatalog)
          .set({ status: "retired" as AgentCatalogStatus, updatedAt: new Date() })
          .where(eq(agentCatalog.id, prior.id))
          .returning();
        await recordAudit(
          {
            agentId: row.id,
            projectId,
            action: "retire",
            actorUserId: userId,
            before: prior,
            after: row,
          },
          tx as unknown as typeof db,
        );
        retired = row;
      }

      const [published] = await tx
        .update(agentCatalog)
        .set({
          status: "published" as AgentCatalogStatus,
          updatedAt: new Date(),
        })
        .where(eq(agentCatalog.id, existing.id))
        .returning();

      await recordAudit(
        {
          agentId: published.id,
          projectId,
          action: "publish",
          actorUserId: userId,
          before: existing,
          after: published,
        },
        tx as unknown as typeof db,
      );

      return { published, retired };
    });

    // Spec 111 §2.3 Phase 3 (amended by spec 119) — broadcast the
    // terminal-state rows to every OPC connected to the org. The
    // broadcast runs after the transaction commits so a fan-out failure
    // never leaves the catalog ahead of the wire. Emit retired first so
    // a desktop with both envelopes inflight applies "retired → published"
    // in an order its local cache merge semantics already handle.
    if (result.retired) {
      await publishAgentCatalogUpdated(result.retired);
    }
    await publishAgentCatalogUpdated(result.published);

    return {
      agent: toWire(result.published),
      ...(result.retired ? { retired: toWire(result.retired) } : {}),
    };
  },
);

export const retireAgent = api(
  { expose: true, auth: true, method: "POST", path: "/api/agents/:id/retire" },
  async (req: RetireAgentRequest): Promise<RetireAgentResponse> => {
    const { userId, orgId, platformRole } = requireOrgAuth();
    requirePublishRole(platformRole);
    const projectId = await resolveAgentProject(req.id, orgId);

    const retired = await db.transaction(async (tx) => {
      const existing = await loadAgent(tx as unknown as typeof db, req.id, projectId);
      if (existing.status !== "published") {
        throw APIError.failedPrecondition(
          `only published agents may be retired (agent is ${existing.status})`,
        );
      }
      const [row] = await tx
        .update(agentCatalog)
        .set({ status: "retired" as AgentCatalogStatus, updatedAt: new Date() })
        .where(eq(agentCatalog.id, existing.id))
        .returning();
      await recordAudit(
        {
          agentId: row.id,
          projectId,
          action: "retire",
          actorUserId: userId,
          before: existing,
          after: row,
        },
        tx as unknown as typeof db,
      );
      return row;
    });

    await publishAgentCatalogUpdated(retired);

    return { agent: toWire(retired) };
  },
);

export const listAgentAudit = api(
  { expose: true, auth: true, method: "GET", path: "/api/agents/:id/audit" },
  async (req: ListAgentAuditRequest): Promise<ListAgentAuditResponse> => {
    const { orgId } = requireOrgAuth();
    const projectId = await resolveAgentProject(req.id, orgId);
    // Authorise by loading the row under the project scope first — the
    // audit rows themselves carry project_id but leaning on the catalog
    // row guarantees "cannot query audit for an agent you cannot read".
    await loadAgent(db, req.id, projectId);

    const rows = await db
      .select()
      .from(agentCatalogAudit)
      .where(
        and(
          eq(agentCatalogAudit.agentId, req.id),
          eq(agentCatalogAudit.projectId, projectId),
        ),
      )
      .orderBy(desc(agentCatalogAudit.createdAt))
      .limit(500);

    return {
      entries: rows.map((r) => ({
        id: r.id,
        agent_id: r.agentId,
        project_id: r.projectId,
        action: r.action,
        actor_user_id: r.actorUserId,
        before: (r.before as Record<string, unknown> | null) ?? null,
        after: (r.after as Record<string, unknown> | null) ?? null,
        created_at: r.createdAt.toISOString(),
      })),
    };
  },
);

export const forkAgent = api(
  { expose: true, auth: true, method: "POST", path: "/api/agents/:id/fork" },
  async (req: ForkAgentRequest): Promise<ForkAgentResponse> => {
    if (!KEBAB_CASE.test(req.new_name)) {
      throw APIError.invalidArgument(
        `new_name must be kebab-case (matching ${KEBAB_CASE.source})`,
      );
    }
    const { userId, orgId } = requireOrgAuth();
    const projectId = await resolveAgentProject(req.id, orgId);

    const forked = await db.transaction(async (tx) => {
      const source = await loadAgent(tx as unknown as typeof db, req.id, projectId);
      if (source.name === req.new_name) {
        throw APIError.invalidArgument(
          "new_name must differ from the source agent's name",
        );
      }

      const version = await nextVersion(
        tx as unknown as typeof db,
        projectId,
        req.new_name,
      );
      const frontmatter = source.frontmatter as CatalogFrontmatter;
      const hash = computeContentHash(frontmatter, source.bodyMarkdown);

      const [row] = await tx
        .insert(agentCatalog)
        .values({
          projectId,
          name: req.new_name,
          version,
          status: "draft",
          frontmatter,
          bodyMarkdown: source.bodyMarkdown,
          contentHash: hash,
          createdBy: userId,
        })
        .returning();

      await recordAudit(
        {
          agentId: row.id,
          projectId,
          action: "fork",
          actorUserId: userId,
          before: source,
          after: row,
        },
        tx as unknown as typeof db,
      );
      return row;
    });

    return { agent: toWire(forked) };
  },
);
