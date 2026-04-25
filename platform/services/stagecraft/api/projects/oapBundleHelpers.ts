// Spec 112 §6.3 — pure helpers for the Open-in-OPC bundle endpoint.
//
// Kept free of the Encore native runtime so unit tests can exercise the
// shape-mapping logic without standing up Postgres. The mirror pattern
// used by import.ts / importHelpers.ts.

import { buildProjectOpenDeepLink } from "./scaffold/deepLink";

export interface BundleProjectInput {
  id: string;
  name: string;
  slug: string;
  workspaceId: string;
  orgId: string;
  factoryAdapterId: string | null;
}

export interface BundleRepoInput {
  githubOrg: string;
  repoName: string;
  defaultBranch: string;
}

export interface BundleAdapterInput {
  id: string;
  name: string;
  version: string;
  sourceSha: string;
  syncedAt: Date;
  manifest: unknown;
}

export interface BundleContractInput {
  name: string;
  version: string;
  sourceSha: string;
  syncedAt: Date;
  schema: unknown;
}

export interface BundleProcessInput {
  name: string;
  version: string;
  sourceSha: string;
  syncedAt: Date;
  definition: unknown;
}

export interface BundleAgentInput {
  id: string;
  name: string;
  version: number;
  contentHash: string;
  frontmatter: unknown;
  bodyMarkdown: string;
}

export interface OapBundleProject {
  id: string;
  name: string;
  slug: string;
  workspaceId: string;
  orgId: string;
}

export interface OapBundleRepo {
  cloneUrl: string;
  githubOrg: string;
  repoName: string;
  defaultBranch: string;
}

export interface OapBundleAdapter {
  id: string;
  name: string;
  version: string;
  sourceSha: string;
  syncedAt: string;
  manifest: unknown;
}

export interface OapBundleContract {
  name: string;
  version: string;
  sourceSha: string;
  syncedAt: string;
  schema: unknown;
}

export interface OapBundleProcess {
  name: string;
  version: string;
  sourceSha: string;
  syncedAt: string;
  definition: unknown;
}

export interface OapBundleAgent {
  id: string;
  name: string;
  version: number;
  status: "published";
  contentHash: string;
  frontmatter: unknown;
  bodyMarkdown: string;
}

export interface OapBundleResponse {
  project: OapBundleProject;
  repo: OapBundleRepo | null;
  deepLink: string | null;
  adapter: OapBundleAdapter | null;
  contracts: OapBundleContract[];
  processes: OapBundleProcess[];
  agents: OapBundleAgent[];
}

/**
 * Compose the bundle from raw DB rows. No I/O. Re-exposes
 * buildProjectOpenDeepLink with the project_id + clone URL the bundle
 * uses; OPC and the web UI both rely on the same string.
 */
export function buildOapBundle(input: {
  project: BundleProjectInput;
  repo: BundleRepoInput | null;
  adapter: BundleAdapterInput | null;
  contracts: BundleContractInput[];
  processes: BundleProcessInput[];
  agents: BundleAgentInput[];
}): OapBundleResponse {
  const repo = input.repo ? toBundleRepo(input.repo) : null;
  return {
    project: {
      id: input.project.id,
      name: input.project.name,
      slug: input.project.slug,
      workspaceId: input.project.workspaceId,
      orgId: input.project.orgId,
    },
    repo,
    deepLink: repo
      ? buildProjectOpenDeepLink({
          projectId: input.project.id,
          cloneUrl: repo.cloneUrl,
        })
      : null,
    adapter: input.adapter ? toBundleAdapter(input.adapter) : null,
    contracts: input.contracts.map(toBundleContract),
    processes: input.processes.map(toBundleProcess),
    agents: input.agents.map(toBundleAgent),
  };
}

function toBundleRepo(r: BundleRepoInput): OapBundleRepo {
  return {
    cloneUrl: cloneUrlFor(r.githubOrg, r.repoName),
    githubOrg: r.githubOrg,
    repoName: r.repoName,
    defaultBranch: r.defaultBranch,
  };
}

function toBundleAdapter(a: BundleAdapterInput): OapBundleAdapter {
  return {
    id: a.id,
    name: a.name,
    version: a.version,
    sourceSha: a.sourceSha,
    syncedAt: a.syncedAt.toISOString(),
    manifest: a.manifest,
  };
}

function toBundleContract(c: BundleContractInput): OapBundleContract {
  return {
    name: c.name,
    version: c.version,
    sourceSha: c.sourceSha,
    syncedAt: c.syncedAt.toISOString(),
    schema: c.schema,
  };
}

function toBundleProcess(p: BundleProcessInput): OapBundleProcess {
  return {
    name: p.name,
    version: p.version,
    sourceSha: p.sourceSha,
    syncedAt: p.syncedAt.toISOString(),
    definition: p.definition,
  };
}

function toBundleAgent(a: BundleAgentInput): OapBundleAgent {
  return {
    id: a.id,
    name: a.name,
    version: a.version,
    status: "published",
    contentHash: a.contentHash,
    frontmatter: a.frontmatter,
    bodyMarkdown: a.bodyMarkdown,
  };
}

export function cloneUrlFor(githubOrg: string, repoName: string): string {
  return `https://github.com/${githubOrg}/${repoName}.git`;
}
