// Spec 112 §6.3 — Open-in-OPC handoff bundle endpoint.
//
// Returns the resolution OPC needs after activating an `oap://` deep link:
//   - project + primary repo (clone URL)
//   - the precomputed deep link the stagecraft UI also surfaces
//   - the factory_adapters row referenced by projects.factory_adapter_id
//     (null for non-factory projects — the endpoint still works)
//   - org-scoped factory_contracts and factory_processes (latest sync per
//     name, mirroring the spec 108 browser behaviour)
//   - workspace-scoped agent catalog (status='published')
//
// Per-project agent overrides and adapter-declared agent compatibility
// filters are out of scope here (spec 112 §6.3 step 3, "Agents") — the
// bundle returns the workspace's full published catalog and lets OPC
// decide. A future spec will narrow this.
//
// Authority posture: workspace-scoped via getAuthData(). Org-scoped
// catalog tables are read with the project's orgId, not the caller's
// (these match for in-workspace callers).

import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { and, asc, desc, eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  agentCatalog,
  factoryAdapters,
  factoryContracts,
  factoryProcesses,
  projectRepos,
  projects,
} from "../db/schema";
import {
  buildOapBundle,
  type BundleContractInput,
  type BundleProcessInput,
  type OapBundleResponse,
} from "./oapBundleHelpers";

interface OapBundleRequest {
  projectId: string;
}

export const getProjectOapBundle = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/projects/:projectId/oap-bundle",
  },
  async (req: OapBundleRequest): Promise<OapBundleResponse> => {
    const auth = getAuthData()!;

    const [project] = await db
      .select()
      .from(projects)
      .where(
        and(eq(projects.id, req.projectId), eq(projects.workspaceId, auth.workspaceId))
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
        loadPublishedAgents(project.workspaceId),
      ]);

    return buildOapBundle({
      project: {
        id: project.id,
        name: project.name,
        slug: project.slug,
        workspaceId: project.workspaceId,
        orgId: project.orgId,
        factoryAdapterId: project.factoryAdapterId,
      },
      repo: primaryRepo,
      adapter: adapterRow,
      contracts: contractRows,
      processes: processRows,
      agents: agentRows,
    });
  }
);

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

async function loadPublishedAgents(workspaceId: string) {
  const rows = await db
    .select()
    .from(agentCatalog)
    .where(
      and(eq(agentCatalog.workspaceId, workspaceId), eq(agentCatalog.status, "published"))
    )
    .orderBy(asc(agentCatalog.name));

  return rows.map((r) => ({
    id: r.id,
    name: r.name,
    version: r.version,
    contentHash: r.contentHash,
    frontmatter: r.frontmatter,
    bodyMarkdown: r.bodyMarkdown,
  }));
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
