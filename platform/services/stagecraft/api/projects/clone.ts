// Spec 113 §FR-023..FR-038 — clone an existing project's GitHub repo into
// the caller's current OAP org's GitHub installation, register a new
// stagecraft project bound to that repo, and hydrate raw artefacts so the
// new project starts operational on day 1.
//
// The endpoint is the substantive piece of the Clone feature; pure
// helpers + shell wrappers live in `cloneHelpers.ts` so vitest can pin
// the parts that don't need the Encore runtime.

import { rm } from "node:fs/promises";
import { dirname } from "node:path";
import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import log from "encore.dev/log";
import { and, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  auditLog,
  githubInstallations,
  projectMembers,
  projectRepos,
  projects,
} from "../db/schema";
import { hasOrgPermission } from "../auth/membership";
import { brokerInstallationToken } from "../github/repoInit";
import { resolveProjectToken } from "./tokenResolver";
import { publishProjectCatalogUpsert } from "../sync/projectCatalogRelay";
import { buildProjectOpenDeepLink } from "./scaffold/deepLink";
import { registerRawArtifactsFromRepo } from "./importArtifacts";
import {
  addWorktree,
  createCloneDestRepo,
  defaultProjectSlug,
  defaultRepoName,
  deleteGithubRepo,
  isOverSizeCap,
  measureBytes,
  measureCommitCount,
  mirrorClone,
  mirrorPush,
  probeSourceRepo,
  readDefaultBranch,
  resolveSizeCaps,
  setDefaultBranch,
  suffixCandidate,
} from "./cloneHelpers";
import { checkSlugAvailable } from "./cloneAvailability";
import {
  checkRepoAvailable,
  isValidGithubRepoName,
  isValidProjectSlug,
} from "./cloneAvailabilityHelpers";

const MAX_SUFFIX_ATTEMPTS = 25;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface CloneProjectRequest {
  sourceProjectId: string;
  name?: string;
  slug?: string;
  repoName?: string;
}

export interface CloneProjectResponse {
  projectId: string;
  name: string;
  slug: string;
  repoFullName: string;
  defaultBranch: string;
  opcDeepLink: string | null;
  rawArtifactsCopied: number;
  rawArtifactsSkipped: number;
  durationMs: number;
}

// ---------------------------------------------------------------------------
// Endpoint
// ---------------------------------------------------------------------------

