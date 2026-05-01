/**
 * Spec 123 §5.2 — Project agent bindings.
 *
 * Projects consume org-managed agents via this module. A binding pins
 * one org agent at one immutable version per (project, agent name); the
 * binding row carries `pinned_version` and `pinned_content_hash` only —
 * no override of the agent definition (spec invariant I-B1).
 *
 * Endpoints:
 *   GET    /api/projects/:projectId/agents             — list bindings
 *   POST   /api/projects/:projectId/agents/bind        — { org_agent_id, version }
 *   PATCH  /api/projects/:projectId/agents/:bindingId  — { version } repin
 *   DELETE /api/projects/:projectId/agents/:bindingId  — unbind
 *
 * Audit: each mutation writes an `audit_log` row keyed by
 * `agent.binding_created` / `agent.binding_repinned` / `agent.binding_unbound`
 * (spec 123 T003).
 */

import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { and, desc, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  agentCatalog,
  auditLog,
  projectAgentBindings,
  projects,
  type AgentBindingAuditAction,
  type AgentCatalogStatus,
} from "../db/schema";
import { publishProjectAgentBindingUpdated } from "./relay";

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

export type ProjectAgentBinding = {
  binding_id: string;
  project_id: string;
  org_agent_id: string;
  agent_name: string;
  pinned_version: number;
  pinned_content_hash: string;
  /** Status of the catalog row this binding points at. `retired_upstream`
   *  is surfaced when the catalog row was retired AFTER bind time
   *  (spec 123 invariant I-B3 — bindings stay visible read-only). */
  status: "active" | "retired_upstream";
  bound_by: string;
  bound_at: string;
};

type ListBindingsRequest = { projectId: string };
type ListBindingsResponse = { bindings: ProjectAgentBinding[] };

type BindAgentRequest = {
  projectId: string;
  org_agent_id: string;
  /** Version on the org agent to pin. Server resolves to content_hash. */
  version: number;
};
type BindAgentResponse = { binding: ProjectAgentBinding };

type RepinBindingRequest = {
  projectId: string;
  bindingId: string;
  /** New version to pin. */
  version: number;
};
type RepinBindingResponse = { binding: ProjectAgentBinding };

type UnbindRequest = { projectId: string; bindingId: string };
type UnbindResponse = { ok: true };

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function requireOrgAuth(): { userId: string; orgId: string } {
  const auth = getAuthData()!;
  return { userId: auth.userID, orgId: auth.orgId };
}

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

type CatalogRow = typeof agentCatalog.$inferSelect;
type BindingRow = typeof projectAgentBindings.$inferSelect;

/**
 * Resolve an (org_agent_id, version) pair to the catalog row, asserting
 * the agent belongs to `orgId` and is not in `draft` status (drafts cannot
 * be bound — only published or retired definitions). Returns the row so
 * the caller can read content_hash.
 */
async function resolveCatalogRow(
  orgAgentId: string,
  version: number,
  orgId: string,
): Promise<CatalogRow> {
  const [row] = await db
    .select()
    .from(agentCatalog)
    .where(
      and(
        eq(agentCatalog.id, orgAgentId),
        eq(agentCatalog.version, version),
        eq(agentCatalog.orgId, orgId),
      ),
    )
    .limit(1);
  if (!row) {
    throw APIError.notFound(
      `agent ${orgAgentId} v${version} not found in org`,
    );
  }
  if (row.status === ("draft" as AgentCatalogStatus)) {
    throw APIError.failedPrecondition(
      "cannot bind to a draft — publish first",
    );
  }
  if (row.status === ("retired" as AgentCatalogStatus)) {
    // Spec 123 §5.2 — repinning to a retired version is rejected; the
    // initial bind path also prohibits retired targets so a project can't
    // start out pinned to a retired-upstream definition.
    throw APIError.failedPrecondition(
      `cannot bind to retired version v${version}; choose a published version`,
    );
  }
  return row;
}

