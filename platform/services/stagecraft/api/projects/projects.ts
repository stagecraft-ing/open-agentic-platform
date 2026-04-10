import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { db } from "../db/drizzle";
import {
  organizations,
  workspaces,
  projects,
  projectRepos,
  environments,
  projectMembers,
  auditLog,
} from "../db/schema";
import { and, eq, desc } from "drizzle-orm";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type OrgRow = {
  id: string;
  name: string;
  slug: string;
  createdBy: string | null;
  createdAt: Date;
  updatedAt: Date;
};

export type ProjectRow = {
  id: string;
  orgId: string;
  workspaceId: string;
  name: string;
  slug: string;
  description: string;
  createdBy: string | null;
  createdAt: Date;
  updatedAt: Date;
};

export type RepoRow = {
  id: string;
  projectId: string;
  githubOrg: string;
  repoName: string;
  defaultBranch: string;
  isPrimary: boolean;
  githubInstallId: number | null;
  createdAt: Date;
  updatedAt: Date;
};

export type EnvironmentRow = {
  id: string;
  projectId: string;
  name: string;
  kind: "preview" | "development" | "staging" | "production";
  k8sNamespace: string | null;
  autoDeployBranch: string | null;
  requiresApproval: boolean;
  createdAt: Date;
  updatedAt: Date;
};

export type MemberRow = {
  id: string;
  projectId: string;
  userId: string;
  role: "viewer" | "developer" | "deployer" | "admin";
  createdAt: Date;
  updatedAt: Date;
};

// ---------------------------------------------------------------------------
// Organizations
// ---------------------------------------------------------------------------

export const getOrg = api(
  { expose: true, auth: true, method: "GET", path: "/api/orgs/current" },
  async (): Promise<{ org: OrgRow }> => {
    const auth = getAuthData()!;

    const rows = await db
      .select()
      .from(organizations)
      .where(eq(organizations.id, auth.orgId))
      .limit(1);

    if (rows.length === 0) {
      throw APIError.notFound("organization not found");
    }
    return { org: rows[0] };
  }
);

// ---------------------------------------------------------------------------
// Projects CRUD
// ---------------------------------------------------------------------------

export const listProjects = api(
  { expose: true, auth: true, method: "GET", path: "/api/projects" },
  async (): Promise<{ projects: ProjectRow[] }> => {
    const auth = getAuthData()!;

    const rows = await db
      .select()
      .from(projects)
      .where(eq(projects.workspaceId, auth.workspaceId))
      .orderBy(desc(projects.createdAt));
    return { projects: rows };
  }
);

export const getProject = api(
  { expose: true, auth: true, method: "GET", path: "/api/projects/:id" },
  async (req: { id: string }): Promise<{ project: ProjectRow }> => {
    const auth = getAuthData()!;

    const rows = await db
      .select()
      .from(projects)
      .where(
        and(eq(projects.id, req.id), eq(projects.workspaceId, auth.workspaceId))
      )
      .limit(1);

    if (rows.length === 0) {
      throw APIError.notFound("project not found");
    }
    return { project: rows[0] };
  }
);

type CreateProjectRequest = {
  name: string;
  slug: string;
  description?: string;
};

export const createProject = api(
  { expose: true, auth: true, method: "POST", path: "/api/projects" },
  async (req: CreateProjectRequest): Promise<{ project: ProjectRow }> => {
    const auth = getAuthData()!;

    if (!req.name || !req.slug) {
      throw APIError.invalidArgument("name and slug are required");
    }

    const [project] = await db
      .insert(projects)
      .values({
        orgId: auth.orgId,
        workspaceId: auth.workspaceId,
        name: req.name,
        slug: req.slug,
        description: req.description ?? "",
        createdBy: auth.userID,
      })
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "project.create",
      targetType: "project",
      targetId: project.id,
      metadata: { name: req.name, slug: req.slug, workspaceId: auth.workspaceId },
    });

    // Auto-add creator as project admin
    await db.insert(projectMembers).values({
      projectId: project.id,
      userId: auth.userID,
      role: "admin",
    });

    return { project };
  }
);

type UpdateProjectRequest = {
  id: string;
  name?: string;
  description?: string;
};

export const updateProject = api(
  { expose: true, auth: true, method: "PUT", path: "/api/projects/:id" },
  async (req: UpdateProjectRequest): Promise<{ project: ProjectRow }> => {
    const auth = getAuthData()!;

    const existing = await db
      .select()
      .from(projects)
      .where(
        and(eq(projects.id, req.id), eq(projects.workspaceId, auth.workspaceId))
      )
      .limit(1);

    if (existing.length === 0) {
      throw APIError.notFound("project not found");
    }

    const [updated] = await db
      .update(projects)
      .set({
        ...(req.name !== undefined && { name: req.name }),
        ...(req.description !== undefined && { description: req.description }),
        updatedAt: new Date(),
      })
      .where(eq(projects.id, req.id))
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "project.update",
      targetType: "project",
      targetId: req.id,
      metadata: { name: req.name, description: req.description },
    });

    return { project: updated };
  }
);

