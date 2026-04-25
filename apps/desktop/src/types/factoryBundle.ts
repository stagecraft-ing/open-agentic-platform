// Spec 112 §6.3 — OAP project bundle types.
//
// Resolved on the OPC side from state stagecraft already maintains
// (factory_adapters, factory_contracts, factory_processes, agent
// catalog). The bundle does not travel as a payload on the deep link;
// it is fetched via `fetch_project_oap_bundle` after handoff.
//
// Types are extracted here so the Tab context, panel, and inbox hook
// can share them without circular imports.

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
  status: string;
  contentHash: string;
  frontmatter: unknown;
  bodyMarkdown: string;
}

export interface OapBundle {
  project: OapBundleProject;
  repo: OapBundleRepo | null;
  deepLink: string | null;
  adapter: OapBundleAdapter | null;
  contracts: OapBundleContract[];
  processes: OapBundleProcess[];
  agents: OapBundleAgent[];
}