/**
 * The org_agent_id may resolve to ANY version row of an agent name. To
 * find the specific row at a given version we look it up by (orgId, name,
 * version). The caller passes the org_agent_id (any version's id) — we
 * read its name first, then resolve the target row.
 */
async function resolveTargetByNameVersion(
  orgAgentIdHint: string,
  version: number,
  orgId: string,
): Promise<CatalogRow> {
  const [hint] = await db
    .select({ name: agentCatalog.name, orgId: agentCatalog.orgId })
    .from(agentCatalog)
    .where(eq(agentCatalog.id, orgAgentIdHint))
    .limit(1);
  if (!hint) {
    throw APIError.notFound("org_agent_id not found");
  }
  if (hint.orgId !== orgId) {
    throw APIError.permissionDenied("agent belongs to a different org");
  }

  const [row] = await db
    .select()
    .from(agentCatalog)
    .where(
      and(
        eq(agentCatalog.orgId, orgId),
        eq(agentCatalog.name, hint.name),
        eq(agentCatalog.version, version),
      ),
    )
    .limit(1);
  if (!row) {
    throw APIError.notFound(
      `agent ${hint.name} v${version} not found in org`,
    );
  }
  if (row.status === ("draft" as AgentCatalogStatus)) {
    throw APIError.failedPrecondition(
      "cannot bind to a draft — publish first",
    );
  }
  if (row.status === ("retired" as AgentCatalogStatus)) {
    throw APIError.failedPrecondition(
      `cannot bind to retired version v${version}; choose a published version`,
    );
  }
  return row;
}

async function loadBinding(
  bindingId: string,
  projectId: string,
): Promise<BindingRow> {
  const [row] = await db
    .select()
    .from(projectAgentBindings)
    .where(
      and(
        eq(projectAgentBindings.id, bindingId),
        eq(projectAgentBindings.projectId, projectId),
      ),
    )
    .limit(1);
  if (!row) {
    throw APIError.notFound("binding not found");
  }
  return row;
}

async function recordBindingAudit(
  tx: typeof db,
  action: AgentBindingAuditAction,
  actorUserId: string,
  binding: BindingRow,
  detail?: Record<string, unknown>,
): Promise<void> {
  await tx.insert(auditLog).values({
    actorUserId,
    action,
    targetType: "project_agent_binding",
    targetId: binding.id,
    metadata: {
      project_id: binding.projectId,
      org_agent_id: binding.orgAgentId,
      pinned_version: binding.pinnedVersion,
      pinned_content_hash: binding.pinnedContentHash,
      ...(detail ?? {}),
    },
  });
}

function toWire(
  binding: BindingRow,
  catalog: { name: string; status: AgentCatalogStatus },
): ProjectAgentBinding {
  return {
    binding_id: binding.id,
    project_id: binding.projectId,
    org_agent_id: binding.orgAgentId,
    agent_name: catalog.name,
    pinned_version: binding.pinnedVersion,
    pinned_content_hash: binding.pinnedContentHash,
    // Spec 123 I-B3: bindings whose upstream catalog row is now retired
    // surface as `retired_upstream` so the UI can dim/badge them; the data
    // path keeps them visible for audit.
    status: catalog.status === "retired" ? "retired_upstream" : "active",
    bound_by: binding.boundBy,
    bound_at: binding.boundAt.toISOString(),
  };
}

// ---------------------------------------------------------------------------
// Endpoints
// ---------------------------------------------------------------------------

export const listBindings = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/projects/:projectId/agents",
  },
  async (req: ListBindingsRequest): Promise<ListBindingsResponse> => {
    const { orgId } = requireOrgAuth();
    await verifyProjectInOrg(req.projectId, orgId);

    const rows = await db
      .select({
        binding: projectAgentBindings,
        catalogName: agentCatalog.name,
        catalogStatus: agentCatalog.status,
      })
      .from(projectAgentBindings)
      .innerJoin(
        agentCatalog,
        eq(agentCatalog.id, projectAgentBindings.orgAgentId),
      )
      .where(eq(projectAgentBindings.projectId, req.projectId))
      .orderBy(desc(projectAgentBindings.boundAt));

    return {
      bindings: rows.map((r) =>
        toWire(r.binding, {
          name: r.catalogName,
          status: r.catalogStatus,
        }),
      ),
    };
  },
);

