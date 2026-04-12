// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Hiqlite-backed checkpoint metadata store.
//!
//! [`CheckpointStore`] wraps a [`hiqlite::Client`] and a [`BlobStore`] to
//! provide async checkpoint CRUD, diffing, timeline traversal, fork, and GC.

use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::Result;
use hiqlite::{Client, Param};

use super::blobs::BlobStore;
use super::types::{CheckpointDiff, CheckpointInfo, FileEntry, GcResult, TimelineNode};

/// Combined checkpoint metadata + blob storage.
pub struct CheckpointStore {
    pub client: Client,
    pub blobs: BlobStore,
}

impl CheckpointStore {
    /// Create a new store from an existing hiqlite client and blob store.
    pub fn new(client: Client, blobs: BlobStore) -> Self {
        Self { client, blobs }
    }

    // -----------------------------------------------------------------------
    // Write operations
    // -----------------------------------------------------------------------

    /// Persist a new checkpoint and its file manifest.
    ///
    /// Inserts the checkpoint row, all manifest entries, and upserts blob
    /// reference counts.
    pub async fn create_checkpoint(
        &self,
        info: &CheckpointInfo,
        entries: &[FileEntry],
    ) -> Result<()> {
        self.client
            .execute(
                Cow::Borrowed(
                    "INSERT INTO checkpoints \
                     (checkpoint_id, repo_root, parent_id, label, head_sha, fingerprint, \
                      state_hash, merkle_root, file_count, total_bytes, created_at, metadata, \
                      workspace_id, branch_name, run_id) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
                ),
                vec![
                    Param::Text(info.checkpoint_id.clone()),
                    Param::Text(info.repo_root.clone()),
                    info.parent_id
                        .clone()
                        .map(Param::Text)
                        .unwrap_or(Param::Null),
                    info.label.clone().map(Param::Text).unwrap_or(Param::Null),
                    info.head_sha
                        .clone()
                        .map(Param::Text)
                        .unwrap_or(Param::Null),
                    Param::Text(info.fingerprint.clone()),
                    Param::Text(info.state_hash.clone()),
                    Param::Text(info.merkle_root.clone()),
                    Param::Integer(info.file_count),
                    Param::Integer(info.total_bytes),
                    Param::Text(info.created_at.clone()),
                    info.metadata
                        .clone()
                        .map(Param::Text)
                        .unwrap_or(Param::Null),
                    info.workspace_id
                        .clone()
                        .map(Param::Text)
                        .unwrap_or(Param::Null),
                    info.branch_name
                        .clone()
                        .map(Param::Text)
                        .unwrap_or(Param::Null),
                    info.run_id.clone().map(Param::Text).unwrap_or(Param::Null),
                ],
            )
            .await?;

        for entry in entries {
            self.client
                .execute(
                    Cow::Borrowed(
                        "INSERT OR IGNORE INTO manifest_entries \
                         (checkpoint_id, path, blob_hash, size_bytes, permissions) \
                         VALUES ($1, $2, $3, $4, $5)",
                    ),
                    vec![
                        Param::Text(info.checkpoint_id.clone()),
                        Param::Text(entry.path.to_string_lossy().to_string()),
                        Param::Text(entry.content_hash.clone()),
                        Param::Integer(entry.size as i64),
                        Param::Integer(entry.permissions as i64),
                    ],
                )
                .await?;

            self.client
                .execute(
                    Cow::Borrowed(
                        "INSERT INTO blob_refs (blob_hash, ref_count, size_bytes, compression) \
                         VALUES ($1, 1, $2, 'lz4') \
                         ON CONFLICT(blob_hash) DO UPDATE SET ref_count = ref_count + 1",
                    ),
                    vec![
                        Param::Text(entry.content_hash.clone()),
                        Param::Integer(entry.size as i64),
                    ],
                )
                .await?;
        }

        Ok(())
    }

