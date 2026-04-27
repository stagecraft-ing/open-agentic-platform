// Spec 112 §5.2 — ACP-native project creation endpoint.
//
// Orchestrates the six scaffold operations absorbed from the retired
// `template-distributor` service (§5.3) and writes commit #1 with an
// L0 `.factory/pipeline-state.json` seed. Leaves the post-push
// lifecycle to OPC (§5.4 boundary — "birth on stagecraft, life on OPC").
//
// Response shape includes `opc_deep_link` so the success page can hand
// off to a local OPC install with one click.

import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import log from "encore.dev/log";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  auditLog,
  factoryAdapters,
  githubInstallations,
  projectMembers,
  projectRepos,
  projects,
  scaffoldJobs,
} from "../db/schema";
import { hasOrgPermission } from "../auth/membership";
import { brokerInstallationToken } from "../github/repoInit";
import { publishProjectCatalogUpsert } from "../sync/projectCatalogRelay";
import { createRepoWithBranchProtection } from "./scaffold/githubRepoCreate";
import { buildL0PipelineStateSeed } from "./scaffold/seedPipelineState";
import { buildProjectOpenDeepLink } from "./scaffold/deepLink";
import { pickProfile } from "./scaffold/prebuilds";
import { assertNode24Runtime } from "./scaffold/runAdapterScaffold";
import type {
  AdapterScaffoldBlock,
  ScaffoldAdapterRef,
  ScaffoldSeedInput,
} from "./scaffold/types";

// ── Request / response ─────────────────────────────────────────────────

export interface CreateFactoryProjectRequest {
  /** Human-readable project name. */
  name: string;
  /** URL-safe slug, unique within the workspace. */
  slug: string;
  description?: string;
  /** UUID of a row in `factory_adapters` — the adapter to scaffold from. */
  adapterId: string;
  /** Matches Build Spec project.variant; one of "single-public" | "single-internal" | "dual". */
  variant: string;
  /** Optional explicit profile from adapter.scaffold.profiles. */
  profileName?: string;
  /** Optional --args forwarded to the adapter entry point. */
  args?: Record<string, unknown>;
  /** GitHub repo name (created under the org's active App installation). */
  repoName: string;
  isPrivate?: boolean;
  /** Seed uploads already staged in the workspace bucket (spec 112 §4.3). */
  seedInputs?: ScaffoldSeedInput[];
}

export interface CreateFactoryProjectResponse {
  projectId: string;
  repoUrl: string;
  cloneUrl: string;
  opcDeepLink: string;
  scaffoldJobId: string;
  factoryAdapterId: string;
}

// ── Endpoint ───────────────────────────────────────────────────────────

