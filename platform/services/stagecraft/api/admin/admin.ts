import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { db } from "../db/drizzle";
import {
  auditLog,
  githubTeamRoleMappings,
  oidcGroupRoleMappings,
  oidcProviders,
  users,
} from "../db/schema";
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

// ---------------------------------------------------------------------------
// OIDC Provider CRUD (spec 080 Phase 4 — Enterprise OIDC Federation)
// ---------------------------------------------------------------------------

export type OidcProviderRow = {
  id: string;
  orgId: string;
  name: string;
  providerType: string;
  issuer: string;
  clientId: string;
  scopes: string;
  claimsMapping: unknown;
  emailDomain: string | null;
  autoProvision: boolean;
  status: string;
  createdAt: Date;
  updatedAt: Date;
};

export type ListOidcProvidersResponse = { providers: OidcProviderRow[] };

export const listOidcProviders = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/admin/orgs/:orgId/oidc-providers",
  },
  async (req: { orgId: string }): Promise<ListOidcProvidersResponse> => {
    requireOrgAdmin(req.orgId);
    const rows = await db
      .select({
        id: oidcProviders.id,
        orgId: oidcProviders.orgId,
        name: oidcProviders.name,
        providerType: oidcProviders.providerType,
        issuer: oidcProviders.issuer,
        clientId: oidcProviders.clientId,
        scopes: oidcProviders.scopes,
        claimsMapping: oidcProviders.claimsMapping,
        emailDomain: oidcProviders.emailDomain,
        autoProvision: oidcProviders.autoProvision,
        status: oidcProviders.status,
        createdAt: oidcProviders.createdAt,
        updatedAt: oidcProviders.updatedAt,
      })
      .from(oidcProviders)
      .where(eq(oidcProviders.orgId, req.orgId));

    return { providers: rows };
  }
);

export const createOidcProvider = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/admin/orgs/:orgId/oidc-providers",
  },
  async (req: {
    orgId: string;
    name: string;
    providerType: string;
    issuer: string;
    clientId: string;
    clientSecretEnc: string;
    scopes?: string;
    claimsMapping?: Record<string, string>;
    emailDomain?: string;
    autoProvision?: boolean;
  }): Promise<OidcProviderRow> => {
    const auth = requireOrgAdmin(req.orgId);

    const [created] = await db
      .insert(oidcProviders)
      .values({
        orgId: req.orgId,
        name: req.name,
        providerType: req.providerType,
        issuer: req.issuer,
        clientId: req.clientId,
        clientSecretEnc: req.clientSecretEnc,
        scopes: req.scopes ?? "openid profile email",
        claimsMapping: req.claimsMapping ?? {},
        emailDomain: req.emailDomain ?? null,
        autoProvision: req.autoProvision ?? true,
        status: "active",
      })
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "oidc_provider.created",
      targetType: "oidc_provider",
      targetId: created.id,
      metadata: {
        org_id: req.orgId,
        name: req.name,
        provider_type: req.providerType,
        issuer: req.issuer,
        email_domain: req.emailDomain,
      },
    });

    return {
      id: created.id,
      orgId: created.orgId,
      name: created.name,
      providerType: created.providerType,
      issuer: created.issuer,
      clientId: created.clientId,
      scopes: created.scopes,
      claimsMapping: created.claimsMapping,
      emailDomain: created.emailDomain,
      autoProvision: created.autoProvision,
      status: created.status,
      createdAt: created.createdAt,
      updatedAt: created.updatedAt,
    };
  }
);

export const updateOidcProvider = api(
  {
    expose: true,
    auth: true,
    method: "PUT",
    path: "/admin/orgs/:orgId/oidc-providers/:id",
  },
  async (req: {
    orgId: string;
    id: string;
    name?: string;
    scopes?: string;
    claimsMapping?: Record<string, string>;
    emailDomain?: string;
    autoProvision?: boolean;
    status?: string;
  }): Promise<{ ok: true }> => {
    const auth = requireOrgAdmin(req.orgId);

    const VALID_STATUSES = ["active", "disabled", "pending"];
    if (req.status !== undefined && !VALID_STATUSES.includes(req.status)) {
      throw APIError.invalidArgument(`Invalid status: must be one of ${VALID_STATUSES.join(", ")}`);
    }

    const updates: Record<string, unknown> = { updatedAt: new Date() };
    if (req.name !== undefined) updates.name = req.name;
    if (req.scopes !== undefined) updates.scopes = req.scopes;
    if (req.claimsMapping !== undefined) updates.claimsMapping = req.claimsMapping;
    if (req.emailDomain !== undefined) updates.emailDomain = req.emailDomain;
    if (req.autoProvision !== undefined) updates.autoProvision = req.autoProvision;
    if (req.status !== undefined) updates.status = req.status;

    const result = await db
      .update(oidcProviders)
      .set(updates)
      .where(
        and(
          eq(oidcProviders.id, req.id),
          eq(oidcProviders.orgId, req.orgId)
        )
      );

    if (!result.rowCount || result.rowCount === 0) {
      throw APIError.notFound("OIDC provider not found");
    }

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "oidc_provider.updated",
      targetType: "oidc_provider",
      targetId: req.id,
      metadata: { org_id: req.orgId, ...updates },
    });

    return { ok: true };
  }
);