    /// Delete a checkpoint and decrement blob reference counts.
    pub async fn delete_checkpoint(&self, checkpoint_id: &str) -> Result<()> {
        let entries = self.get_entries(checkpoint_id).await?;
        for entry in &entries {
            self.client
                .execute(
                    Cow::Borrowed(
                        "UPDATE blob_refs SET ref_count = ref_count - 1 WHERE blob_hash = $1",
                    ),
                    vec![Param::Text(entry.content_hash.clone())],
                )
                .await?;
        }

        self.client
            .execute(
                Cow::Borrowed("DELETE FROM manifest_entries WHERE checkpoint_id = $1"),
                vec![Param::Text(checkpoint_id.to_string())],
            )
            .await?;

        self.client
            .execute(
                Cow::Borrowed("DELETE FROM checkpoints WHERE checkpoint_id = $1"),
                vec![Param::Text(checkpoint_id.to_string())],
            )
            .await?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Read operations
    // -----------------------------------------------------------------------

    /// Look up a single checkpoint by ID.
    pub async fn get_checkpoint(&self, checkpoint_id: &str) -> Result<Option<CheckpointInfo>> {
        let rows: Vec<CheckpointInfo> = self
            .client
            .query_as(
                "SELECT checkpoint_id, repo_root, parent_id, label, head_sha, fingerprint, \
                 state_hash, merkle_root, file_count, total_bytes, created_at, metadata, \
                 workspace_id, branch_name, run_id \
                 FROM checkpoints WHERE checkpoint_id = $1",
                vec![Param::Text(checkpoint_id.to_string())],
            )
            .await?;
        Ok(rows.into_iter().next())
    }

    /// List all checkpoints for a repository root, newest first.
    ///
    /// When `workspace_id` is `Some`, only checkpoints with a matching
    /// `workspace_id` are returned. `None` returns all checkpoints for the repo.
    pub async fn list_checkpoints(
        &self,
        repo_root: &str,
        workspace_id: Option<&str>,
    ) -> Result<Vec<CheckpointInfo>> {
        match workspace_id {
            Some(wid) => self
                .client
                .query_as(
                    "SELECT checkpoint_id, repo_root, parent_id, label, head_sha, fingerprint, \
                         state_hash, merkle_root, file_count, total_bytes, created_at, metadata, \
                         workspace_id, branch_name, run_id \
                         FROM checkpoints WHERE repo_root = $1 AND workspace_id = $2 \
                         ORDER BY created_at DESC",
                    vec![
                        Param::Text(repo_root.to_string()),
                        Param::Text(wid.to_string()),
                    ],
                )
                .await
                .map_err(Into::into),
            None => self
                .client
                .query_as(
                    "SELECT checkpoint_id, repo_root, parent_id, label, head_sha, fingerprint, \
                         state_hash, merkle_root, file_count, total_bytes, created_at, metadata, \
                         workspace_id, branch_name, run_id \
                         FROM checkpoints WHERE repo_root = $1 ORDER BY created_at DESC",
                    vec![Param::Text(repo_root.to_string())],
                )
                .await
                .map_err(Into::into),
        }
    }

    /// Return all manifest entries for a checkpoint, sorted by path.
    pub async fn get_entries(&self, checkpoint_id: &str) -> Result<Vec<FileEntry>> {
        #[derive(serde::Deserialize)]
        struct EntryRow {
            path: String,
            blob_hash: String,
            size_bytes: i64,
            permissions: Option<i64>,
        }

        let rows: Vec<EntryRow> = self
            .client
            .query_as(
                "SELECT path, blob_hash, size_bytes, permissions \
                 FROM manifest_entries WHERE checkpoint_id = $1 ORDER BY path",
                vec![Param::Text(checkpoint_id.to_string())],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| FileEntry {
                path: r.path.into(),
                content_hash: r.blob_hash,
                size: r.size_bytes as u64,
                permissions: r.permissions.unwrap_or(0o644) as u32,
                // combined_hash is not stored in manifest_entries; left empty
                // because it is only needed at checkpoint-creation time.
                combined_hash: String::new(),
            })
            .collect())
    }

    // -----------------------------------------------------------------------
    // Diff / timeline
    // -----------------------------------------------------------------------

    /// Compute a path-level diff between two checkpoints.
    pub async fn diff_checkpoints(&self, from_id: &str, to_id: &str) -> Result<CheckpointDiff> {
        let from_entries = self.get_entries(from_id).await?;
        let to_entries = self.get_entries(to_id).await?;

        let from_map: HashMap<String, &str> = from_entries
            .iter()
            .map(|e| {
                (
                    e.path.to_string_lossy().to_string(),
                    e.content_hash.as_str(),
                )
            })
            .collect();
        let to_map: HashMap<String, &str> = to_entries
            .iter()
            .map(|e| {
                (
                    e.path.to_string_lossy().to_string(),
                    e.content_hash.as_str(),
                )
            })
            .collect();

        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        for (path, hash) in &to_map {
            match from_map.get(path) {
                None => added.push(path.clone()),
                Some(old_hash) if *old_hash != *hash => modified.push(path.clone()),
                _ => {}
            }
        }
        for path in from_map.keys() {
            if !to_map.contains_key(path) {
                deleted.push(path.clone());
            }
        }

        Ok(CheckpointDiff {
            from_id: from_id.to_string(),
            to_id: to_id.to_string(),
            added,
            modified,
            deleted,
        })
    }

