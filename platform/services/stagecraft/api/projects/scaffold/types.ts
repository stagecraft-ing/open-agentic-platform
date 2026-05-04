// Spec 112 §5.2 + §5.3 — scaffold subflow shared types.

export interface ScaffoldAdapterRef {
  /** factory_adapters.id — links the job to the row the scaffold came from. */
  id: string;
  name: string;
  version: string;
  sourceSha: string;
  /**
   * Upstream `<owner>/<repo>` for the template the scaffold clones at
   * Create time. Stamped onto the synthetic adapter manifest by the
   * translator (spec 112 §5.3 op 1) so the scaffold layer doesn't need
   * to round-trip through `factory_upstreams`.
   */
  templateRemote: string;
  /** Branch the cache pins to. Falls back to `main` for sha-pinned upstreams. */
  templateDefaultBranch: string;
}

/**
 * Synthetic-adapter top-level fields the translator stamps onto the
 * manifest at sync time so the scaffold layer can resolve a clone URL
 * without round-tripping through `factory_upstreams`. Spec 112 §5.2 / §5.3.
 */
export interface AdapterTemplateRemote {
  /** GitHub repo identity, e.g. `"GovAlta-Pronghorn/template"`. */
  template_remote: string;
  /** Branch the cache layer pins to; refresher polls this. */
  template_default_branch: string;
}

export interface ScaffoldRequest {
  orgId: string;
  requestedBy: string;
  adapter: ScaffoldAdapterRef;
  /** One of build-spec.schema.yaml's project.variant values. */
  variant: string;
  /** Modules the user picked from MODULE_CATALOG (filtered server-side). */
  selectedModules: string[];
  githubOrg: string;
  repoName: string;
  isPrivate: boolean;
  /** Seed inputs uploaded into the project bucket (spec 112 §4.3). */
  seedInputs?: ScaffoldSeedInput[];
}

export interface ScaffoldSeedInput {
  bucketObjectId: string;
  filename: string;
  mimeType: string;
  /** SHA-256 of the raw bytes, captured at upload time. */
  contentHash: string;
}

export interface ScaffoldJobRecord {
  id: string;
  status: ScaffoldJobStatus;
  step: ScaffoldStep | null;
  errorMessage: string | null;
  githubOrg: string | null;
  repoName: string | null;
  cloneUrl: string | null;
  commitSha: string | null;
}

export type ScaffoldJobStatus =
  | "pending"
  | "running"
  | "succeeded"
  | "failed"
  | "orphaned";

export type ScaffoldStep =
  | "template-cache"
  | "prebuild"
  | "run-entry"
  | "seed-pipeline-state"
  | "extract-artifacts"
  | "repo-create"
  | "push-initial"
  | "cleanup";

export interface ScaffoldResult {
  jobId: string;
  githubOrg: string;
  repoName: string;
  cloneUrl: string;
  htmlUrl: string;
  defaultBranch: string;
  commitSha: string;
  /** L0 pipeline-state.json contents that were committed at commit #1. */
  pipelineStateSeed: Record<string, unknown>;
}
