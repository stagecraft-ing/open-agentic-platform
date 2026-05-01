// Spec 112 §6.3 (amended by spec 119) — Open-in-OPC handoff bundle endpoint.
//
// Returns the resolution OPC needs after activating an `opc://` deep link:
//   - project + primary repo (clone URL)
//   - the precomputed deep link the stagecraft UI also surfaces
//   - the factory_adapters row referenced by projects.factory_adapter_id
//     (null for non-factory projects — the endpoint still works)
//   - org-scoped factory_contracts and factory_processes (latest sync per
//     name, mirroring the spec 108 browser behaviour)
//   - project-scoped agent catalog (status='published')
//   - a short-lived clone token (spec 112 §6.4) OPC threads into both
//     the git clone subprocess and the factory engine launch
//
// Per-project agent overrides and adapter-declared agent compatibility
// filters are out of scope here (spec 112 §6.3 step 3, "Agents") — the
// bundle returns the project's full published catalog and lets OPC
// decide. A future spec will narrow this.
//
// Authority posture: org-scoped via getAuthData(). The project must
// belong to the caller's org.

import { api, APIError } from "encore.dev/api";
import log from "encore.dev/log";
import { getAuthData } from "~encore/auth";
import { and, asc, desc, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  agentCatalog,
  auditLog,
  factoryAdapters,
  factoryContracts,
  factoryProcesses,
  projectAgentBindings,
  projectRepos,
  projects,
} from "../db/schema";
import {
  buildOpcBundle,
  type BundleContractInput,
  type BundleProcessInput,
  type OpcBundleCloneToken,
  type OpcBundleResponse,
} from "./opcBundleHelpers";
import { resolveProjectToken } from "./tokenResolver";

interface OpcBundleRequest {
  projectId: string;
}

export const getProjectOpcBundle = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/projects/:projectId/opc-bundle",
  },
  async (req: OpcBundleRequest): Promise<OpcBundleResponse> => {
    const auth = getAuthData()!;

    const [project] = await db
      .select()
      .from(projects)
      .where(
        and(eq(projects.id, req.projectId), eq(projects.orgId, auth.orgId))
      )
      .limit(1);

    if (!project) {
      throw APIError.notFound("project not found");
    }

    const [primaryRepo, adapterRow, contractRows, processRows, agentRows] =
      await Promise.all([
        loadPrimaryRepo(project.id),
        project.factoryAdapterId
          ? loadAdapter(project.orgId, project.factoryAdapterId)
          : Promise.resolve(null),
        loadLatestContracts(project.orgId),
        loadLatestProcesses(project.orgId),
        loadPublishedAgents(project.id),
      ]);

    const cloneToken = await resolveCloneTokenForBundle({
      orgId: project.orgId,
      projectId: project.id,
      actorUserId: auth.userID,
      primaryRepo,
    });

    return buildOpcBundle({
      project: {
        id: project.id,
        name: project.name,
        slug: project.slug,
        orgId: project.orgId,
        factoryAdapterId: project.factoryAdapterId,
      },
      repo: primaryRepo,
      adapter: adapterRow,
      contracts: contractRows,
      processes: processRows,
      agents: agentRows,
      cloneToken,
    });
  }
);

// ---------------------------------------------------------------------------
// Spec 112 §6.4 — clone-token refresh endpoint
// ---------------------------------------------------------------------------
//
// Lightweight sibling of the full bundle. OPC calls this to refresh
// an installation token within ~5 minutes of expiry, or after a 401
// from any GitHub call made through the cached token. Returns the
// same `OpcBundleCloneToken | null` shape the bundle exposes.

interface CloneTokenResponse {
  cloneToken: OpcBundleCloneToken | null;
}

export const refreshProjectCloneToken = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/projects/:projectId/clone-token",
  },
  async (req: OpcBundleRequest): Promise<CloneTokenResponse> => {
    const auth = getAuthData()!;

    const [project] = await db
      .select({
        id: projects.id,
        orgId: projects.orgId,
      })
      .from(projects)
      .where(
        and(eq(projects.id, req.projectId), eq(projects.orgId, auth.orgId))
      )
      .limit(1);

    if (!project) {
      throw APIError.notFound("project not found");
    }

    const primaryRepo = await loadPrimaryRepo(project.id);
    const cloneToken = await resolveCloneTokenForBundle({
      orgId: project.orgId,
      projectId: project.id,
      actorUserId: auth.userID,
      primaryRepo,
    });
    return { cloneToken };
  }
);

/**
 * Spec 112 §6.4.2 — resolve the clone token to ship in the bundle (or
 * the refresh endpoint). The long-lived PAT lives only in stagecraft;
 * what crosses the wire is either an installation token (preferred,
 * ~1h TTL) or — when the target org has no App installation — a copy
 * of the project PAT (acknowledged in §10 risks). For repos with no
 * primary repo row we return null (the bundle still works for
 * non-factory projects), which OPC treats as "clone anonymously".
 *
 * Hard resolution failures (App broker timeout, PAT decrypt error)
 * surface as 503 — silent degradation to null would mis-classify as
 * "public repo" deep in OPC.
 */
