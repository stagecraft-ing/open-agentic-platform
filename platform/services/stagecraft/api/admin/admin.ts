import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { db } from "../db/drizzle";
import { auditLog, githubTeamRoleMappings, users } from "../db/schema";
import { desc, eq, and } from "drizzle-orm";

/** Require admin or owner platform role. Throws 403 if not. */
function requireAdmin(): { userID: string; orgId: string } {
  const auth = getAuthData()!;
  if (auth.platformRole !== "admin" && auth.platformRole !== "owner") {
    throw APIError.permissionDenied("Admin access required");
  }
  return auth;
}

/** Require admin for a specific org. Prevents cross-org access. */
function requireOrgAdmin(reqOrgId: string): { userID: string; orgId: string } {
  const auth = requireAdmin();
  if (auth.orgId !== reqOrgId) {
    throw APIError.permissionDenied("Cannot manage another organization's resources");
  }
  return auth;
}

export type UserRow = {
  id: string;
  email: string;
  name: string;
  role: "user" | "admin";
  disabled: boolean;
  createdAt: Date;
};

export type ListUsersResponse = { users: UserRow[] };

export type SetRoleResponse = { ok: true };

export type AuditRow = {
  id: string;
  actorUserId: string;
  action: string;
  targetType: string;
  targetId: string;
  metadata: Record<string, unknown>;
  createdAt: Date;
};

export type ListAuditResponse = { events: AuditRow[] };

export const listUsers = api(
  { expose: true, auth: true, method: "GET", path: "/admin/users" },
  async (): Promise<ListUsersResponse> => {
    requireAdmin();
    const rows = await db.select({
      id: users.id,
      email: users.email,
      name: users.name,
      role: users.role,
      disabled: users.disabled,
      createdAt: users.createdAt,
    }).from(users);

    return { users: rows };
  }
);

export const setRole = api(
  { expose: true, auth: true, method: "POST", path: "/admin/users/set-role" },
  async (req: {
    userId: string;
    role: "user" | "admin";
  }): Promise<SetRoleResponse> => {
    const auth = requireAdmin();
    await db.update(users).set({ role: req.role }).where(eq(users.id, req.userId));

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "user.set_role",
      targetType: "user",
      targetId: req.userId,
      metadata: { role: req.role },
    });

    return { ok: true };
  }
);

export const listAudit = api(
  { expose: true, auth: true, method: "GET", path: "/admin/audit" },
  async (): Promise<ListAuditResponse> => {
    requireAdmin();
    const rows = await db
      .select()
      .from(auditLog)
      .orderBy(desc(auditLog.createdAt))
      .limit(200);
    return {
      events: rows.map((r) => ({
        ...r,
        metadata: (r.metadata ?? {}) as Record<string, unknown>,
      })),
    };
  }
);

// ---------------------------------------------------------------------------
// Team-to-role mapping CRUD (spec 080 Phase 3 — FR-009)
// ---------------------------------------------------------------------------

export type TeamMappingRow = {
  id: string;
  orgId: string;
  githubTeamSlug: string;
  githubTeamId: number;
  targetScope: "org" | "project";
  targetId: string | null;
  role: string;
  createdAt: Date;
};

export type ListTeamMappingsResponse = { mappings: TeamMappingRow[] };

export const listTeamMappings = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/admin/orgs/:orgId/team-mappings",
  },
  async (req: { orgId: string }): Promise<ListTeamMappingsResponse> => {
    requireOrgAdmin(req.orgId);
    const rows = await db
      .select()
      .from(githubTeamRoleMappings)
      .where(eq(githubTeamRoleMappings.orgId, req.orgId));

    return { mappings: rows };
  }
);

export const createTeamMapping = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/admin/orgs/:orgId/team-mappings",
  },
  async (req: {
    orgId: string;
    githubTeamSlug: string;
    githubTeamId: number;
    targetScope: "org" | "project";
    targetId?: string;
    role: string;
  }): Promise<TeamMappingRow> => {
    const auth = requireOrgAdmin(req.orgId);

    // Validate role against scope
    const validOrgRoles = new Set(["owner", "admin", "member"]);
    const validProjectRoles = new Set(["viewer", "developer", "deployer", "admin"]);
    const validRoles = req.targetScope === "org" ? validOrgRoles : validProjectRoles;
    if (!validRoles.has(req.role)) {
      throw APIError.invalidArgument(
        `Invalid role "${req.role}" for ${req.targetScope} scope. Valid: ${[...validRoles].join(", ")}`
      );
    }

    if (req.targetScope === "project" && !req.targetId) {
      throw APIError.invalidArgument("targetId is required for project-scope mappings");
    }

    const [created] = await db
      .insert(githubTeamRoleMappings)
      .values({
        orgId: req.orgId,
        githubTeamSlug: req.githubTeamSlug,
        githubTeamId: req.githubTeamId,
        targetScope: req.targetScope,
        targetId: req.targetId ?? null,
        role: req.role,
      })
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "team_mapping.created",
      targetType: "team_mapping",
      targetId: created.id,
      metadata: {
        org_id: req.orgId,
        team_slug: req.githubTeamSlug,
        target_scope: req.targetScope,
        role: req.role,
      },
    });

    return created;
  }
);

export const updateTeamMapping = api(
  {
    expose: true,
    auth: true,
    method: "PUT",
    path: "/admin/orgs/:orgId/team-mappings/:id",
  },
  async (req: { orgId: string; id: string; role: string }): Promise<{ ok: true }> => {
    const auth = requireOrgAdmin(req.orgId);

    const result = await db
      .update(githubTeamRoleMappings)
      .set({ role: req.role })
      .where(
        and(
          eq(githubTeamRoleMappings.id, req.id),
          eq(githubTeamRoleMappings.orgId, req.orgId)
        )
      );

    if (!result.rowCount || result.rowCount === 0) {
      throw APIError.notFound("Team mapping not found");
    }

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "team_mapping.updated",
      targetType: "team_mapping",
      targetId: req.id,
      metadata: { org_id: req.orgId, role: req.role },
    });

    return { ok: true };
  }
);

export const deleteTeamMapping = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/admin/orgs/:orgId/team-mappings/:id",
  },
  async (req: { orgId: string; id: string }): Promise<{ ok: true }> => {
    const auth = requireOrgAdmin(req.orgId);

    const [deleted] = await db
      .delete(githubTeamRoleMappings)
      .where(
        and(
          eq(githubTeamRoleMappings.id, req.id),
          eq(githubTeamRoleMappings.orgId, req.orgId)
        )
      )
      .returning({ id: githubTeamRoleMappings.id });

    if (!deleted) {
      throw APIError.notFound("Team mapping not found");
    }

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "team_mapping.deleted",
      targetType: "team_mapping",
      targetId: req.id,
      metadata: { org_id: req.orgId },
    });

    return { ok: true };
  }
);
