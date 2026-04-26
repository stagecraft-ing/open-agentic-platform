// Spec 112 §6.3 — OAP project bundle types.
//
// Resolved on the OPC side from state stagecraft already maintains
// (factory_adapters, factory_contracts, factory_processes, agent
// catalog). The bundle does not travel as a payload on the deep link;
// it is fetched via `fetch_project_opc_bundle` after handoff.
//
// Types are extracted here so the Tab context, panel, and inbox hook
// can share them without circular imports.

export interface OpcBundleProject {
  id: string;
  name: string;
  slug: string;
  workspaceId: string;
  orgId: string;
}

export interface OpcBundleRepo {
  cloneUrl: string;
  githubOrg: string;
  repoName: string;
  defaultBranch: string;
}

export interface OpcBundleAdapter {
  id: string;
  name: string;
  version: string;
  sourceSha: string;
  syncedAt: string;
  manifest: unknown;
}

export interface OpcBundleContract {
  name: string;
  version: string;
  sourceSha: string;
  syncedAt: string;
  schema: unknown;
}

export interface OpcBundleProcess {
  name: string;
  version: string;
  sourceSha: string;
  syncedAt: string;
  definition: unknown;
}

export interface OpcBundleAgent {
  id: string;
  name: string;
  version: number;
  status: string;
  contentHash: string;
  frontmatter: unknown;
  bodyMarkdown: string;
}

export interface OpcBundle {
  project: OpcBundleProject;
  repo: OpcBundleRepo | null;
  deepLink: string | null;
  adapter: OpcBundleAdapter | null;
  contracts: OpcBundleContract[];
  processes: OpcBundleProcess[];
  agents: OpcBundleAgent[];
}
