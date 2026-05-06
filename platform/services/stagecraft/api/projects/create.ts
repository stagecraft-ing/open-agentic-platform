// Spec 112 §5.2 — ACP-native project creation endpoint.
//
// Drives the six absorbed scaffold operations (§5.3) end-to-end:
//
//   1. Resolve adapter + verify warmup readiness.
//   2. Insert a scaffold_jobs row in `running` state.
//   3. Create the GitHub repo (auto_init=false → commit #1 is ours).
//   4. Run the per-request scaffold against the prebuilt profile, copy
//      into a temp dir, install user-selected extras, write the L0
//      pipeline-state seed.
//   5. Run server-side artifact extraction (when seed inputs supplied).
//   6. Init + commit + push to the new GitHub repo.
//   7. Insert projects / project_repos / project_members / environments
//      rows + audit event in one transaction.
//   8. Mark the job succeeded and clean up the local working tree.
//
// Failure handling (spec 112 §10):
//   - Pre-repo-create failures (validation, no PAT, warmup not ready)
//     surface as APIError.failedPrecondition / invalidArgument with the
//     actual cause — never the Encore-wrapped "an internal error occurred".
//   - Post-repo-create failures mark the scaffold_jobs row `orphaned` so
//     an operator can reclaim the empty repo. The error is rethrown as
//     APIError.internal with the underlying message attached.

import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import log from "encore.dev/log";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  auditLog,
  environments,
  githubInstallations,
  projectMembers,
  projectRepos,
  projects,
  scaffoldJobs,
} from "../db/schema";
import { hasOrgPermission } from "../auth/membership";
import { brokerInstallationToken } from "../github/repoInit";
import { loadFactoryUpstreamPatToken } from "../factory/upstreamPat";
import { loadSubstrateForOrg } from "../factory/substrateBrowser";
import { projectSubstrateToLegacy } from "../factory/projection";
import { publishProjectCatalogUpsert } from "../sync/projectCatalogRelay";
import { createRepoWithBranchProtection } from "./scaffold/githubRepoCreate";
import { gitInitAndPush } from "./scaffold/gitInitAndPush";
import { scaffoldFromPrebuilt } from "./scaffold/perRequestScaffold";
import { buildL0PipelineStateSeed } from "./scaffold/seedPipelineState";
import { buildProjectOpenDeepLink } from "./scaffold/deepLink";
import { extractArtifactsIntoTree } from "./scaffold/artifactExtract";
import { resolveScaffoldUpstream } from "./scaffold/scheduler";
import {
  getInitStatus,
  isTemplateCacheReady,
  isTemplateCacheRefreshing,
  defaultWorkspaceDir,
} from "./scaffold/templateCache";
import {
  isKnownModule,
  pickProfileFromModules,
} from "./scaffold/moduleCatalog";
import type {
  ScaffoldAdapterRef,
  ScaffoldSeedInput,
  ScaffoldStep,
} from "./scaffold/types";

// ── Request / response ─────────────────────────────────────────────────

export interface CreateFactoryProjectRequest {
  /** Human-readable project name. */
  name: string;
  /** URL-safe slug, unique within the org. */
  slug: string;
  description?: string;
  /** UUID of a row in `factory_adapters` — the adapter to scaffold from. */
  adapterId: string;
  /** Matches Build Spec project.variant; one of "single-public" | "single-internal" | "dual". */
  variant: string;
  /** Modules selected from MODULE_CATALOG. Filtered server-side. */
  modules?: string[];
  /** GitHub repo name (created under the org's active App installation). */
  repoName: string;
  isPrivate?: boolean;
  /** Seed uploads already staged in the project bucket (spec 112 §4.3). */
  seedInputs?: ScaffoldSeedInput[];
}

export interface CreateFactoryProjectResponse {
  projectId: string;
  repoUrl: string;
  cloneUrl: string;
  opcDeepLink: string;
  scaffoldJobId: string;
  factoryAdapterId: string;
  /** Spec 112 §11 / Phase 7 — auto-provisioned dev environment row. */
  devEnvironmentId: string;
  /** Profile chosen by pickProfileFromModules (informational, for UI). */
  profile: string;
}

// ── Endpoint ───────────────────────────────────────────────────────────