export const cloneProject = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/projects/:sourceProjectId/clone",
  },
  async (req: CloneProjectRequest): Promise<CloneProjectResponse> => {
    const startedAt = Date.now();
    const auth = getAuthData()!;

    // FR-024 — `org:project.create` gate (same as factory-create).
    if (!hasOrgPermission(auth.platformRole, "project:create")) {
      throw APIError.permissionDenied(
        "Insufficient permissions to clone projects in this org"
      );
    }
    if (!auth.workspaceId) {
      throw APIError.failedPrecondition(
        "No active workspace. Contact your org admin to set up a default workspace."
      );
    }

    // Load source project + its primary repo. Workspace-scoped to enforce
    // read access on the source via the standard membership resolver.
    const sourceRows = await db
      .select({
        id: projects.id,
        name: projects.name,
        slug: projects.slug,
        description: projects.description,
        factoryAdapterId: projects.factoryAdapterId,
        workspaceId: projects.workspaceId,
        orgId: projects.orgId,
      })
      .from(projects)
      .where(
        and(
          eq(projects.id, req.sourceProjectId),
          eq(projects.workspaceId, auth.workspaceId)
        )
      )
      .limit(1);
    const source = sourceRows[0];
    if (!source) {
      throw APIError.notFound("source project not found in this workspace");
    }

    const sourceRepoRows = await db
      .select({
        githubOrg: projectRepos.githubOrg,
        repoName: projectRepos.repoName,
        defaultBranch: projectRepos.defaultBranch,
      })
      .from(projectRepos)
      .where(
        and(
          eq(projectRepos.projectId, source.id),
          eq(projectRepos.isPrimary, true)
        )
      )
      .limit(1);
    const sourceRepo = sourceRepoRows[0];
    if (!sourceRepo) {
      throw APIError.failedPrecondition(
        "source_repo_missing: source project has no primary repo"
      );
    }

    // FR-025 — destination installation lookup. The cloned repo always
    // lands under the OAP-current-org's installation login.
    const [destInstallation] = await db
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
    if (!destInstallation) {
      throw APIError.failedPrecondition(
        "no_github_installation: no active GitHub App installation for this org"
      );
    }

    // FR-026 — source-side token. May be an installation token (preferred)
    // or a project PAT, depending on which org the source repo lives in.
    const sourceToken = await resolveProjectToken({
      orgId: auth.orgId,
      projectId: source.id,
      targetGithubOrgLogin: sourceRepo.githubOrg,
      permissions: { contents: "read", metadata: "read" },
    });
    if (!sourceToken) {
      throw APIError.permissionDenied(
        "source_unauthorized: no usable GitHub token for the source repo"
      );
    }

    // Probe source repo for privacy + canonical default branch (FR-028).
    const sourceProbe = await probeSourceRepo(
      sourceToken.token,
      sourceRepo.githubOrg,
      sourceRepo.repoName
    );
    const sourceIsPrivate = sourceProbe?.isPrivate ?? true;

    // ──────────────────────────────────────────────────────────────────────
    // Mirror clone + size cap
    // ──────────────────────────────────────────────────────────────────────

    let mirrorPath: string;
    try {
      mirrorPath = await mirrorClone(
        sourceToken.token,
        sourceRepo.githubOrg,
        sourceRepo.repoName
      );
    } catch (err) {
      throw APIError.internal(
        `failed to mirror-clone source repo: ${errorMessage(err)}`
      );
    }
    let cleanupMirror = mirrorPath;
    let cleanupWorktree: string | null = null;

    try {
      const sourceDefaultBranch =
        sourceProbe?.defaultBranch ??
        sourceRepo.defaultBranch ??
        (await readDefaultBranch(mirrorPath));

      const { maxBytes, maxCommits } = resolveSizeCaps(process.env);
      const [bytes, commits] = await Promise.all([
        measureBytes(mirrorPath),
        measureCommitCount(mirrorPath),
      ]);
      const cap = isOverSizeCap({ bytes, commits, maxBytes, maxCommits });
      if (cap.over) {
        throw APIError.failedPrecondition(
          `source_too_large: source ${cap.reason} exceeds cap (${
            cap.reason === "bytes" ? `${bytes}B > ${maxBytes}B` : `${commits} > ${maxCommits} commits`
          })`
        );
      }

      // ────────────────────────────────────────────────────────────────────
      // Final-name resolution (FR-029, FR-030)
      // ────────────────────────────────────────────────────────────────────

      // Destination installation token — used for repo create, push, and
      // `default_branch` PATCH. Source token is unrelated (different org).
      let destToken: string;
      try {
        ({ token: destToken } = await brokerInstallationToken(
          destInstallation.installationId,
          {
            contents: "write",
            administration: "write",
            metadata: "read",
          }
        ));
      } catch (err) {
        throw APIError.internal(
          `failed to broker destination installation token: ${errorMessage(err)}`
        );
      }

      const baseRepoName = defaultRepoName(sourceRepo.repoName);
      const userRepoName = req.repoName?.trim();
      const repoNameIsDefault =
        userRepoName === undefined || userRepoName === baseRepoName;
      if (userRepoName !== undefined && !isValidGithubRepoName(userRepoName)) {
        throw APIError.invalidArgument(
          "invalid_repo_name: repoName must match GitHub naming rules"
        );
      }
      const requestedRepoName = userRepoName ?? baseRepoName;

      const finalRepoName = await resolveFinalRepoName({
        token: destToken,
        githubOrgLogin: destInstallation.githubOrgLogin,
        baseName: baseRepoName,
        userTyped: !repoNameIsDefault ? userRepoName : null,
      });

      const baseSlug = defaultProjectSlug(source.slug);
      const userSlug = req.slug?.trim();
      const slugIsDefault = userSlug === undefined || userSlug === baseSlug;
      if (userSlug !== undefined && !isValidProjectSlug(userSlug)) {
        throw APIError.invalidArgument(
          "invalid_slug: slug must match ^[a-z0-9][a-z0-9-]{0,62}$"
        );
      }
      const requestedSlug = userSlug ?? baseSlug;

      const finalSlug = await resolveFinalSlug({
        workspaceId: auth.workspaceId,
        baseSlug,
        userTyped: !slugIsDefault ? userSlug : null,
      });

      const finalName =
        req.name?.trim() && req.name.trim().length > 0
          ? req.name.trim()
          : `${source.name} (clone)`;

      // ────────────────────────────────────────────────────────────────────
      // Create destination repo + mirror push
      // ────────────────────────────────────────────────────────────────────

      const destRepo = await createCloneDestRepo(
        destToken,
        destInstallation.githubOrgLogin,
        finalRepoName,
        {
          isPrivate: sourceIsPrivate,
          description: source.description ?? "",
        }
      );

      try {
        const authedRemote = `https://x-access-token:${destToken}@github.com/${destRepo.fullName}.git`;
        await mirrorPush(mirrorPath, authedRemote);

        // FR-028 — pin destination's default branch to source's, so the
        // landing page is consistent regardless of GitHub's auto-init
        // default. Best-effort; non-2xx is logged and swallowed.
        await setDefaultBranch(destToken, destRepo.fullName, sourceDefaultBranch);
      } catch (err) {
        // Push failed → delete the destination repo, no DB writes.
        await deleteGithubRepo(destToken, destRepo.fullName);
        throw APIError.internal(
          `failed to push mirror to destination repo: ${errorMessage(err)}`
        );
      }

      // ────────────────────────────────────────────────────────────────────
      // DB writes
      // ────────────────────────────────────────────────────────────────────

      let projectRow: { id: string; updatedAt: Date };
      try {
        projectRow = await db.transaction(async (tx) => {
          const [p] = await tx
            .insert(projects)
            .values({
              orgId: auth.orgId,
              workspaceId: auth.workspaceId,
              name: finalName,
              slug: finalSlug,
              description: source.description ?? "",
              factoryAdapterId: source.factoryAdapterId,
              createdBy: auth.userID,
            })
            .returning({
              id: projects.id,
              updatedAt: projects.updatedAt,
            });

          await tx.insert(projectRepos).values({
            projectId: p.id,
            githubOrg: destInstallation.githubOrgLogin,
            repoName: finalRepoName,
            defaultBranch: sourceDefaultBranch,
            isPrimary: true,
            githubInstallId: destInstallation.installationId,
          });

          // FR-037 — only the cloning user is bound to the new project.
          // Members, environments, and PATs are governance state and are
          // not copied.
          await tx.insert(projectMembers).values({
            projectId: p.id,
            userId: auth.userID,
            role: "admin",
          });

          return p;
        });
      } catch (err) {
        // DB insert failed → delete destination repo, surface the error.
        await deleteGithubRepo(destToken, destRepo.fullName);
        const msg = errorMessage(err);
        if (/unique|duplicate/i.test(msg)) {
          throw APIError.alreadyExists(
            `slug_taken: slug "${finalSlug}" already exists in this workspace`
          );
        }
        throw APIError.internal(`failed to register cloned project: ${msg}`);
      }

      // ────────────────────────────────────────────────────────────────────
      // Raw-artefact hydration (best-effort, partial success allowed)
      // ────────────────────────────────────────────────────────────────────

      let rawCopied = 0;
      let rawSkipped = 0;
      try {
        const wt = await addWorktree(mirrorPath, sourceDefaultBranch);
        cleanupWorktree = wt;
        const result = await registerRawArtifactsFromRepo({
          projectId: projectRow.id,
          workspaceId: auth.workspaceId,
          boundBy: auth.userID,
          repoRoot: wt,
          sourceRepo: `${sourceRepo.githubOrg}/${sourceRepo.repoName}`,
        });
        rawCopied = result.registered.length;
        rawSkipped = result.skipped.length;
      } catch (err) {
        log.warn("clone: raw artefact hydration failed — continuing", {
          projectId: projectRow.id,
          err: errorMessage(err),
        });
        rawSkipped = -1; // sentinel: the walk itself failed
      }

      // FR-035 — audit row carrying both requested and final names.
      const durationMs = Date.now() - startedAt;
      await db.insert(auditLog).values({
        actorUserId: auth.userID,
        action: "project.cloned",
        targetType: "project",
        targetId: projectRow.id,
        metadata: {
          sourceProjectId: source.id,
          sourceRepoFullName: `${sourceRepo.githubOrg}/${sourceRepo.repoName}`,
          newRepoFullName: destRepo.fullName,
          requestedRepoName,
          requestedSlug,
          finalRepoName,
          finalSlug,
          finalName,
          rawArtifactsCopied: rawCopied,
          rawArtifactsSkipped: rawSkipped,
          durationMs,
          workspaceId: auth.workspaceId,
        } as Record<string, unknown>,
      });

      // Spec 112 Phase 8 — broadcast the new project to connected OPCs.
      // Fire-and-log; sync hiccup must not roll back a successful clone.
      void publishProjectCatalogUpsert({
        workspaceId: auth.workspaceId,
        project: {
          id: projectRow.id,
          workspaceId: auth.workspaceId,
          name: finalName,
          slug: finalSlug,
          description: source.description ?? "",
          factoryAdapterId: source.factoryAdapterId,
          detectionLevel: null,
          updatedAt: projectRow.updatedAt,
        },
        repo: {
          githubOrg: destInstallation.githubOrgLogin,
          repoName: finalRepoName,
          defaultBranch: sourceDefaultBranch,
        },
      }).catch((err) => {
        log.warn("clone: catalog upsert broadcast failed", {
          projectId: projectRow.id,
          err: errorMessage(err),
        });
      });

      const opcDeepLink = buildProjectOpenDeepLink({
        projectId: projectRow.id,
        cloneUrl: `https://github.com/${destRepo.fullName}.git`,
        detectionLevel: source.factoryAdapterId ? "acp_produced" : undefined,
      });

      return {
        projectId: projectRow.id,
        name: finalName,
        slug: finalSlug,
        repoFullName: destRepo.fullName,
        defaultBranch: sourceDefaultBranch,
        opcDeepLink,
        rawArtifactsCopied: rawCopied,
        rawArtifactsSkipped: rawSkipped < 0 ? 0 : rawSkipped,
        durationMs,
      };
    } finally {
      // Best-effort cleanup of mirror + worktree tempdirs. The worktree
      // lives in a sibling tempdir; remove its parent so the wrapper dir
      // goes too.
      if (cleanupWorktree) {
        await rm(dirname(cleanupWorktree), { recursive: true, force: true }).catch(
          () => undefined
        );
      }
      await rm(dirname(cleanupMirror), { recursive: true, force: true }).catch(
        () => undefined
      );
    }
  }
);

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

