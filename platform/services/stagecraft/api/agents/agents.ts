import { api, APIError } from "encore.dev/api";
import { db } from "../db/drizzle";
import { agentPolicies, auditLog } from "../db/schema";
import { and, eq, desc } from "drizzle-orm";

/** Default org scope when multi-tenancy is not yet wired. */
const DEFAULT_ORG = "default";

// ---------------------------------------------------------------------------
// Seam D: runtime authorization check
// ---------------------------------------------------------------------------

type AgentAuthorizedResponse = { authorized: true };

/**
 * Seam D: Validate agent execution against org-level policies.
 * GET /api/agents/:slug/authorized
 *
 * Returns 200 if the agent is authorized.
 * Returns 403 with { reason } if the agent is blocked.
 * Agents with no policy row are allowed by default.
 */
export const isAgentAuthorized = api(
  { expose: true, method: "GET", path: "/api/agents/:slug/authorized" },
  async (req: { slug: string }): Promise<AgentAuthorizedResponse> => {
    const rows = await db
      .select()
      .from(agentPolicies)
      .where(
        and(
          eq(agentPolicies.orgId, DEFAULT_ORG),
          eq(agentPolicies.slug, req.slug)
        )
      )
      .limit(1);

    if (rows.length > 0 && rows[0].blocked) {
      const reason = rows[0].reason || `agent '${req.slug}' is blocked by org policy`;
      throw APIError.permissionDenied(reason);
    }

    return { authorized: true };
  }
);

// ---------------------------------------------------------------------------
// Admin CRUD for agent policies
// ---------------------------------------------------------------------------

type AgentPolicyRow = {
  id: string;
  orgId: string;
  slug: string;
  blocked: boolean;
  reason: string;
  createdAt: Date;
  updatedAt: Date;
};

type ListAgentPoliciesResponse = { policies: AgentPolicyRow[] };

export const listAgentPolicies = api(
  { expose: true, method: "GET", path: "/admin/agent-policies" },
  async (): Promise<ListAgentPoliciesResponse> => {
    const rows = await db
      .select()
      .from(agentPolicies)
      .orderBy(desc(agentPolicies.createdAt))
      .limit(500);
    return { policies: rows };
  }
);

type UpsertAgentPolicyRequest = {
  slug: string;
  blocked: boolean;
  reason?: string;
  actorUserId?: string;
};

type UpsertAgentPolicyResponse = { policy: AgentPolicyRow };

export const upsertAgentPolicy = api(
  { expose: true, method: "POST", path: "/admin/agent-policies" },
  async (req: UpsertAgentPolicyRequest): Promise<UpsertAgentPolicyResponse> => {
    if (!req.slug) {
      throw APIError.invalidArgument("slug is required");
    }

    const now = new Date();
    const existing = await db
      .select()
      .from(agentPolicies)
      .where(
        and(
          eq(agentPolicies.orgId, DEFAULT_ORG),
          eq(agentPolicies.slug, req.slug)
        )
      )
      .limit(1);

    let policy: AgentPolicyRow;

    if (existing.length > 0) {
      const [updated] = await db
        .update(agentPolicies)
        .set({
          blocked: req.blocked,
          reason: req.reason ?? "",
          updatedAt: now,
        })
        .where(eq(agentPolicies.id, existing[0].id))
        .returning();
      policy = updated;
    } else {
      const [inserted] = await db
        .insert(agentPolicies)
        .values({
          orgId: DEFAULT_ORG,
          slug: req.slug,
          blocked: req.blocked,
          reason: req.reason ?? "",
        })
        .returning();
      policy = inserted;
    }

    await db.insert(auditLog).values({
      actorUserId: req.actorUserId ?? "00000000-0000-0000-0000-000000000000",
      action: req.blocked ? "agent_policy.block" : "agent_policy.allow",
      targetType: "agent_policy",
      targetId: policy.id,
      metadata: { slug: req.slug, blocked: req.blocked, reason: req.reason ?? "" },
    });

    return { policy };
  }
);

type DeleteAgentPolicyResponse = { ok: true };

export const deleteAgentPolicy = api(
  { expose: true, method: "DELETE", path: "/admin/agent-policies/:id" },
  async (req: { id: string }): Promise<DeleteAgentPolicyResponse> => {
    const existing = await db
      .select()
      .from(agentPolicies)
      .where(eq(agentPolicies.id, req.id))
      .limit(1);

    if (existing.length === 0) {
      throw APIError.notFound("agent policy not found");
    }

    await db.delete(agentPolicies).where(eq(agentPolicies.id, req.id));

    await db.insert(auditLog).values({
      actorUserId: "00000000-0000-0000-0000-000000000000",
      action: "agent_policy.delete",
      targetType: "agent_policy",
      targetId: req.id,
      metadata: { slug: existing[0].slug },
    });

    return { ok: true };
  }
);