export const bindAgent = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/projects/:projectId/agents/bind",
  },
  async (req: BindAgentRequest): Promise<BindAgentResponse> => {
    const { userId, orgId } = requireOrgAuth();
    await verifyProjectInOrg(req.projectId, orgId);

    const target = await resolveTargetByNameVersion(
      req.org_agent_id,
      req.version,
      orgId,
    );

    const binding = await db.transaction(async (tx) => {
      // Reject if the project already has a binding for this agent name —
      // operators must explicitly repin or unbind first. The unique
      // constraint at the DB layer is on (project_id, org_agent_id) which
      // would catch same-version dupes; we widen the check to "same name"
      // so v1 and v2 of the same agent can't both be bound to the project
      // (spec 123 §6.2 UI semantics: one binding per agent name).
      const existingByName = await tx
        .select({ id: projectAgentBindings.id })
        .from(projectAgentBindings)
        .innerJoin(
          agentCatalog,
          eq(agentCatalog.id, projectAgentBindings.orgAgentId),
        )
        .where(
          and(
            eq(projectAgentBindings.projectId, req.projectId),
            eq(agentCatalog.name, target.name),
          ),
        )
        .limit(1);
      if (existingByName.length > 0) {
        throw APIError.alreadyExists(
          `project already has a binding for agent "${target.name}"; repin or unbind first`,
        );
      }

      const [inserted] = await tx
        .insert(projectAgentBindings)
        .values({
          projectId: req.projectId,
          orgAgentId: target.id,
          pinnedVersion: target.version,
          pinnedContentHash: target.contentHash,
          boundBy: userId,
        })
        .returning();

      await recordBindingAudit(
        tx as unknown as typeof db,
        "agent.binding_created",
        userId,
        inserted,
        { agent_name: target.name },
      );
      return inserted;
    });

    await publishProjectAgentBindingUpdated({
      orgId,
      projectId: req.projectId,
      binding,
      agentName: target.name,
      action: "bound",
    });

    return {
      binding: toWire(binding, { name: target.name, status: target.status }),
    };
  },
);

export const repinBinding = api(
  {
    expose: true,
    auth: true,
    method: "PATCH",
    path: "/api/projects/:projectId/agents/:bindingId",
  },
  async (req: RepinBindingRequest): Promise<RepinBindingResponse> => {
    const { userId, orgId } = requireOrgAuth();
    await verifyProjectInOrg(req.projectId, orgId);

    const result = await db.transaction(async (tx) => {
      const existing = await loadBinding(req.bindingId, req.projectId);
      const target = await resolveTargetByNameVersion(
        existing.orgAgentId,
        req.version,
        orgId,
      );

      // Spec 123 §6.2 / I-B3 — repinning to a retired version is rejected
      // (already enforced inside resolveTargetByNameVersion). If the
      // requested version resolves to a different row id, the binding's
      // org_agent_id must move with it.
      const [updated] = await tx
        .update(projectAgentBindings)
        .set({
          orgAgentId: target.id,
          pinnedVersion: target.version,
          pinnedContentHash: target.contentHash,
          boundBy: userId,
          boundAt: new Date(),
        })
        .where(eq(projectAgentBindings.id, existing.id))
        .returning();

      await recordBindingAudit(
        tx as unknown as typeof db,
        "agent.binding_repinned",
        userId,
        updated,
        {
          agent_name: target.name,
          previous_pinned_version: existing.pinnedVersion,
          previous_org_agent_id: existing.orgAgentId,
          previous_pinned_content_hash: existing.pinnedContentHash,
        },
      );
      return { binding: updated, target };
    });

    await publishProjectAgentBindingUpdated({
      orgId,
      projectId: req.projectId,
      binding: result.binding,
      agentName: result.target.name,
      action: "rebound",
    });

    return {
      binding: toWire(result.binding, {
        name: result.target.name,
        status: result.target.status,
      }),
    };
  },
);