/**
 * FR-029 — repo-name resolution.
 *
 * - `userTyped !== null` (user supplied a non-default name): a collision
 *   surfaces `name_taken`. We do not silently pick a different value.
 * - `userTyped === null` (default path): suffix `-2`, `-3`, … up to 25
 *   attempts on collision. Exhaustion → `name_exhausted`.
 */
async function resolveFinalRepoName(args: {
  token: string;
  githubOrgLogin: string;
  baseName: string;
  userTyped: string | null;
}): Promise<string> {
  if (args.userTyped !== null) {
    const probe = await checkRepoAvailable(
      args.token,
      args.githubOrgLogin,
      args.userTyped
    );
    if (probe.state === "available" || probe.state === "unverifiable") {
      // unverifiable (rate-limited / transient) is treated as "submit anyway";
      // a true collision surfaces from createCloneDestRepo's 422.
      return args.userTyped;
    }
    throw APIError.alreadyExists(
      `name_taken: GitHub repo "${args.githubOrgLogin}/${args.userTyped}" is unavailable`
    );
  }

  for (let i = 0; i < MAX_SUFFIX_ATTEMPTS; i++) {
    const candidate = suffixCandidate(args.baseName, i);
    const probe = await checkRepoAvailable(
      args.token,
      args.githubOrgLogin,
      candidate
    );
    if (probe.state === "available" || probe.state === "unverifiable") {
      return candidate;
    }
  }
  throw APIError.failedPrecondition(
    `name_exhausted: no available repo name after ${MAX_SUFFIX_ATTEMPTS} attempts`
  );
}

/**
 * FR-030 — slug resolution. Mirror of `resolveFinalRepoName` but against
 * the workspace's `(workspace_id, slug)` index.
 */
async function resolveFinalSlug(args: {
  workspaceId: string;
  baseSlug: string;
  userTyped: string | null;
}): Promise<string> {
  if (args.userTyped !== null) {
    const probe = await checkSlugAvailable(args.workspaceId, args.userTyped);
    if (probe.state === "available") return args.userTyped;
    throw APIError.alreadyExists(
      `slug_taken: project slug "${args.userTyped}" already exists in this workspace`
    );
  }
  for (let i = 0; i < MAX_SUFFIX_ATTEMPTS; i++) {
    const candidate = suffixCandidate(args.baseSlug, i);
    const probe = await checkSlugAvailable(args.workspaceId, candidate);
    if (probe.state === "available") return candidate;
  }
  throw APIError.failedPrecondition(
    `slug_exhausted: no available slug after ${MAX_SUFFIX_ATTEMPTS} attempts`
  );
}
