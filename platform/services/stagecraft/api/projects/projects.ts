import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import log from "encore.dev/log";
import { db } from "../db/drizzle";
import {
  organizations,
  workspaces,
  projects,
  projectRepos,
  environments,
  projectMembers,
  githubInstallations,
  auditLog,
} from "../db/schema";
import { and, eq, desc } from "drizzle-orm";
import { hasOrgPermission } from "../auth/membership";
import {
  brokerInstallationToken,
  createGitHubRepo,
  seedRepoFromAdapter,
  configureBranchProtection,
  createOapWorkflow,
} from "../github/repoInit";

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
  factoryAdapterId: string | null;
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

    // FR-007: Only admin/owner can delete projects
    if (!hasOrgPermission(auth.platformRole, "project:delete")) {
      throw APIError.permissionDenied("Insufficient permissions to delete projects");
    }

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
// Self-Service Project Creation (spec 080 Phase 2 — FR-006)
// ---------------------------------------------------------------------------

type CreateProjectWithRepoRequest = {
  name: string;
  slug: string;
  description?: string;
  adapter: "aim-vue-node" | "encore-react" | "next-prisma" | "rust-axum";
  repoName: string;
  isPrivate?: boolean;
};

type CreateProjectWithRepoResponse = {
  project: ProjectRow;
  repo: RepoRow;
  environments: EnvironmentRow[];
  githubRepoUrl: string;
};

