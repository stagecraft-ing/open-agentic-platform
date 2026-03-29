/** Mirrors titor::checkpoint::CheckpointMetadata (serialized via serde). */
export interface CheckpointMetadata {
  file_count: number;
  total_size: number;
  compressed_size: number;
  files_changed: number;
  bytes_changed: number;
  tags: string[];
  custom: Record<string, string>;
  titor_version: string;
  host_info: Record<string, unknown>;
}

/** Mirrors titor::checkpoint::Checkpoint. */
export interface Checkpoint {
  id: string;
  parent_id: string | null;
  timestamp: string; // ISO 8601
  description: string | null;
  metadata: CheckpointMetadata;
  state_hash: string;
  content_merkle_root: string;
  signature: string | null;
}

/** Mirrors titor::types::FileEntry. */
export interface FileEntry {
  path: string;
  content_hash: string;
  size: number;
  permissions: number;
  modified: string;
  is_compressed: boolean;
  metadata_hash: string;
  combined_hash: string;
  is_symlink: boolean;
  symlink_target: string | null;
  is_directory: boolean;
}

/** Mirrors titor::types::ChangeStats. */
export interface ChangeStats {
  files_added: number;
  files_modified: number;
  files_deleted: number;
  bytes_added: number;
  bytes_modified: number;
  bytes_deleted: number;
  changed_files: string[];
}

/** Mirrors titor::types::CheckpointDiff. */
export interface CheckpointDiff {
  from_id: string;
  to_id: string;
  added_files: FileEntry[];
  modified_files: [FileEntry, FileEntry][];
  deleted_files: FileEntry[];
  stats: ChangeStats;
}

/** Mirrors titor::verification::VerificationReport. */
export interface VerificationReport {
  checkpoint_id: string;
  metadata_valid: boolean;
  state_hash_valid: boolean;
  merkle_root_valid: boolean;
  file_checks: FileVerification[];
  parent_valid: boolean;
  orphaned_objects: string[];
  verification_time_ms: number;
  total_files_checked: number;
  files_valid: number;
  errors: string[];
}

export interface FileVerification {
  path: string;
  content_valid: boolean;
  metadata_valid: boolean;
  error: string | null;
}