export const deleteOidcProvider = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/admin/orgs/:orgId/oidc-providers/:id",
  },
  async (req: { orgId: string; id: string }): Promise<{ ok: true }> => {
    const auth = requireOrgAdmin(req.orgId);

    // Cascade will handle oidc_group_role_mappings
    const [deleted] = await db
      .delete(oidcProviders)
      .where(
        and(
          eq(oidcProviders.id, req.id),
          eq(oidcProviders.orgId, req.orgId)
        )
      )
      .returning({ id: oidcProviders.id });

    if (!deleted) {
      throw APIError.notFound("OIDC provider not found");
    }

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "oidc_provider.deleted",
      targetType: "oidc_provider",
      targetId: req.id,
      metadata: { org_id: req.orgId },
    });

    return { ok: true };
  }
);

// ---------------------------------------------------------------------------
// OIDC Group-to-Role Mapping CRUD (spec 080 Phase 4)
// ---------------------------------------------------------------------------

export type OidcGroupMappingRow = {
  id: string;
  orgId: string;
  providerId: string;
  idpGroupId: string;
  idpGroupName: string | null;
  targetScope: "org" | "project";
  targetId: string | null;
  role: string;
  createdAt: Date;
};

export type ListOidcGroupMappingsResponse = { mappings: OidcGroupMappingRow[] };

export const listOidcGroupMappings = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/admin/orgs/:orgId/oidc-providers/:providerId/group-mappings",
  },
  async (req: { orgId: string; providerId: string }): Promise<ListOidcGroupMappingsResponse> => {
    requireOrgAdmin(req.orgId);
    const rows = await db
      .select()
      .from(oidcGroupRoleMappings)
      .where(
        and(
          eq(oidcGroupRoleMappings.orgId, req.orgId),
          eq(oidcGroupRoleMappings.providerId, req.providerId)
        )
      );

    return { mappings: rows };
  }
);

export const createOidcGroupMapping = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/admin/orgs/:orgId/oidc-providers/:providerId/group-mappings",
  },
  async (req: {
    orgId: string;
    providerId: string;
    idpGroupId: string;
    idpGroupName?: string;
    targetScope: "org" | "project";
    targetId?: string;
    role: string;
  }): Promise<OidcGroupMappingRow> => {
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

    // Verify the provider belongs to this org
    const [provider] = await db
      .select({ id: oidcProviders.id })
      .from(oidcProviders)
      .where(
        and(eq(oidcProviders.id, req.providerId), eq(oidcProviders.orgId, req.orgId))
      )
      .limit(1);

    if (!provider) {
      throw APIError.notFound("OIDC provider not found in this organization");
    }

    const [created] = await db
      .insert(oidcGroupRoleMappings)
      .values({
        orgId: req.orgId,
        providerId: req.providerId,
        idpGroupId: req.idpGroupId,
        idpGroupName: req.idpGroupName ?? null,
        targetScope: req.targetScope,
        targetId: req.targetId ?? null,
        role: req.role,
      })
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "oidc_group_mapping.created",
      targetType: "oidc_group_mapping",
      targetId: created.id,
      metadata: {
        org_id: req.orgId,
        provider_id: req.providerId,
        idp_group_id: req.idpGroupId,
        target_scope: req.targetScope,
        role: req.role,
      },
    });

    return created;
  }
);

export const deleteOidcGroupMapping = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/admin/orgs/:orgId/oidc-providers/:providerId/group-mappings/:id",
  },
  async (req: { orgId: string; providerId: string; id: string }): Promise<{ ok: true }> => {
    const auth = requireOrgAdmin(req.orgId);

    const [deleted] = await db
      .delete(oidcGroupRoleMappings)
      .where(
        and(
          eq(oidcGroupRoleMappings.id, req.id),
          eq(oidcGroupRoleMappings.orgId, req.orgId),
          eq(oidcGroupRoleMappings.providerId, req.providerId)
        )
      )
      .returning({ id: oidcGroupRoleMappings.id });

    if (!deleted) {
      throw APIError.notFound("OIDC group mapping not found");
    }

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "oidc_group_mapping.deleted",
      targetType: "oidc_group_mapping",
      targetId: req.id,
      metadata: { org_id: req.orgId, provider_id: req.providerId },
    });

    return { ok: true };
  }
);