export const createProjectWithRepo = api(
  { expose: true, auth: true, method: "POST", path: "/api/projects/with-repo" },
  async (req: CreateProjectWithRepoRequest): Promise<CreateProjectWithRepoResponse> => {
    const auth = getAuthData()!;

    log.info("createProjectWithRepo invoked", {
      userID: auth.userID,
      orgId: auth.orgId,
      workspaceId: auth.workspaceId,
      platformRole: auth.platformRole,
      slug: req.slug,
      repoName: req.repoName,
      adapter: req.adapter,
    });

    // Validate inputs
    if (!req.name || !req.slug || !req.repoName || !req.adapter) {
      throw APIError.invalidArgument("name, slug, repoName, and adapter are required");
    }

    // Sanitize slug and repoName to prevent path traversal/injection
    const slugPattern = /^[a-z0-9][a-z0-9-]*[a-z0-9]$/;
    if (req.slug.length < 2 || req.slug.length > 100 || !slugPattern.test(req.slug)) {
      throw APIError.invalidArgument("slug must be 2-100 chars, lowercase alphanumeric with hyphens");
    }
    const repoPattern = /^[a-zA-Z0-9][a-zA-Z0-9._-]*$/;
    if (req.repoName.length > 100 || !repoPattern.test(req.repoName)) {
      throw APIError.invalidArgument("repoName must match GitHub repo naming rules");
    }

    // FR-007: Check org-level permission
    if (!hasOrgPermission(auth.platformRole, "project:create")) {
      throw APIError.permissionDenied("Insufficient permissions to create projects in this org");
    }

    // Validate workspace context
    if (!auth.workspaceId) {
      throw APIError.failedPrecondition(
        "No active workspace. Contact your org admin to set up a default workspace."
      );
    }

    // Look up the active GitHub App installation for this org
    const [installation] = await db
      .select({
        installationId: githubInstallations.installationId,
        githubOrgLogin: githubInstallations.githubOrgLogin,
      })
      .from(githubInstallations)
      .where(
        and(
          eq(githubInstallations.orgId, auth.orgId),
          eq(githubInstallations.installationState, "active")
        )
      )
      .limit(1);

    if (!installation) {
      throw APIError.failedPrecondition(
        "No active GitHub App installation for this org. Install the OAP GitHub App first."
      );
    }

    // Broker a scoped installation token with permissions for repo init
    let installToken: string;
    try {
      ({ token: installToken } = await brokerInstallationToken(
        installation.installationId,
        {
          contents: "write",
          administration: "write",
          actions: "write",
        }
      ));
    } catch (err) {
      const msg = String(err);
      log.error("brokerInstallationToken failed", {
        installationId: installation.installationId,
        githubOrg: installation.githubOrgLogin,
        error: msg,
      });
      if (msg.includes("permissions requested are not granted")) {
        throw APIError.failedPrecondition(
          `GitHub App installation on ${installation.githubOrgLogin} is missing required permissions (contents: write, administration: write). An org admin must update the app's permissions at github.com/organizations/${installation.githubOrgLogin}/settings/installations/${installation.installationId} and approve the changes.`
        );
      }
      throw APIError.internal(`Failed to obtain GitHub installation token: ${msg}`);
    }

    // FR-008: Create GitHub repo
    let repoResult;
    try {
      repoResult = await createGitHubRepo(
        installToken,
        installation.githubOrgLogin,
        req.repoName,
        {
          isPrivate: req.isPrivate ?? true,
          description: req.description ?? "",
        }
      );
    } catch (err) {
      const msg = String(err);
      if (msg.includes("already exists")) {
        throw APIError.alreadyExists(`Repository ${installation.githubOrgLogin}/${req.repoName} already exists`);
      }
      log.error("GitHub repo creation failed", { error: msg });
      throw APIError.internal("Failed to create GitHub repository");
    }

    // FR-008: Seed adapter template, branch protection, workflow
    // These are best-effort — repo creation succeeded, so we continue even if these fail
    await seedRepoFromAdapter(installToken, repoResult.fullName, req.adapter);
    await configureBranchProtection(installToken, repoResult.fullName, repoResult.defaultBranch);
    await createOapWorkflow(installToken, repoResult.fullName);

    // FR-006: Create DB records atomically within a transaction.
    // Note: GitHub repo creation (above) cannot be rolled back. If the DB
    // transaction fails, the GitHub repo is orphaned. This is logged so an
    // operator can clean up manually.
    let project: ProjectRow;
    let repo: RepoRow;
    let envRows: EnvironmentRow[];

    try {
      const txResult = await db.transaction(async (tx) => {
        // Insert project
        const [projectRow] = await tx
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

        // Insert project repo
        const [repoRow] = await tx
          .insert(projectRepos)
          .values({
            projectId: projectRow.id,
            githubOrg: installation.githubOrgLogin,
            repoName: req.repoName,
            defaultBranch: repoResult.defaultBranch,
            isPrimary: true,
            githubInstallId: installation.installationId,
          })
          .returning();

        // Insert default environments
        const envDefs: Array<{
          name: string;
          kind: "development" | "staging" | "production";
          autoDeployBranch: string | null;
          requiresApproval: boolean;
        }> = [
          { name: "development", kind: "development", autoDeployBranch: null, requiresApproval: false },
          { name: "staging", kind: "staging", autoDeployBranch: null, requiresApproval: false },
          { name: "production", kind: "production", autoDeployBranch: null, requiresApproval: true },
        ];

        const txEnvRows: EnvironmentRow[] = [];
        for (const envDef of envDefs) {
          const k8sNamespace = `proj--${req.slug}--${envDef.name}`;
          const [env] = await tx
            .insert(environments)
            .values({
              projectId: projectRow.id,
              name: envDef.name,
              kind: envDef.kind,
              k8sNamespace,
              autoDeployBranch: envDef.autoDeployBranch,
              requiresApproval: envDef.requiresApproval,
            })
            .returning();
          txEnvRows.push(env);
        }

        // Add creator as project admin
        await tx.insert(projectMembers).values({
          projectId: projectRow.id,
          userId: auth.userID,
          role: "admin",
        });

        // Audit log
        await tx.insert(auditLog).values({
          actorUserId: auth.userID,
          action: "project.created",
          targetType: "project",
          targetId: projectRow.id,
          metadata: {
            name: req.name,
            slug: req.slug,
            adapter: req.adapter,
            repoName: req.repoName,
            githubOrg: installation.githubOrgLogin,
            githubRepoUrl: repoResult.htmlUrl,
            workspaceId: auth.workspaceId,
          },
        });

        return { project: projectRow, repo: repoRow, envRows: txEnvRows };
      });

      project = txResult.project;
      repo = txResult.repo;
      envRows = txResult.envRows;
    } catch (err) {
      const msg = String(err);
      if (msg.includes("unique") || msg.includes("duplicate")) {
        throw APIError.alreadyExists(
          "A project with that slug already exists in this workspace"
        );
      }
      log.error("Project DB transaction failed after GitHub repo creation", {
        error: msg,
        githubRepo: repoResult.fullName,
        githubRepoUrl: repoResult.htmlUrl,
      });
      throw APIError.internal("Failed to create project records");
    }

    return {
      project,
      repo,
      environments: envRows,
      githubRepoUrl: repoResult.htmlUrl,
    };
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