export const unbindAgent = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/api/projects/:projectId/agents/:bindingId",
  },
  async (req: UnbindRequest): Promise<UnbindResponse> => {
    const { userId, orgId } = requireOrgAuth();
    await verifyProjectInOrg(req.projectId, orgId);

    const removed = await db.transaction(async (tx) => {
      const existing = await loadBinding(req.bindingId, req.projectId);
      // Audit BEFORE delete so the row contents live in the audit row.
      const [agentNameRow] = await tx
        .select({ name: agentCatalog.name })
        .from(agentCatalog)
        .where(eq(agentCatalog.id, existing.orgAgentId))
        .limit(1);
      await recordBindingAudit(
        tx as unknown as typeof db,
        "agent.binding_unbound",
        userId,
        existing,
        { agent_name: agentNameRow?.name ?? null },
      );
      await tx
        .delete(projectAgentBindings)
        .where(eq(projectAgentBindings.id, existing.id));
      return { binding: existing, agentName: agentNameRow?.name ?? "" };
    });

    await publishProjectAgentBindingUpdated({
      orgId,
      projectId: req.projectId,
      binding: removed.binding,
      agentName: removed.agentName,
      action: "unbound",
    });

    return { ok: true };
  },
);

// ---------------------------------------------------------------------------
// Spec 098 integrity probe — exported for the nightly job (spec 123 T036).
// ---------------------------------------------------------------------------

export type BindingIntegrityViolation = {
  binding_id: string;
  project_id: string;
  org_agent_id: string;
  pinned_version: number;
  recorded_content_hash: string;
  current_content_hash: string | null;
  reason: "row_missing" | "hash_drift";
};

/**
 * Verify every binding's `pinned_content_hash` still matches the catalog
 * row at `(org_agent_id, pinned_version)`. Returns the list of violations
 * for the spec 098 nightly integrity job. Empty list → all bindings clean.
 */
export async function verifyBindingIntegrity(): Promise<
  BindingIntegrityViolation[]
> {
  const rows = await db
    .select({
      bindingId: projectAgentBindings.id,
      projectId: projectAgentBindings.projectId,
      orgAgentId: projectAgentBindings.orgAgentId,
      pinnedVersion: projectAgentBindings.pinnedVersion,
      recordedContentHash: projectAgentBindings.pinnedContentHash,
      currentContentHash: agentCatalog.contentHash,
      currentVersion: agentCatalog.version,
    })
    .from(projectAgentBindings)
    .innerJoin(
      agentCatalog,
      eq(agentCatalog.id, projectAgentBindings.orgAgentId),
    );

  const violations: BindingIntegrityViolation[] = [];
  for (const r of rows) {
    if (r.currentVersion !== r.pinnedVersion) {
      // The binding's org_agent_id row is at a different version than the
      // pinned one — should be impossible since org_agent_id is the
      // specific version row, but defensive.
      violations.push({
        binding_id: r.bindingId,
        project_id: r.projectId,
        org_agent_id: r.orgAgentId,
        pinned_version: r.pinnedVersion,
        recorded_content_hash: r.recordedContentHash,
        current_content_hash: r.currentContentHash,
        reason: "row_missing",
      });
    } else if (r.recordedContentHash !== r.currentContentHash) {
      violations.push({
        binding_id: r.bindingId,
        project_id: r.projectId,
        org_agent_id: r.orgAgentId,
        pinned_version: r.pinnedVersion,
        recorded_content_hash: r.recordedContentHash,
        current_content_hash: r.currentContentHash,
        reason: "hash_drift",
      });
    }
  }
  return violations;
}

// Silence unused-import lint when only resolveCatalogRow is needed for tests.
export const _internal = { resolveCatalogRow };