    /// Build the timeline graph for a repository root.
    ///
    /// The "current" checkpoint is the most-recently-created one (first in the
    /// list returned by [`list_checkpoints`], which orders by `created_at DESC`).
    /// When `workspace_id` is `Some`, the graph is restricted to checkpoints
    /// belonging to that workspace.
    pub async fn get_timeline(
        &self,
        repo_root: &str,
        workspace_id: Option<&str>,
    ) -> Result<Vec<TimelineNode>> {
        let checkpoints = self.list_checkpoints(repo_root, workspace_id).await?;

        let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
        for cp in &checkpoints {
            if let Some(pid) = &cp.parent_id {
                children_map
                    .entry(pid.clone())
                    .or_default()
                    .push(cp.checkpoint_id.clone());
            }
        }

        let current_id = checkpoints.first().map(|c| c.checkpoint_id.clone());

        Ok(checkpoints
            .into_iter()
            .map(|cp| {
                let children = children_map
                    .get(&cp.checkpoint_id)
                    .cloned()
                    .unwrap_or_default();
                let is_current = current_id.as_ref() == Some(&cp.checkpoint_id);
                TimelineNode {
                    checkpoint_id: cp.checkpoint_id,
                    parent_id: cp.parent_id,
                    label: cp.label,
                    created_at: cp.created_at,
                    children,
                    is_current,
                    head_sha: cp.head_sha,
                    branch_name: cp.branch_name,
                    run_id: cp.run_id,
                }
            })
            .collect())
    }

    // -----------------------------------------------------------------------
    // Fork
    // -----------------------------------------------------------------------

    /// Create a copy of `source_id` as a new checkpoint (with a fresh ID) and
    /// return the new [`CheckpointInfo`].
    pub async fn fork_checkpoint(
        &self,
        source_id: &str,
        label: Option<String>,
    ) -> Result<CheckpointInfo> {
        let source = self
            .get_checkpoint(source_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Checkpoint not found: {}", source_id))?;
        let entries = self.get_entries(source_id).await?;

        let new_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let fork = CheckpointInfo {
            checkpoint_id: new_id,
            parent_id: Some(source_id.to_string()),
            label,
            repo_root: source.repo_root,
            head_sha: source.head_sha,
            fingerprint: source.fingerprint,
            state_hash: source.state_hash,
            merkle_root: source.merkle_root,
            file_count: source.file_count,
            total_bytes: source.total_bytes,
            created_at: now,
            metadata: source.metadata,
            workspace_id: source.workspace_id,
            branch_name: source.branch_name,
            run_id: source.run_id,
        };

        self.create_checkpoint(&fork, &entries).await?;
        Ok(fork)
    }

    // -----------------------------------------------------------------------
    // Garbage collection
    // -----------------------------------------------------------------------

    /// Remove all blobs whose `ref_count` has dropped to zero or below.
    ///
    /// `repo_root` is accepted for API symmetry but GC is global across all
    /// repos sharing this store (blob refs are not scoped by repo).
    pub async fn gc(&self, _repo_root: &str) -> Result<GcResult> {
        #[derive(serde::Deserialize)]
        struct BlobRow {
            blob_hash: String,
            size_bytes: i64,
        }

        let orphans: Vec<BlobRow> = self
            .client
            .query_as(
                "SELECT blob_hash, size_bytes FROM blob_refs WHERE ref_count <= 0",
                vec![],
            )
            .await?;

        let mut removed = 0usize;
        let mut freed = 0u64;

        for orphan in &orphans {
            self.blobs.delete(&orphan.blob_hash)?;

            self.client
                .execute(
                    Cow::Borrowed("DELETE FROM blob_refs WHERE blob_hash = $1"),
                    vec![Param::Text(orphan.blob_hash.clone())],
                )
                .await?;

            removed += 1;
            freed += orphan.size_bytes as u64;
        }

        Ok(GcResult {
            objects_removed: removed,
            bytes_freed: freed,
        })
    }
}
