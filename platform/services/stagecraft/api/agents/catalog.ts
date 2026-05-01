/**
 * Spec 123: agents are org-scoped; projects consume via bindings.
 *
 * This module owns the org-scoped agent catalog CRUD. Drafts mutate in
 * place (bumping `content_hash`); publication promotes a draft to the next
 * version and auto-retires the prior published row for the same
 * (org_id, name). Retirement is a status flip; no hard delete.
 *
 * Project consumption (bind / repin / unbind) lives in `bindings.ts`.
 * Duplex broadcast lives in `relay.ts` and now carries `orgId` directly
 * on the wire payload (spec 123 §7.1).
 */

import { createHash } from "node:crypto";
import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { db } from "../db/drizzle";
import {
  agentCatalog,
  agentCatalogAudit,
  organizations,
  type AgentCatalogAuditAction,
  type AgentCatalogStatus,
} from "../db/schema";
import { and, desc, eq, max, ne } from "drizzle-orm";
import type { CatalogFrontmatter } from "./frontmatter";
import { publishAgentCatalogUpdated } from "./relay";

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

export type { CatalogFrontmatter };

export type CatalogAgent = {
  id: string;
  org_id: string;
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
  orgId: string;
  name: string;
  frontmatter: CatalogFrontmatter;
  body_markdown: string;
};
type CreateAgentResponse = { agent: CatalogAgent };

type ListAgentsRequest = { orgId: string; status?: AgentCatalogStatus };
type ListAgentsResponse = { agents: CatalogAgent[] };

type GetAgentRequest = { orgId: string; id: string };
type GetAgentResponse = { agent: CatalogAgent };

type PatchAgentRequest = {
  orgId: string;
  id: string;
  frontmatter?: CatalogFrontmatter;
  body_markdown?: string;
  /** Optimistic lock: rejected if the current content_hash doesn't match. */
  expected_content_hash?: string;
};
type PatchAgentResponse = { agent: CatalogAgent };

type PublishAgentRequest = { orgId: string; id: string };
type PublishAgentResponse = { agent: CatalogAgent; retired?: CatalogAgent };

type RetireAgentRequest = { orgId: string; id: string };
type RetireAgentResponse = { agent: CatalogAgent };

type ForkAgentRequest = { orgId: string; id: string; new_name: string };
type ForkAgentResponse = { agent: CatalogAgent };

export type CatalogAuditEntry = {
  id: string;
  agent_id: string;
  org_id: string;
  action: AgentCatalogAuditAction;
  actor_user_id: string;
  before: Record<string, unknown> | null;
  after: Record<string, unknown> | null;
  created_at: string;
};

type ListAgentAuditRequest = { orgId: string; id: string };
type ListAgentAuditResponse = { entries: CatalogAuditEntry[] };

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const KEBAB_CASE = /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/;

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
    org_id: row.orgId,
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

function auditSnapshot(row: AgentRow): Record<string, unknown> {
  return {
    id: row.id,
    org_id: row.orgId,
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
    orgId: string;
    action: AgentCatalogAuditAction;
    actorUserId: string;
    before?: AgentRow | null;
    after?: AgentRow | null;
  },
  tx: typeof db,
): Promise<void> {
  await tx.insert(agentCatalogAudit).values({
    agentId: entry.agentId,
    orgId: entry.orgId,
    action: entry.action,
    actorUserId: entry.actorUserId,
    before: entry.before ? auditSnapshot(entry.before) : null,
    after: entry.after ? auditSnapshot(entry.after) : null,
  });
}

/**
 * Verify the org id on the path matches the caller's authenticated org.
 * Replaces the prior project-scoped `verifyProjectInOrg` (spec 123 T030).
 */
