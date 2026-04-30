// Spec 112 §5.2 + §5.3 — scaffold subflow shared types.

export interface ScaffoldAdapterRef {
  /** factory_adapters.id — links the job to the row the scaffold came from. */
  id: string;
  name: string;
  version: string;
  sourceSha: string;
  /** Parsed manifest.scaffold block (spec 112 §8). */
  scaffold: AdapterScaffoldBlock;
}

export interface AdapterScaffoldBlock {
  entry_point?: string;
  runtime?: string;
  args_schema?: Record<string, unknown>;
  profiles?: Array<{
    name: string;
    variant: string;
    modules?: string[];
    default?: boolean;
  }>;
  emits?: Array<{ path: string; description?: string }>;
  // Legacy optional fields (pre-spec 112) tolerated on read.
  source?: string;
  description?: string;
  modules?: Record<string, string[]>;
  setup_commands?: string[];
}

export interface ScaffoldRequest {
  orgId: string;
  requestedBy: string;
  adapter: ScaffoldAdapterRef;
  /** One of the adapter's declared variants; validated against args_schema. */
  variant: string;
  /** Optional named profile from adapter.scaffold.profiles. */
  profileName?: string;
  githubOrg: string;
  repoName: string;
  isPrivate: boolean;
  /** Additional free-form --args to pass through to the entry point. */
  args?: Record<string, unknown>;
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
