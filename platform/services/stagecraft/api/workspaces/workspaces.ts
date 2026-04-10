/**
 * Workspace CRUD service (spec 087 Phase 1).
 *
 * The workspace is the unit of identity, governance, collaboration,
 * knowledge intake, and factory execution. Scoped to a GitHub org.
 */

import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { db } from "../db/drizzle";
import { workspaces, organizations, auditLog } from "../db/schema";
import { and, eq, desc } from "drizzle-orm";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type WorkspaceRow = {
  id: string;
  orgId: string;
  name: string;
  slug: string;
  objectStoreBucket: string;
  createdAt: Date;
  updatedAt: Date;
};

// ---------------------------------------------------------------------------
// List workspaces for the authenticated user's org
// ---------------------------------------------------------------------------

export const listWorkspaces = api(
  { expose: true, auth: true, method: "GET", path: "/api/workspaces" },
  async (): Promise<{ workspaces: WorkspaceRow[] }> => {
    const auth = getAuthData()!;

    const rows = await db
      .select()
      .from(workspaces)
      .where(eq(workspaces.orgId, auth.orgId))
      .orderBy(desc(workspaces.createdAt));

    return { workspaces: rows };
  }
);

// ---------------------------------------------------------------------------
// Get a single workspace by ID
// ---------------------------------------------------------------------------

export const getWorkspace = api(
  { expose: true, auth: true, method: "GET", path: "/api/workspaces/:id" },
  async (req: { id: string }): Promise<{ workspace: WorkspaceRow }> => {
    const auth = getAuthData()!;

    const [row] = await db
      .select()
      .from(workspaces)
      .where(and(eq(workspaces.id, req.id), eq(workspaces.orgId, auth.orgId)))
      .limit(1);

    if (!row) {
      throw APIError.notFound("workspace not found");
    }

    return { workspace: row };
  }
);

// ---------------------------------------------------------------------------
// Create workspace
// ---------------------------------------------------------------------------

type CreateWorkspaceRequest = {
  name: string;
  slug: string;
};

export const createWorkspace = api(
  { expose: true, auth: true, method: "POST", path: "/api/workspaces" },
  async (req: CreateWorkspaceRequest): Promise<{ workspace: WorkspaceRow }> => {
    const auth = getAuthData()!;

    if (!req.name || !req.slug) {
      throw APIError.invalidArgument("name and slug are required");
    }

    // Validate slug format (lowercase, alphanumeric, hyphens)
    if (!/^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/.test(req.slug)) {
      throw APIError.invalidArgument(
        "slug must be lowercase alphanumeric with hyphens"
      );
    }

    // Derive bucket name from org slug + workspace slug
    const [org] = await db
      .select({ slug: organizations.slug })
      .from(organizations)
      .where(eq(organizations.id, auth.orgId))
      .limit(1);

    if (!org) {
      throw APIError.notFound("organization not found");
    }

    const bucket = `oap-${org.slug}-${req.slug}`;

    const [workspace] = await db
      .insert(workspaces)
      .values({
        orgId: auth.orgId,
        name: req.name,
        slug: req.slug,
        objectStoreBucket: bucket,
      })
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "workspace.create",
      targetType: "workspace",
      targetId: workspace.id,
      metadata: { name: req.name, slug: req.slug, orgId: auth.orgId },
    });

    return { workspace };
  }
);

// ---------------------------------------------------------------------------
// Update workspace
// ---------------------------------------------------------------------------

type UpdateWorkspaceRequest = {
  id: string;
  name?: string;
};

export const updateWorkspace = api(
  { expose: true, auth: true, method: "PUT", path: "/api/workspaces/:id" },
  async (req: UpdateWorkspaceRequest): Promise<{ workspace: WorkspaceRow }> => {
    const auth = getAuthData()!;

    const [existing] = await db
      .select()
      .from(workspaces)
      .where(and(eq(workspaces.id, req.id), eq(workspaces.orgId, auth.orgId)))
      .limit(1);

    if (!existing) {
      throw APIError.notFound("workspace not found");
    }

    const [updated] = await db
      .update(workspaces)
      .set({
        ...(req.name !== undefined && { name: req.name }),
        updatedAt: new Date(),
      })
      .where(eq(workspaces.id, req.id))
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "workspace.update",
      targetType: "workspace",
      targetId: req.id,
      metadata: { name: req.name },
    });

    return { workspace: updated };
  }
);

// ---------------------------------------------------------------------------
// Get the default workspace for an org (convenience endpoint)
// ---------------------------------------------------------------------------

export const getDefaultWorkspace = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/workspaces/by-org/default",
  },
  async (): Promise<{ workspace: WorkspaceRow }> => {
    const auth = getAuthData()!;

    const [row] = await db
      .select()
      .from(workspaces)
      .where(and(eq(workspaces.orgId, auth.orgId), eq(workspaces.slug, "default")))
      .limit(1);

    if (!row) {
      // Auto-create a default workspace if none exists
      const [org] = await db
        .select({ slug: organizations.slug })
        .from(organizations)
        .where(eq(organizations.id, auth.orgId))
        .limit(1);

      const orgSlug = org?.slug ?? "unknown";

      const [created] = await db
        .insert(workspaces)
        .values({
          orgId: auth.orgId,
          name: "Default",
          slug: "default",
          objectStoreBucket: `oap-${orgSlug}-default`,
        })
        .returning();

      return { workspace: created };
    }

    return { workspace: row };
  }
);