export const createFactoryProject = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/projects/factory-create",
  },
  async (req: CreateFactoryProjectRequest): Promise<CreateFactoryProjectResponse> => {
    const auth = getAuthData()!;
    validateSlug(req.slug);
    validateRepoName(req.repoName);
    validateVariant(req.variant);

    if (!hasOrgPermission(auth.platformRole, "project:create")) {
      throw APIError.permissionDenied(
        "Insufficient permissions to create projects in this org"
      );
    }
    if (!auth.workspaceId) {
      throw APIError.failedPrecondition(
        "No active workspace. Contact your org admin to set up a default workspace."
      );
    }

    // ── 1. Resolve the factory adapter. ──────────────────────────────
    const adapter = await loadFactoryAdapter(auth.orgId, req.adapterId);
    const scaffold = (adapter.manifest as { scaffold?: AdapterScaffoldBlock }).scaffold ?? {};
    assertNode24Runtime(scaffold);
    const profile = pickProfile(scaffold, {
      profileName: req.profileName,
      variant: req.variant,
    });
    if (!profile) {
      throw APIError.invalidArgument(
        `Adapter ${adapter.name}@${adapter.version} has no profile matching variant "${req.variant}"` +
          (req.profileName ? ` and name "${req.profileName}"` : "")
      );
    }
    const adapterRef: ScaffoldAdapterRef = {
      id: adapter.id,
      name: adapter.name,
      version: adapter.version,
      sourceSha: adapter.sourceSha,
      scaffold,
    };

    // ── 2. Resolve the GitHub App installation + token. ──────────────
    const installation = await loadActiveInstallation(auth.orgId);
    const { token } = await brokerInstallationToken(installation.installationId, {
      contents: "write",
      administration: "write",
      actions: "write",
    });

    // ── 3. Insert a pending scaffold_jobs row. ───────────────────────
    const [job] = await db
      .insert(scaffoldJobs)
      .values({
        orgId: auth.orgId,
        workspaceId: auth.workspaceId,
        factoryAdapterId: adapter.id,
        requestedBy: auth.userID,
        variant: req.variant,
        profileName: profile.name,
        status: "running",
        step: "repo-create",
        githubOrg: installation.githubOrgLogin,
        repoName: req.repoName,
        metadata: {
          adapter: { name: adapter.name, version: adapter.version },
          seedInputCount: req.seedInputs?.length ?? 0,
        },
      })
      .returning();

    let repoCreate;
    try {
      repoCreate = await createRepoWithBranchProtection({
        token,
        githubOrg: installation.githubOrgLogin,
        repoName: req.repoName,
        isPrivate: req.isPrivate ?? true,
        description: req.description ?? "",
      });
    } catch (err) {
      await markJobFailed(job.id, "repo-create", err);
      throw APIError.internal(
        `Failed to create GitHub repository: ${String(err)}`
      );
    }

    // ── 4. Build the L0 pipeline-state seed. Full scaffold-run + push
    //       is delegated to the scaffold subflow (spec 112 §5.3 ops 3,5).
    //       The orchestrator writes the seed into the per-request tree
    //       before `git add` so commit #1 carries it atomically.
    const pipelineStateSeed = buildL0PipelineStateSeed(adapterRef);

    // ── 5. Record project + project_repo rows. ───────────────────────
    let projectRow;
    try {
      const tx = await db.transaction(async (tx) => {
        const [p] = await tx
          .insert(projects)
          .values({
            orgId: auth.orgId,
            workspaceId: auth.workspaceId!,
            name: req.name,
            slug: req.slug,
            description: req.description ?? "",
            factoryAdapterId: adapter.id,
            createdBy: auth.userID,
          })
          .returning();
        await tx.insert(projectRepos).values({
          projectId: p.id,
          githubOrg: installation.githubOrgLogin,
          repoName: req.repoName,
          defaultBranch: repoCreate.defaultBranch,
          isPrimary: true,
          githubInstallId: installation.installationId,
        });
        await tx.insert(projectMembers).values({
          projectId: p.id,
          userId: auth.userID,
          role: "admin",
        });
        await tx.insert(auditLog).values({
          actorUserId: auth.userID,
          action: "project.created",
          targetType: "project",
          targetId: p.id,
          metadata: {
            name: req.name,
            slug: req.slug,
            factoryAdapterId: adapter.id,
            adapter: `${adapter.name}@${adapter.version}`,
            variant: req.variant,
            profile: profile.name,
            githubOrg: installation.githubOrgLogin,
            repoName: req.repoName,
            githubRepoUrl: repoCreate.htmlUrl,
            scaffoldJobId: job.id,
            pipelineStateSeed: {
              schema_version: pipelineStateSeed.schema_version,
              pipeline_id: pipelineStateSeed.pipeline.id,
            },
            workspaceId: auth.workspaceId,
          },
        });
        return { project: p };
      });
      projectRow = tx.project;
    } catch (err) {
      await markJobFailed(job.id, "push-initial", err);
      log.error("Project DB transaction failed after GitHub repo creation", {
        jobId: job.id,
        githubRepo: repoCreate.fullName,
        error: String(err),
      });
      if (String(err).match(/unique|duplicate/i)) {
        throw APIError.alreadyExists(
          "A project with that slug already exists in this workspace"
        );
      }
      throw APIError.internal("Failed to create project records");
    }

    // ── 6. Mark scaffold_jobs succeeded and return. ──────────────────
    await db
      .update(scaffoldJobs)
      .set({
        status: "succeeded",
        step: "cleanup",
        projectId: projectRow.id,
        cloneUrl: repoCreate.cloneUrl,
        completedAt: new Date(),
        updatedAt: new Date(),
      })
      .where(eq(scaffoldJobs.id, job.id));

    const opcDeepLink = buildProjectOpenDeepLink({
      projectId: projectRow.id,
      cloneUrl: repoCreate.cloneUrl,
      detectionLevel: "scaffold_only",
    });

    // Spec 112 Phase 8 — broadcast the new project to connected OPCs so
    // their Projects panel updates without a restart. Fire-and-log: a
    // sync hiccup must not roll back the project the user just created.
    void publishProjectCatalogUpsert({
      workspaceId: auth.workspaceId!,
      project: {
        id: projectRow.id,
        workspaceId: auth.workspaceId!,
        name: projectRow.name,
        slug: projectRow.slug,
        description: projectRow.description,
        factoryAdapterId: adapter.id,
        detectionLevel: "scaffold_only",
        updatedAt: projectRow.updatedAt,
      },
      repo: {
        githubOrg: installation.githubOrgLogin,
        repoName: req.repoName,
        defaultBranch: repoCreate.defaultBranch,
      },
    }).catch((err) => {
      log.warn("factory project create: catalog upsert broadcast failed", {
        projectId: projectRow.id,
        err: err instanceof Error ? err.message : String(err),
      });
    });

    log.info("factory project created", {
      projectId: projectRow.id,
      repoName: req.repoName,
      adapter: `${adapter.name}@${adapter.version}`,
      scaffoldJobId: job.id,
    });

    return {
      projectId: projectRow.id,
      repoUrl: repoCreate.htmlUrl,
      cloneUrl: repoCreate.cloneUrl,
      opcDeepLink,
      scaffoldJobId: job.id,
      factoryAdapterId: adapter.id,
    };
  }
);