export const deleteProject = api(
  { expose: true, auth: true, method: "DELETE", path: "/api/projects/:id" },
  async (req: { id: string }): Promise<{ ok: true }> => {
    const auth = getAuthData()!;

    const existing = await db
      .select()
      .from(projects)
      .where(
        and(eq(projects.id, req.id), eq(projects.workspaceId, auth.workspaceId))
      )
      .limit(1);

    if (existing.length === 0) {
      throw APIError.notFound("project not found");
    }

    await db.delete(projects).where(eq(projects.id, req.id));

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "project.delete",
      targetType: "project",
      targetId: req.id,
      metadata: { name: existing[0].name, slug: existing[0].slug },
    });

    return { ok: true };
  }
);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function verifyProjectOwnership(
  projectId: string,
  workspaceId: string
): Promise<void> {
  const [row] = await db
    .select({ id: projects.id })
    .from(projects)
    .where(
      and(eq(projects.id, projectId), eq(projects.workspaceId, workspaceId))
    )
    .limit(1);

  if (!row) {
    throw APIError.notFound("project not found");
  }
}

// ---------------------------------------------------------------------------
// Project Repos
// ---------------------------------------------------------------------------

export const listProjectRepos = api(
  { expose: true, auth: true, method: "GET", path: "/api/projects/:projectId/repos" },
  async (req: { projectId: string }): Promise<{ repos: RepoRow[] }> => {
    const auth = getAuthData()!;
    await verifyProjectOwnership(req.projectId, auth.workspaceId);

    const rows = await db
      .select()
      .from(projectRepos)
      .where(eq(projectRepos.projectId, req.projectId))
      .orderBy(desc(projectRepos.createdAt));
    return { repos: rows };
  }
);

type AddRepoRequest = {
  projectId: string;
  githubOrg: string;
  repoName: string;
  defaultBranch?: string;
  isPrimary?: boolean;
};

export const addProjectRepo = api(
  { expose: true, auth: true, method: "POST", path: "/api/projects/:projectId/repos" },
  async (req: AddRepoRequest): Promise<{ repo: RepoRow }> => {
    const auth = getAuthData()!;
    await verifyProjectOwnership(req.projectId, auth.workspaceId);

    if (!req.githubOrg || !req.repoName) {
      throw APIError.invalidArgument("githubOrg and repoName are required");
    }

    const [repo] = await db
      .insert(projectRepos)
      .values({
        projectId: req.projectId,
        githubOrg: req.githubOrg,
        repoName: req.repoName,
        defaultBranch: req.defaultBranch ?? "main",
        isPrimary: req.isPrimary ?? false,
      })
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "project_repo.add",
      targetType: "project_repo",
      targetId: repo.id,
      metadata: {
        projectId: req.projectId,
        githubOrg: req.githubOrg,
        repoName: req.repoName,
      },
    });

    return { repo };
  }
);

export const removeProjectRepo = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/api/projects/:projectId/repos/:id",
  },
  async (req: {
    projectId: string;
    id: string;
  }): Promise<{ ok: true }> => {
    const auth = getAuthData()!;
    await verifyProjectOwnership(req.projectId, auth.workspaceId);

    const existing = await db
      .select()
      .from(projectRepos)
      .where(
        and(
          eq(projectRepos.id, req.id),
          eq(projectRepos.projectId, req.projectId)
        )
      )
      .limit(1);

    if (existing.length === 0) {
      throw APIError.notFound("repo not found");
    }

    await db.delete(projectRepos).where(eq(projectRepos.id, req.id));

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "project_repo.remove",
      targetType: "project_repo",
      targetId: req.id,
      metadata: {
        projectId: req.projectId,
        githubOrg: existing[0].githubOrg,
        repoName: existing[0].repoName,
      },
    });

    return { ok: true };
  }
);

// ---------------------------------------------------------------------------
// Environments
// ---------------------------------------------------------------------------

export const listEnvironments = api(
  { expose: true, auth: true, method: "GET", path: "/api/projects/:projectId/envs" },
  async (req: {
    projectId: string;
  }): Promise<{ environments: EnvironmentRow[] }> => {
    const auth = getAuthData()!;
    await verifyProjectOwnership(req.projectId, auth.workspaceId);

    const rows = await db
      .select()
      .from(environments)
      .where(eq(environments.projectId, req.projectId))
      .orderBy(environments.createdAt);
    return { environments: rows };
  }
);

type CreateEnvironmentRequest = {
  projectId: string;
  name: string;
  kind?: "preview" | "development" | "staging" | "production";
  autoDeployBranch?: string;
  requiresApproval?: boolean;
};