async function verifyOrgAccess(orgId: string, callerOrgId: string): Promise<void> {
  if (orgId !== callerOrgId) {
    throw APIError.permissionDenied(
      "agent catalog access is restricted to the caller's org",
    );
  }
  // Cheap existence check so a typo'd path returns 404 rather than 200/empty.
  const [row] = await db
    .select({ id: organizations.id })
    .from(organizations)
    .where(eq(organizations.id, orgId))
    .limit(1);
  if (!row) {
    throw APIError.notFound("org not found");
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
  orgId: string,
): Promise<AgentRow> {
  const rows = await tx
    .select()
    .from(agentCatalog)
    .where(and(eq(agentCatalog.id, id), eq(agentCatalog.orgId, orgId)))
    .limit(1);
  if (rows.length === 0) {
    throw APIError.notFound("agent not found");
  }
  return rows[0];
}

async function nextVersion(
  tx: typeof db,
  orgId: string,
  name: string,
): Promise<number> {
  const [row] = await tx
    .select({ max: max(agentCatalog.version) })
    .from(agentCatalog)
    .where(
      and(eq(agentCatalog.orgId, orgId), eq(agentCatalog.name, name)),
    );
  return (row?.max ?? 0) + 1;
}

// ---------------------------------------------------------------------------
// Endpoints (spec 123 §5.1)
// ---------------------------------------------------------------------------

export const createAgent = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/orgs/:orgId/agents",
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
    await verifyOrgAccess(req.orgId, orgId);
    const hash = computeContentHash(req.frontmatter, req.body_markdown);

    const inserted = await db.transaction(async (tx) => {
      const version = await nextVersion(
        tx as unknown as typeof db,
        req.orgId,
        req.name,
      );
      const [row] = await tx
        .insert(agentCatalog)
        .values({
          orgId: req.orgId,
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
          orgId: req.orgId,
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
    path: "/api/orgs/:orgId/agents",
  },
  async (req: ListAgentsRequest): Promise<ListAgentsResponse> => {
    const { orgId } = requireOrgAuth();
    await verifyOrgAccess(req.orgId, orgId);

    const where = req.status
      ? and(
          eq(agentCatalog.orgId, req.orgId),
          eq(agentCatalog.status, req.status),
        )
      : eq(agentCatalog.orgId, req.orgId);

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
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/orgs/:orgId/agents/:id",
  },
  async (req: GetAgentRequest): Promise<GetAgentResponse> => {
    const { orgId } = requireOrgAuth();
    await verifyOrgAccess(req.orgId, orgId);
    const row = await loadAgent(db, req.id, req.orgId);
    return { agent: toWire(row) };
  },
);

export const patchAgent = api(
  {
    expose: true,
    auth: true,
    method: "PATCH",
    path: "/api/orgs/:orgId/agents/:id",
  },
  async (req: PatchAgentRequest): Promise<PatchAgentResponse> => {
    const { userId, orgId } = requireOrgAuth();
    await verifyOrgAccess(req.orgId, orgId);

    const updated = await db.transaction(async (tx) => {
      const existing = await loadAgent(
        tx as unknown as typeof db,
        req.id,
        req.orgId,
      );
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
          orgId: req.orgId,
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
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/orgs/:orgId/agents/:id/publish",
  },
  async (req: PublishAgentRequest): Promise<PublishAgentResponse> => {
    const { userId, orgId, platformRole } = requireOrgAuth();
    requirePublishRole(platformRole);
    await verifyOrgAccess(req.orgId, orgId);

    const result = await db.transaction(async (tx) => {
      const existing = await loadAgent(
        tx as unknown as typeof db,
        req.id,
        req.orgId,
      );
      if (existing.status !== "draft") {
        throw APIError.failedPrecondition(
          `only drafts may be published (agent is ${existing.status})`,
        );
      }

      // Auto-retire any currently-published sibling for the same
      // (org_id, name). Keeps "published" a single active row per name.
      const priorPublished = await tx
        .select()
        .from(agentCatalog)
        .where(
          and(
            eq(agentCatalog.orgId, req.orgId),
            eq(agentCatalog.name, existing.name),
            eq(agentCatalog.status, "published"),
            ne(agentCatalog.id, existing.id),
          ),
        );

      let retired: AgentRow | null = null;
      for (const prior of priorPublished) {
        const [row] = await tx
          .update(agentCatalog)
          .set({
            status: "retired" as AgentCatalogStatus,
            updatedAt: new Date(),
          })
          .where(eq(agentCatalog.id, prior.id))
          .returning();
        await recordAudit(
          {
            agentId: row.id,
            orgId: req.orgId,
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
          orgId: req.orgId,
          action: "publish",
          actorUserId: userId,
          before: existing,
          after: published,
        },
        tx as unknown as typeof db,
      );

      return { published, retired };
    });

    // Spec 123 §7.1 — broadcast org-keyed envelopes after the transaction
    // commits so a fan-out failure never leaves the catalog ahead of the
    // wire. Emit retired first so a desktop with both envelopes inflight
    // applies "retired → published" in an order its merge semantics handle.
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
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/orgs/:orgId/agents/:id/retire",
  },
  async (req: RetireAgentRequest): Promise<RetireAgentResponse> => {
    const { userId, orgId, platformRole } = requireOrgAuth();
    requirePublishRole(platformRole);
    await verifyOrgAccess(req.orgId, orgId);

    const retired = await db.transaction(async (tx) => {
      const existing = await loadAgent(
        tx as unknown as typeof db,
        req.id,
        req.orgId,
      );
      if (existing.status !== "published") {
        throw APIError.failedPrecondition(
          `only published agents may be retired (agent is ${existing.status})`,
        );
      }
      const [row] = await tx
        .update(agentCatalog)
        .set({
          status: "retired" as AgentCatalogStatus,
          updatedAt: new Date(),
        })
        .where(eq(agentCatalog.id, existing.id))
        .returning();
      await recordAudit(
        {
          agentId: row.id,
          orgId: req.orgId,
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
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/orgs/:orgId/agents/:id/audit",
  },
  async (req: ListAgentAuditRequest): Promise<ListAgentAuditResponse> => {
    const { orgId } = requireOrgAuth();
    await verifyOrgAccess(req.orgId, orgId);
    // Authorise by loading the catalog row under the org scope first — the
    // audit rows themselves carry org_id but leaning on the catalog row
    // guarantees "cannot query audit for an agent you cannot read."
    await loadAgent(db, req.id, req.orgId);

    const rows = await db
      .select()
      .from(agentCatalogAudit)
      .where(
        and(
          eq(agentCatalogAudit.agentId, req.id),
          eq(agentCatalogAudit.orgId, req.orgId),
        ),
      )
      .orderBy(desc(agentCatalogAudit.createdAt))
      .limit(500);

    return {
      entries: rows.map((r) => ({
        id: r.id,
        agent_id: r.agentId,
        org_id: r.orgId,
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
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/orgs/:orgId/agents/:id/fork",
  },
  async (req: ForkAgentRequest): Promise<ForkAgentResponse> => {
    if (!KEBAB_CASE.test(req.new_name)) {
      throw APIError.invalidArgument(
        `new_name must be kebab-case (matching ${KEBAB_CASE.source})`,
      );
    }
    const { userId, orgId, platformRole } = requireOrgAuth();
    requirePublishRole(platformRole);
    await verifyOrgAccess(req.orgId, orgId);

    const forked = await db.transaction(async (tx) => {
      const source = await loadAgent(
        tx as unknown as typeof db,
        req.id,
        req.orgId,
      );
      if (source.name === req.new_name) {
        throw APIError.invalidArgument(
          "new_name must differ from the source agent's name",
        );
      }

      const version = await nextVersion(
        tx as unknown as typeof db,
        req.orgId,
        req.new_name,
      );
      const frontmatter = source.frontmatter as CatalogFrontmatter;
      const hash = computeContentHash(frontmatter, source.bodyMarkdown);

      const [row] = await tx
        .insert(agentCatalog)
        .values({
          orgId: req.orgId,
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
          orgId: req.orgId,
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