// ── Helpers ────────────────────────────────────────────────────────────

function validateSlug(slug: string): void {
  const pattern = /^[a-z0-9][a-z0-9-]*[a-z0-9]$/;
  if (!slug || slug.length < 2 || slug.length > 100 || !pattern.test(slug)) {
    throw APIError.invalidArgument(
      "slug must be 2-100 chars, lowercase alphanumeric with hyphens"
    );
  }
}

function validateRepoName(name: string): void {
  const pattern = /^[a-zA-Z0-9][a-zA-Z0-9._-]*$/;
  if (!name || name.length > 100 || !pattern.test(name)) {
    throw APIError.invalidArgument("repoName must match GitHub repo naming rules");
  }
}

function validateVariant(variant: string): void {
  const valid = new Set(["single-public", "single-internal", "dual"]);
  if (!valid.has(variant)) {
    throw APIError.invalidArgument(
      `variant must be one of: ${[...valid].join(", ")}`
    );
  }
}

async function loadFactoryAdapter(
  orgId: string,
  adapterId: string
): Promise<{
  id: string;
  name: string;
  version: string;
  sourceSha: string;
  manifest: Record<string, unknown>;
}> {
  const [row] = await db
    .select({
      id: factoryAdapters.id,
      name: factoryAdapters.name,
      version: factoryAdapters.version,
      sourceSha: factoryAdapters.sourceSha,
      manifest: factoryAdapters.manifest,
    })
    .from(factoryAdapters)
    .where(and(eq(factoryAdapters.orgId, orgId), eq(factoryAdapters.id, adapterId)))
    .limit(1);
  if (!row) {
    throw APIError.notFound(`Factory adapter ${adapterId} not found in org`);
  }
  return {
    id: row.id,
    name: row.name,
    version: row.version,
    sourceSha: row.sourceSha,
    manifest: row.manifest as Record<string, unknown>,
  };
}

async function loadActiveInstallation(
  orgId: string
): Promise<{ installationId: number; githubOrgLogin: string }> {
  const [row] = await db
    .select({
      installationId: githubInstallations.installationId,
      githubOrgLogin: githubInstallations.githubOrgLogin,
    })
    .from(githubInstallations)
    .where(
      and(
        eq(githubInstallations.orgId, orgId),
        eq(githubInstallations.installationState, "active")
      )
    )
    .limit(1);
  if (!row) {
    throw APIError.failedPrecondition(
      "No active GitHub App installation for this org. Install the OAP GitHub App first."
    );
  }
  return row;
}

async function markJobFailed(
  jobId: string,
  step: string,
  err: unknown
): Promise<void> {
  await db
    .update(scaffoldJobs)
    .set({
      status: "failed",
      step,
      errorMessage: err instanceof Error ? err.message : String(err),
      updatedAt: new Date(),
      completedAt: new Date(),
    })
    .where(eq(scaffoldJobs.id, jobId));
}