export const createEnvironment = api(
  { expose: true, auth: true, method: "POST", path: "/api/projects/:projectId/envs" },
  async (
    req: CreateEnvironmentRequest
  ): Promise<{ environment: EnvironmentRow }> => {
    const auth = getAuthData()!;
    await verifyProjectOwnership(req.projectId, auth.workspaceId);

    if (!req.name) {
      throw APIError.invalidArgument("name is required");
    }

    // Auto-derive k8s namespace from project slug + env name
    const projectRows = await db
      .select({ slug: projects.slug })
      .from(projects)
      .where(eq(projects.id, req.projectId))
      .limit(1);

    if (projectRows.length === 0) {
      throw APIError.notFound("project not found");
    }

    const k8sNamespace = `proj--${projectRows[0].slug}--${req.name}`;

    const [env] = await db
      .insert(environments)
      .values({
        projectId: req.projectId,
        name: req.name,
        kind: req.kind ?? "development",
        k8sNamespace,
        autoDeployBranch: req.autoDeployBranch,
        requiresApproval: req.requiresApproval ?? false,
      })
      .returning();

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "environment.create",
      targetType: "environment",
      targetId: env.id,
      metadata: {
        projectId: req.projectId,
        name: req.name,
        kind: req.kind ?? "development",
      },
    });

    return { environment: env };
  }
);

export const deleteEnvironment = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/api/projects/:projectId/envs/:id",
  },
  async (req: {
    projectId: string;
    id: string;
  }): Promise<{ ok: true }> => {
    const auth = getAuthData()!;
    await verifyProjectOwnership(req.projectId, auth.workspaceId);

    const existing = await db
      .select()
      .from(environments)
      .where(
        and(
          eq(environments.id, req.id),
          eq(environments.projectId, req.projectId)
        )
      )
      .limit(1);

    if (existing.length === 0) {
      throw APIError.notFound("environment not found");
    }

    await db.delete(environments).where(eq(environments.id, req.id));

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "environment.delete",
      targetType: "environment",
      targetId: req.id,
      metadata: {
        projectId: req.projectId,
        name: existing[0].name,
      },
    });

    return { ok: true };
  }
);

// ---------------------------------------------------------------------------
// Project Members
// ---------------------------------------------------------------------------

export const listProjectMembers = api(
  { expose: true, auth: true, method: "GET", path: "/api/projects/:projectId/members" },
  async (req: { projectId: string }): Promise<{ members: MemberRow[] }> => {
    const auth = getAuthData()!;
    await verifyProjectOwnership(req.projectId, auth.workspaceId);

    const rows = await db
      .select()
      .from(projectMembers)
      .where(eq(projectMembers.projectId, req.projectId))
      .orderBy(projectMembers.createdAt);
    return { members: rows };
  }
);

type SetMemberRequest = {
  projectId: string;
  userId: string;
  role: "viewer" | "developer" | "deployer" | "admin";
};

export const setProjectMember = api(
  { expose: true, auth: true, method: "POST", path: "/api/projects/:projectId/members" },
  async (req: SetMemberRequest): Promise<{ member: MemberRow }> => {
    const auth = getAuthData()!;
    await verifyProjectOwnership(req.projectId, auth.workspaceId);

    if (!req.userId || !req.role) {
      throw APIError.invalidArgument("userId and role are required");
    }

    const now = new Date();
    const existing = await db
      .select()
      .from(projectMembers)
      .where(
        and(
          eq(projectMembers.projectId, req.projectId),
          eq(projectMembers.userId, req.userId)
        )
      )
      .limit(1);

    let member: MemberRow;

    if (existing.length > 0) {
      const [updated] = await db
        .update(projectMembers)
        .set({ role: req.role, updatedAt: now })
        .where(eq(projectMembers.id, existing[0].id))
        .returning();
      member = updated;
    } else {
      const [inserted] = await db
        .insert(projectMembers)
        .values({
          projectId: req.projectId,
          userId: req.userId,
          role: req.role,
        })
        .returning();
      member = inserted;
    }

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "project_member.set",
      targetType: "project_member",
      targetId: member.id,
      metadata: {
        projectId: req.projectId,
        userId: req.userId,
        role: req.role,
      },
    });

    return { member };
  }
);

export const removeProjectMember = api(
  {
    expose: true,
    auth: true,
    method: "DELETE",
    path: "/api/projects/:projectId/members/:userId",
  },
  async (req: {
    projectId: string;
    userId: string;
  }): Promise<{ ok: true }> => {
    const auth = getAuthData()!;
    await verifyProjectOwnership(req.projectId, auth.workspaceId);

    const existing = await db
      .select()
      .from(projectMembers)
      .where(
        and(
          eq(projectMembers.projectId, req.projectId),
          eq(projectMembers.userId, req.userId)
        )
      )
      .limit(1);

    if (existing.length === 0) {
      throw APIError.notFound("member not found");
    }

    await db
      .delete(projectMembers)
      .where(eq(projectMembers.id, existing[0].id));

    await db.insert(auditLog).values({
      actorUserId: auth.userID,
      action: "project_member.remove",
      targetType: "project_member",
      targetId: existing[0].id,
      metadata: {
        projectId: req.projectId,
        userId: req.userId,
      },
    });

    return { ok: true };
  }
);