async function resolveCloneTokenForBundle(args: {
  orgId: string;
  projectId: string;
  actorUserId: string;
  primaryRepo: { githubOrg: string; repoName: string; defaultBranch: string } | null;
}): Promise<OpcBundleCloneToken | null> {
  if (!args.primaryRepo) return null;

  let resolved;
  try {
    resolved = await resolveProjectToken({
      orgId: args.orgId,
      projectId: args.projectId,
      targetGithubOrgLogin: args.primaryRepo.githubOrg,
      permissions: { contents: "read", metadata: "read" },
    });
  } catch (err) {
    log.error("opc bundle: clone-token resolution failed hard", {
      orgId: args.orgId,
      projectId: args.projectId,
      githubOrg: args.primaryRepo.githubOrg,
      err: String(err),
    });
    throw APIError.unavailable(
      "Clone token could not be resolved. Check the GitHub App installation or the project PAT under /app/project/:id/settings/github-pat."
    );
  }

  if (!resolved) {
    // Anonymous-clone path — bundle returns null. OPC treats null as
    // "public repo, no auth". Distinct from the 503 above.
    return null;
  }

  // Spec 109 §8 — the resolver itself emits no audit event today; we
  // record one here so the audit trail captures *who* obtained a clone
  // token *for which project* and via which credential.
  await db.insert(auditLog).values({
    actorUserId: args.actorUserId,
    action: "project.token.resolved",
    targetType: "projects",
    targetId: args.projectId,
    metadata: {
      source: resolved.source,
      target_github_org: args.primaryRepo.githubOrg,
      expires_at: resolved.expiresAt?.toISOString() ?? null,
    },
  });

  return {
    value: resolved.token,
    source: resolved.source,
    expiresAt: resolved.expiresAt?.toISOString() ?? null,
  };
}

async function loadPrimaryRepo(projectId: string) {
  const rows = await db
    .select({
      githubOrg: projectRepos.githubOrg,
      repoName: projectRepos.repoName,
      defaultBranch: projectRepos.defaultBranch,
      isPrimary: projectRepos.isPrimary,
      createdAt: projectRepos.createdAt,
    })
    .from(projectRepos)
    .where(eq(projectRepos.projectId, projectId))
    .orderBy(desc(projectRepos.isPrimary), asc(projectRepos.createdAt));

  const pick = rows[0];
  if (!pick) return null;
  return {
    githubOrg: pick.githubOrg,
    repoName: pick.repoName,
    defaultBranch: pick.defaultBranch,
  };
}

async function loadAdapter(orgId: string, adapterId: string) {
  const [row] = await db
    .select()
    .from(factoryAdapters)
    .where(and(eq(factoryAdapters.orgId, orgId), eq(factoryAdapters.id, adapterId)))
    .limit(1);
  if (!row) return null;
  return {
    id: row.id,
    name: row.name,
    version: row.version,
    sourceSha: row.sourceSha,
    syncedAt: row.syncedAt,
    manifest: row.manifest,
  };
}

async function loadLatestContracts(orgId: string): Promise<BundleContractInput[]> {
  const rows = await db
    .select()
    .from(factoryContracts)
    .where(eq(factoryContracts.orgId, orgId))
    .orderBy(asc(factoryContracts.name), desc(factoryContracts.syncedAt));

  return dedupeByName(rows).map((r) => ({
    name: r.name,
    version: r.version,
    sourceSha: r.sourceSha,
    syncedAt: r.syncedAt,
    schema: r.schema,
  }));
}

async function loadLatestProcesses(orgId: string): Promise<BundleProcessInput[]> {
  const rows = await db
    .select()
    .from(factoryProcesses)
    .where(eq(factoryProcesses.orgId, orgId))
    .orderBy(asc(factoryProcesses.name), desc(factoryProcesses.syncedAt));

  return dedupeByName(rows).map((r) => ({
    name: r.name,
    version: r.version,
    sourceSha: r.sourceSha,
    syncedAt: r.syncedAt,
    definition: r.definition,
  }));
}

// Spec 123: project-scoped agents are now consumed via project_agent_bindings.
// Return only bindings whose catalog row is currently published — retired-upstream
// bindings (I-B3) stay visible in the project Agents UI but are excluded from
// the OPC active-agent bundle so OPC doesn't attempt to invoke a retired prompt.
async function loadPublishedAgents(projectId: string) {
  const rows = await db
    .select({
      id: agentCatalog.id,
      name: agentCatalog.name,
      version: agentCatalog.version,
      contentHash: agentCatalog.contentHash,
      frontmatter: agentCatalog.frontmatter,
      bodyMarkdown: agentCatalog.bodyMarkdown,
    })
    .from(projectAgentBindings)
    .innerJoin(
      agentCatalog,
      eq(agentCatalog.id, projectAgentBindings.orgAgentId),
    )
    .where(
      and(
        eq(projectAgentBindings.projectId, projectId),
        eq(agentCatalog.status, "published"),
      ),
    )
    .orderBy(asc(agentCatalog.name));

  return rows;
}

/**
 * Spec 108 keeps `factory_contracts` / `factory_processes` keyed on
 * (orgId, name, version). The catalog browser picks the latest synced
 * row per name (browse.ts §getContract / §getProcess). We mirror that
 * here so the bundle is consistent with what the UI shows.
 */
function dedupeByName<T extends { name: string }>(rows: T[]): T[] {
  const seen = new Set<string>();
  const out: T[] = [];
  for (const row of rows) {
    if (seen.has(row.name)) continue;
    seen.add(row.name);
    out.push(row);
  }
  return out;
}
