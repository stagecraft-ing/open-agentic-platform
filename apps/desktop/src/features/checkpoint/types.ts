/** Checkpoint metadata from axiomregent's checkpoint.* MCP tools. */
export interface CheckpointInfo {
  checkpoint_id: string;
  repo_root: string;
  parent_id: string | null;
  label: string | null;
  head_sha: string | null;
  fingerprint: string;
  state_hash: string;
  merkle_root: string;
  file_count: number;
  total_bytes: number;
  created_at: string; // ISO 8601
  metadata: string | null;
}

/** Alias used by the checkpoint flow. */
export type Checkpoint = CheckpointInfo;

/** Diff result from checkpoint.diff. */
export interface CheckpointDiff {
  from_checkpoint_id: string;
  to_checkpoint_id: string;
  added: string[];
  modified: string[];
  deleted: string[];
  file_diffs?: FileDiff[];
}

export interface FileDiff {
  path: string;
  hunks: string[];
}

/** Verification result from checkpoint.verify. */
export interface VerificationReport {
  checkpoint_id: string;
  ok: boolean;
  merkle_root_valid: boolean;
  corrupted_files: string[];
  missing_blobs: string[];
}