export const createFactoryProject = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/projects/factory-create",
  },
  async (
    req: CreateFactoryProjectRequest
  ): Promise<CreateFactoryProjectResponse> => {
    const auth = getAuthData()!;
    validateSlug(req.slug);
    validateRepoName(req.repoName);
    validateVariant(req.variant);
    const selectedModules = (req.modules ?? []).filter(isKnownModule);

    if (!hasOrgPermission(auth.platformRole, "project:create")) {
      throw APIError.permissionDenied(
        "Insufficient permissions to create projects in this org"
      );
    }

    // ── 1. Warmup readiness — Create blocks if cache/prebuilds aren't up. ──
    const status = getInitStatus();
    if (isTemplateCacheRefreshing()) {
      throw APIError.failedPrecondition(
        `Template cache is still initializing (step: ${status.step}, ${status.progress}%). ` +
          `Please retry in a couple of minutes.`
      );
    }
    if (!isTemplateCacheReady() || !status.ready) {
      throw APIError.failedPrecondition(
        status.error
          ? `Scaffold infrastructure not ready: ${status.error}`
          : `Scaffold infrastructure not ready (step: ${status.step}, ${status.progress}%). ` +
              `Wait for warmup to complete and retry.`
      );
    }

    // ── 2. Resolve the factory adapter + verify scaffold_source_id. ───
    // Spec 140 §2.2 — adapter manifest carries `scaffold_source_id`
    // (an id, not a URL). The actual clone target lives in
    // `factory_upstreams` for the same org and is resolved by
    // `scheduler.ts::resolveScaffoldUpstream` at warmup time.
    const adapter = await loadFactoryAdapter(auth.orgId, req.adapterId);
    const manifest = adapter.manifest;

    // Spec 112 §10 — Create-eligibility gate. Adapters that declare a
    // non-Node-24 scaffold runtime are not executable on stagecraft (web
    // UI is Node-24-only); they need OPC-side dispatch via spec 110.
    // The check accepts both `scaffold.runtime` (legacy block) and
    // `scaffold_runtime` (spec 139 §7.2 / spec 140 §2.1 top-level).
    const declaredRuntime =
      (manifest as { scaffold?: { runtime?: string } }).scaffold?.runtime ??
      (typeof (manifest as { scaffold_runtime?: unknown }).scaffold_runtime ===
      "string"
        ? ((manifest as { scaffold_runtime: string }).scaffold_runtime)
        : undefined);
    if (declaredRuntime && declaredRuntime !== "node-24") {
      throw APIError.failedPrecondition(
        `Factory adapter ${adapter.name}@${adapter.version} declares scaffold runtime "${declaredRuntime}". ` +
          `Stagecraft Create executes Node-24 adapters only (spec 112 §10); ` +
          `non-Node-24 scaffolds must dispatch to OPC.`
      );
    }

    const scaffoldSourceId =
      typeof (manifest as { scaffold_source_id?: unknown }).scaffold_source_id ===
      "string"
        ? ((manifest as { scaffold_source_id: string }).scaffold_source_id)
        : "";
    if (!scaffoldSourceId) {
      throw APIError.failedPrecondition(
        `Factory adapter ${adapter.name}@${adapter.version} has no scaffold_source_id in its manifest. ` +
          `Re-run /factory-sync to repopulate the adapter row (spec 139 §7.2 / spec 140 §2.1).`
      );
    }

    const upstream = await resolveScaffoldUpstream(auth.orgId, scaffoldSourceId);
    if (!upstream) {
      throw APIError.failedPrecondition(
        `Factory adapter ${adapter.name}@${adapter.version} declares scaffold_source_id "${scaffoldSourceId}" but no factory_upstreams row matches it. ` +
          `Register the upstream at /app/factory/upstreams before creating projects (spec 139 §7.2).`
      );
    }

    // ── 3. Verify the org has an upstream PAT (used for warmup + clone). ─
    const upstreamPat = await loadFactoryUpstreamPatToken(auth.orgId);
    if (!upstreamPat) {
      throw APIError.failedPrecondition(
        "No factory upstream PAT configured for this org. " +
          "Add one at /app/admin/factory/pat before creating projects."
      );
    }

    const profile = pickProfileFromModules(req.variant, selectedModules);

    const adapterRef: ScaffoldAdapterRef = {
      id: adapter.id,
      name: adapter.name,
      version: adapter.version,
      sourceSha: adapter.sourceSha,
    };

    // ── 4. Resolve the GitHub App installation + token. ───────────────
    const installation = await loadActiveInstallation(auth.orgId);
    const { token } = await brokerInstallationToken(installation.installationId, {
      contents: "write",
      administration: "write",
      actions: "write",
      workflows: "write",
    });

    // ── 5. Insert a running scaffold_jobs row. ────────────────────────
    const [job] = await db
      .insert(scaffoldJobs)
      .values({
        orgId: auth.orgId,
        factoryAdapterId: adapter.id,
        requestedBy: auth.userID,
        variant: req.variant,
        profileName: profile,
        status: "running",
        step: "repo-create",
        githubOrg: installation.githubOrgLogin,
        repoName: req.repoName,
        metadata: {
          adapter: { name: adapter.name, version: adapter.version },
          selectedModules,
          seedInputCount: req.seedInputs?.length ?? 0,
        },
      })
      .returning();

    // ── 6. Create the GitHub repo. (Past this point, failures = orphan.) ─
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
      // Pre-repo failure — no orphan to leave behind.
      const msg = err instanceof Error ? err.message : String(err);
      if (/already exists/i.test(msg)) {
        throw APIError.alreadyExists(
          `GitHub repository ${installation.githubOrgLogin}/${req.repoName} already exists`
        );
      }
      throw APIError.internal(`Failed to create GitHub repository: ${msg}`);
    }

    // ── 7. Per-request scaffold + seed write + push. ──────────────────
    const workspace = defaultWorkspaceDir();
    const tmpRoot = await mkdtemp(join(tmpdir(), "stagecraft-create-"));
    const projectRoot = join(tmpRoot, req.repoName);

    const pipelineStateSeed = buildL0PipelineStateSeed(adapterRef);
    let commitSha = "";

    try {
      await advanceStep(job.id, "run-entry");
      await scaffoldFromPrebuilt({
        workspaceDir: workspace,
        profile,
        selectedModules,
        destDir: projectRoot,
        pipelineStateSeed: pipelineStateSeed as unknown as Record<string, unknown>,
      });

      if (req.seedInputs && req.seedInputs.length > 0) {
        await advanceStep(job.id, "extract-artifacts");
        await extractArtifactsIntoTree({
          projectRoot,
          inputs: req.seedInputs,
        });
      }

      await advanceStep(job.id, "push-initial");
      const pushed = await gitInitAndPush({
        projectRoot,
        githubOrg: installation.githubOrgLogin,
        repoName: req.repoName,
        token,
        branch: repoCreate.defaultBranch,
        commitMessage: "Initial commit",
        authorName: "Stagecraft",
        authorEmail: "noreply@stagecraft.ing",
      });
      commitSha = pushed.commitSha;
    } catch (err) {
      await markJobOrphaned(
        job.id,
        installation.githubOrgLogin,
        req.repoName,
        err
      );
      await rm(tmpRoot, { recursive: true, force: true }).catch(() => {});
      const msg = err instanceof Error ? err.message : String(err);
      throw APIError.internal(`Scaffold/push failed: ${msg}`);
    }

    // ── 8. DB transaction: project + repo + member + env + audit. ─────
    let projectRow: typeof projects.$inferSelect;
    let devEnvRow: typeof environments.$inferSelect;
    try {
      const tx = await db.transaction(async (tx) => {
        const [p] = await tx
          .insert(projects)
          .values({
            orgId: auth.orgId,
            name: req.name,
            slug: req.slug,
            description: req.description ?? "",
            objectStoreBucket: `oap-${auth.orgSlug || "unknown"}-${req.slug}`,
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
        // Spec 112 Phase 7 — auto-provision the development env row so
        // POST /api/projects/:id/factory/deploy and the PR webhook path
        // both target a pre-existing row instead of one path lazy-creating
        // and the other not. Pure DB insert; no namespace is created on
        // the cluster until something actually deploys.
        const namespaceSlug = `oap-${auth.orgSlug || "unknown"}-${req.slug}-dev`;
        const [devEnv] = await tx
          .insert(environments)
          .values({
            projectId: p.id,
            name: "development",
            kind: "development",
            k8sNamespace: namespaceSlug,
            autoDeployBranch: repoCreate.defaultBranch,
            requiresApproval: false,
          })
          .returning();
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
            profile,
            selectedModules,
            githubOrg: installation.githubOrgLogin,
            repoName: req.repoName,
            githubRepoUrl: repoCreate.htmlUrl,
            commitSha,
            scaffoldJobId: job.id,
            devEnvironmentId: devEnv.id,
            pipelineStateSeed: {
              schema_version: pipelineStateSeed.schema_version,
              pipeline_id: pipelineStateSeed.pipeline.id,
            },
            orgId: auth.orgId,
          },
        });
        return { project: p, devEnv };
      });
      projectRow = tx.project;
      devEnvRow = tx.devEnv;
    } catch (err) {
      await markJobOrphaned(
        job.id,
        installation.githubOrgLogin,
        req.repoName,
        err
      );
      await rm(tmpRoot, { recursive: true, force: true }).catch(() => {});
      log.error("Project DB transaction failed after GitHub push", {
        jobId: job.id,
        githubRepo: repoCreate.fullName,
        error: err instanceof Error ? err.message : String(err),
      });
      const msg = err instanceof Error ? err.message : String(err);
      if (/unique|duplicate/i.test(msg)) {
        throw APIError.alreadyExists(
          "A project with that slug already exists in this org"
        );
      }
      throw APIError.internal(`Failed to record project: ${msg}`);
    }

    // ── 9. Mark scaffold_jobs succeeded + clean up the working tree. ──
    await db
      .update(scaffoldJobs)
      .set({
        status: "succeeded",
        step: "cleanup",
        projectId: projectRow.id,
        cloneUrl: repoCreate.cloneUrl,
        commitSha,
        completedAt: new Date(),
        updatedAt: new Date(),
      })
      .where(eq(scaffoldJobs.id, job.id));
    await rm(tmpRoot, { recursive: true, force: true }).catch((err) => {
      log.warn("scaffold cleanup failed (non-fatal)", {
        tmpRoot,
        error: err instanceof Error ? err.message : String(err),
      });
    });

    const opcDeepLink = buildProjectOpenDeepLink({
      projectId: projectRow.id,
      cloneUrl: repoCreate.cloneUrl,
      detectionLevel: "scaffold_only",
    });

    // Spec 112 Phase 8 — broadcast the new project to connected OPCs.
    // Fire-and-log: a sync hiccup must not roll back the project we
    // just created.
    void publishProjectCatalogUpsert({
      orgId: auth.orgId,
      project: {
        id: projectRow.id,
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
      profile,
      commitSha,
      devEnvironmentId: devEnvRow.id,
      scaffoldJobId: job.id,
    });

    return {
      projectId: projectRow.id,
      repoUrl: repoCreate.htmlUrl,
      cloneUrl: repoCreate.cloneUrl,
      opcDeepLink,
      scaffoldJobId: job.id,
      factoryAdapterId: adapter.id,
      devEnvironmentId: devEnvRow.id,
      profile,
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
    throw APIError.invalidArgument(
      "repoName must match GitHub repo naming rules"
    );
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
  // Spec 139 Phase 4 (T091): adapter manifests live in
  // `factory_artifact_substrate`. The synthesised id matches
  // `browse.ts::synthesiseId` so consumers that received the id from
  // listAdapters can use it here unchanged. Adapters resolve by name
  // when the synthesised id matches a projection-emitted adapter.
  const substrate = await loadSubstrateForOrg(orgId);
  const projection = projectSubstrateToLegacy(substrate);
  const found = projection.adapters.find(
    (a) => synthesiseAdapterId(orgId, a.name) === adapterId,
  );
  if (!found) {
    throw APIError.notFound(`Factory adapter ${adapterId} not found in org`);
  }
  return {
    id: adapterId,
    name: found.name,
    version: found.version,
    sourceSha: found.sourceSha,
    manifest: found.manifest as Record<string, unknown>,
  };
}

/** Spec 139 Phase 4 — must match `browse.ts::synthesiseId` (substrate cutover). */
function synthesiseAdapterId(orgId: string, name: string): string {
  return `synthetic-adapter-${orgId.slice(0, 8)}-${name}`;
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

async function advanceStep(jobId: string, step: ScaffoldStep): Promise<void> {
  await db
    .update(scaffoldJobs)
    .set({ step, updatedAt: new Date() })
    .where(eq(scaffoldJobs.id, jobId));
}

async function markJobFailed(
  jobId: string,
  step: ScaffoldStep,
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

/**
 * Spec 112 §10 — orphan-repo recovery. Past `repo-create` we have an empty
 * GitHub repo; mark the job `orphaned` so an operator can reclaim it via
 * the admin UI rather than letting the empty repo sit silently.
 */
async function markJobOrphaned(
  jobId: string,
  githubOrg: string,
  repoName: string,
  err: unknown
): Promise<void> {
  await db
    .update(scaffoldJobs)
    .set({
      status: "orphaned",
      errorMessage: err instanceof Error ? err.message : String(err),
      githubOrg,
      repoName,
      updatedAt: new Date(),
      completedAt: new Date(),
    })
    .where(eq(scaffoldJobs.id, jobId));
}
