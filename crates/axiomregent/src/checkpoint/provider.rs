// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! MCP tool provider for the checkpoint subsystem.
//!
//! [`CheckpointProvider`] wraps a [`CheckpointStore`] and exposes 10 MCP tools
//! under the `checkpoint.*` namespace.

use async_trait::async_trait;
use serde_json::{Map, Value, json};
use std::path::Path;
use std::sync::Arc;

use super::diff::create_file_diff;
use super::merkle;
use super::store::CheckpointStore;
use super::types::{CheckpointInfo, FileEntry};
use super::verify;
use crate::router::provider::{ToolPermissions, ToolProvider};

/// Map legacy snapshot.* tool names to checkpoint.* equivalents.
fn normalize_tool_name(name: &str) -> &str {
    match name {
        "snapshot.create" => "checkpoint.create",
        "snapshot.list" => "checkpoint.list",
        "snapshot.read" => "checkpoint.info", // closest equivalent
        "snapshot.diff" => "checkpoint.diff",
        "snapshot.info" => "checkpoint.info",
        "snapshot.export" => "checkpoint.info", // no direct equivalent yet
        "snapshot.changes" => "checkpoint.diff",
        _ => name,
    }
}

/// MCP tool provider backed by a [`CheckpointStore`].
pub struct CheckpointProvider {
    store: Arc<CheckpointStore>,
}

impl CheckpointProvider {
    pub fn new(store: Arc<CheckpointStore>) -> Self {
        Self { store }
    }
}

// ---------------------------------------------------------------------------
// ToolProvider implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl ToolProvider for CheckpointProvider {
    fn tool_schemas(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "checkpoint.create",
                "description": "Create a checkpoint of the current directory state. Walks the directory, hashes all files, stores compressed blobs, and records the checkpoint with a Merkle root for integrity verification.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo_root": { "type": "string", "description": "Absolute path to the repository root" },
                        "label": { "type": "string", "description": "Optional label for this checkpoint" }
                    },
                    "required": ["repo_root"]
                }
            }),
            json!({
                "name": "checkpoint.restore",
                "description": "Restore directory state from a checkpoint. Writes all files from the checkpoint back to disk.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo_root": { "type": "string" },
                        "checkpoint_id": { "type": "string", "description": "ID of the checkpoint to restore" },
                        "dry_run": { "type": "boolean", "description": "If true, list files that would be restored without writing" }
                    },
                    "required": ["repo_root", "checkpoint_id"]
                }
            }),
            json!({
                "name": "checkpoint.list",
                "description": "List all checkpoints for a repository, newest first",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo_root": { "type": "string" }
                    },
                    "required": ["repo_root"]
                }
            }),
            json!({
                "name": "checkpoint.timeline",
                "description": "Show the checkpoint DAG (directed acyclic graph) with parent/child relationships",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo_root": { "type": "string" }
                    },
                    "required": ["repo_root"]
                }
            }),
            json!({
                "name": "checkpoint.fork",
                "description": "Create a new branch from an existing checkpoint",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "checkpoint_id": { "type": "string" },
                        "label": { "type": "string", "description": "Label for the forked checkpoint" }
                    },
                    "required": ["checkpoint_id"]
                }
            }),
            json!({
                "name": "checkpoint.diff",
                "description": "Compare two checkpoints showing added, modified, and deleted files with optional line-level diffs",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "from_checkpoint_id": { "type": "string" },
                        "to_checkpoint_id": { "type": "string" },
                        "detailed": { "type": "boolean", "description": "If true, include line-level diffs for modified files" }
                    },
                    "required": ["from_checkpoint_id", "to_checkpoint_id"]
                }
            }),
            json!({
                "name": "checkpoint.verify",
                "description": "Verify checkpoint integrity by checking blob hashes and Merkle root",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "checkpoint_id": { "type": "string" }
                    },
                    "required": ["checkpoint_id"]
                }
            }),
            json!({
                "name": "checkpoint.gc",
                "description": "Garbage collect unreferenced blobs to reclaim disk space",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo_root": { "type": "string" }
                    },
                    "required": ["repo_root"]
                }
            }),
            json!({
                "name": "checkpoint.status",
                "description": "Show current checkpoint status for a repository",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo_root": { "type": "string" }
                    },
                    "required": ["repo_root"]
                }
            }),
            json!({
                "name": "checkpoint.info",
                "description": "Get detailed information about a specific checkpoint",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "checkpoint_id": { "type": "string" }
                    },
                    "required": ["checkpoint_id"]
                }
            }),
            // Backward-compatible snapshot.* aliases
            json!({
                "name": "snapshot.create",
                "description": "[Alias for checkpoint.create] Create a checkpoint of the current directory state",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo_root": { "type": "string", "description": "Absolute path to the repository root" },
                        "label": { "type": "string", "description": "Optional label for this checkpoint" }
                    },
                    "required": ["repo_root"]
                }
            }),
            json!({
                "name": "snapshot.list",
                "description": "[Alias for checkpoint.list] List all checkpoints for a repository, newest first",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo_root": { "type": "string" }
                    },
                    "required": ["repo_root"]
                }
            }),
            json!({
                "name": "snapshot.info",
                "description": "[Alias for checkpoint.info] Get detailed information about a specific checkpoint",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "checkpoint_id": { "type": "string" }
                    },
                    "required": ["checkpoint_id"]
                }
            }),
            json!({
                "name": "snapshot.diff",
                "description": "[Alias for checkpoint.diff] Compare two checkpoints showing added, modified, and deleted files",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "from_checkpoint_id": { "type": "string" },
                        "to_checkpoint_id": { "type": "string" },
                        "detailed": { "type": "boolean", "description": "If true, include line-level diffs for modified files" }
                    },
                    "required": ["from_checkpoint_id", "to_checkpoint_id"]
                }
            }),
        ]
    }

    async fn handle(&self, name: &str, args: &Map<String, Value>) -> Option<anyhow::Result<Value>> {
        let name = normalize_tool_name(name);
        match name {
            "checkpoint.create" => {
                let repo_root = match args.get("repo_root").and_then(|v| v.as_str()) {
                    Some(v) => Path::new(v),
                    None => return Some(Err(anyhow::anyhow!("repo_root required"))),
                };
                let label = args.get("label").and_then(|v| v.as_str()).map(String::from);
                Some(self.do_create(repo_root, label).await)
            }

            "checkpoint.restore" => {
                let repo_root = match args.get("repo_root").and_then(|v| v.as_str()) {
                    Some(v) => Path::new(v),
                    None => return Some(Err(anyhow::anyhow!("repo_root required"))),
                };
                let checkpoint_id = match args.get("checkpoint_id").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("checkpoint_id required"))),
                };
                let dry_run = args
                    .get("dry_run")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Some(self.do_restore(repo_root, checkpoint_id, dry_run).await)
            }

            "checkpoint.list" => {
                let repo_root = match args.get("repo_root").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("repo_root required"))),
                };
                Some(self.do_list(repo_root).await)
            }

            "checkpoint.timeline" => {
                let repo_root = match args.get("repo_root").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("repo_root required"))),
                };
                Some(self.do_timeline(repo_root).await)
            }

            "checkpoint.fork" => {
                let checkpoint_id = match args.get("checkpoint_id").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("checkpoint_id required"))),
                };
                let label = args.get("label").and_then(|v| v.as_str()).map(String::from);
                Some(self.do_fork(checkpoint_id, label).await)
            }

            "checkpoint.diff" => {
                let from_id = match args.get("from_checkpoint_id").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("from_checkpoint_id required"))),
                };
                let to_id = match args.get("to_checkpoint_id").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("to_checkpoint_id required"))),
                };
                let detailed = args
                    .get("detailed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Some(self.do_diff(from_id, to_id, detailed).await)
            }

            "checkpoint.verify" => {
                let checkpoint_id = match args.get("checkpoint_id").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("checkpoint_id required"))),
                };
                Some(self.do_verify(checkpoint_id).await)
            }

            "checkpoint.gc" => {
                let repo_root = match args.get("repo_root").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("repo_root required"))),
                };
                Some(self.do_gc(repo_root).await)
            }

            "checkpoint.status" => {
                let repo_root = match args.get("repo_root").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("repo_root required"))),
                };
                Some(self.do_status(repo_root).await)
            }

            "checkpoint.info" => {
                let checkpoint_id = match args.get("checkpoint_id").and_then(|v| v.as_str()) {
                    Some(v) => v,
                    None => return Some(Err(anyhow::anyhow!("checkpoint_id required"))),
                };
                Some(self.do_info(checkpoint_id).await)
            }

            _ => None,
        }
    }

    fn tier(&self, name: &str) -> Option<agent::safety::ToolTier> {
        let name = normalize_tool_name(name);
        match name {
            "checkpoint.list"
            | "checkpoint.timeline"
            | "checkpoint.status"
            | "checkpoint.info"
            | "checkpoint.diff"
            | "checkpoint.verify" => Some(agent::safety::ToolTier::Tier1),
            "checkpoint.create" | "checkpoint.fork" | "checkpoint.gc" => {
                Some(agent::safety::ToolTier::Tier2)
            }
            "checkpoint.restore" => Some(agent::safety::ToolTier::Tier3),
            _ => None,
        }
    }

    fn permissions(&self, name: &str) -> Option<ToolPermissions> {
        let name = normalize_tool_name(name);
        match name {
            "checkpoint.list"
            | "checkpoint.timeline"
            | "checkpoint.status"
            | "checkpoint.info"
            | "checkpoint.diff"
            | "checkpoint.verify" => Some(ToolPermissions {
                requires_file_read: true,
                ..Default::default()
            }),
            "checkpoint.create" | "checkpoint.fork" | "checkpoint.gc" => Some(ToolPermissions {
                requires_file_read: true,
                requires_file_write: true,
                ..Default::default()
            }),
            "checkpoint.restore" => Some(ToolPermissions {
                requires_file_read: true,
                requires_file_write: true,
                ..Default::default()
            }),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Private implementation methods
// ---------------------------------------------------------------------------

impl CheckpointProvider {
    async fn do_create(&self, repo_root: &Path, label: Option<String>) -> anyhow::Result<Value> {
        let root = repo_root.canonicalize()?;
        let mut entries: Vec<FileEntry> = Vec::new();

        for entry in walkdir::WalkDir::new(&root).into_iter().filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                ".git" | ".axiomregent" | "node_modules" | "target"
            )
        }) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let rel = path.strip_prefix(&root)?;
            let content = std::fs::read(path)?;
            let content_hash = self.store.blobs.put(&content)?;
            let meta = std::fs::metadata(path)?;

            #[cfg(unix)]
            let perms = {
                use std::os::unix::fs::PermissionsExt;
                meta.permissions().mode()
            };
            #[cfg(not(unix))]
            let perms = 0o644u32;

            let combined = merkle::combined_hash(&content_hash, perms);
            entries.push(FileEntry {
                path: rel.to_path_buf(),
                content_hash,
                size: meta.len(),
                permissions: perms,
                combined_hash: combined,
            });
        }

        let tree = merkle::MerkleTree::from_entries(&entries);
        let merkle_root = tree.root_hash().unwrap_or("empty").to_string();

        let root_str = root.to_string_lossy().to_string();
        let existing = self.store.list_checkpoints(&root_str).await?;
        let parent_id = existing.first().map(|c| c.checkpoint_id.clone());

        let state_hash =
            merkle::hash_content(format!("{}:{}", merkle_root, entries.len()).as_bytes());

        let now = chrono::Utc::now().to_rfc3339();
        let cp_id = uuid::Uuid::new_v4().to_string();
        let total_bytes: u64 = entries.iter().map(|e| e.size).sum();

        let info = CheckpointInfo {
            checkpoint_id: cp_id.clone(),
            parent_id,
            label,
            repo_root: root_str,
            head_sha: None,
            fingerprint: "{}".to_string(),
            state_hash,
            merkle_root,
            file_count: entries.len() as i64,
            total_bytes: total_bytes as i64,
            created_at: now,
            metadata: None,
        };

        self.store.create_checkpoint(&info, &entries).await?;

        // Emit cross-session event (FR-006)
        crate::events::emit(
            &self.store.client,
            crate::events::EVENT_CHECKPOINT_CREATED,
            serde_json::json!({
                "checkpoint_id": &cp_id,
                "repo_root": info.repo_root,
                "file_count": entries.len(),
            }),
        )
        .await;

        Ok(json!({
            "checkpoint_id": cp_id,
            "file_count": entries.len(),
            "total_bytes": total_bytes,
            "status": "created"
        }))
    }

    async fn do_restore(
        &self,
        repo_root: &Path,
        checkpoint_id: &str,
        dry_run: bool,
    ) -> anyhow::Result<Value> {
        let entries = self.store.get_entries(checkpoint_id).await?;
        let root = repo_root.canonicalize()?;
        let mut restored: Vec<String> = Vec::new();

        for entry in &entries {
            let target = root.join(&entry.path);
            if dry_run {
                restored.push(entry.path.to_string_lossy().to_string());
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if let Some(content) = self.store.blobs.get(&entry.content_hash)? {
                std::fs::write(&target, &content)?;
                restored.push(entry.path.to_string_lossy().to_string());
            }
        }

        Ok(json!({
            "checkpoint_id": checkpoint_id,
            "files_restored": restored.len(),
            "dry_run": dry_run,
            "files": restored
        }))
    }

    async fn do_list(&self, repo_root: &str) -> anyhow::Result<Value> {
        let checkpoints = self.store.list_checkpoints(repo_root).await?;
        Ok(json!({ "checkpoints": checkpoints }))
    }

    async fn do_timeline(&self, repo_root: &str) -> anyhow::Result<Value> {
        let timeline = self.store.get_timeline(repo_root).await?;
        Ok(json!({ "timeline": timeline }))
    }

    async fn do_fork(&self, checkpoint_id: &str, label: Option<String>) -> anyhow::Result<Value> {
        let forked = self.store.fork_checkpoint(checkpoint_id, label).await?;
        Ok(json!({
            "checkpoint_id": forked.checkpoint_id,
            "parent_id": forked.parent_id,
            "label": forked.label,
            "status": "forked"
        }))
    }

    async fn do_diff(&self, from_id: &str, to_id: &str, detailed: bool) -> anyhow::Result<Value> {
        let diff = self.store.diff_checkpoints(from_id, to_id).await?;

        if !detailed {
            return Ok(json!({
                "from_checkpoint_id": diff.from_id,
                "to_checkpoint_id": diff.to_id,
                "added": diff.added,
                "modified": diff.modified,
                "deleted": diff.deleted
            }));
        }

        // Build line-level diffs for modified files.
        let from_entries = self.store.get_entries(from_id).await?;
        let to_entries = self.store.get_entries(to_id).await?;

        let from_map: std::collections::HashMap<String, String> = from_entries
            .iter()
            .map(|e| (e.path.to_string_lossy().to_string(), e.content_hash.clone()))
            .collect();
        let to_map: std::collections::HashMap<String, String> = to_entries
            .iter()
            .map(|e| (e.path.to_string_lossy().to_string(), e.content_hash.clone()))
            .collect();

        let mut file_diffs = Vec::new();
        for path in &diff.modified {
            let old_bytes = from_map
                .get(path)
                .and_then(|h| self.store.blobs.get(h).ok().flatten())
                .unwrap_or_default();
            let new_bytes = to_map
                .get(path)
                .and_then(|h| self.store.blobs.get(h).ok().flatten())
                .unwrap_or_default();
            let fd = create_file_diff(path, &old_bytes, &new_bytes);
            file_diffs.push(fd);
        }

        Ok(json!({
            "from_checkpoint_id": diff.from_id,
            "to_checkpoint_id": diff.to_id,
            "added": diff.added,
            "modified": diff.modified,
            "deleted": diff.deleted,
            "file_diffs": file_diffs
        }))
    }

    async fn do_verify(&self, checkpoint_id: &str) -> anyhow::Result<Value> {
        let info = self
            .store
            .get_checkpoint(checkpoint_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Checkpoint not found: {}", checkpoint_id))?;

        let entries = self.store.get_entries(checkpoint_id).await?;

        let mut corrupted: Vec<String> = Vec::new();
        let mut missing_blobs: Vec<String> = Vec::new();

        for entry in &entries {
            match self.store.blobs.get(&entry.content_hash)? {
                None => missing_blobs.push(entry.path.to_string_lossy().to_string()),
                Some(content) => {
                    if !verify::verify_file_hash(entry, &content) {
                        corrupted.push(entry.path.to_string_lossy().to_string());
                    }
                }
            }
        }

        // Re-compute combined hashes so Merkle verification is accurate.
        let verified_entries: Vec<FileEntry> = entries
            .iter()
            .map(|e| FileEntry {
                combined_hash: merkle::combined_hash(&e.content_hash, e.permissions),
                ..e.clone()
            })
            .collect();

        let merkle_ok = verify::verify_merkle_root(&verified_entries, &info.merkle_root);

        let ok = corrupted.is_empty() && missing_blobs.is_empty() && merkle_ok;

        Ok(json!({
            "checkpoint_id": checkpoint_id,
            "ok": ok,
            "merkle_root_valid": merkle_ok,
            "corrupted_files": corrupted,
            "missing_blobs": missing_blobs
        }))
    }

    async fn do_gc(&self, repo_root: &str) -> anyhow::Result<Value> {
        let result = self.store.gc(repo_root).await?;
        Ok(json!({
            "objects_removed": result.objects_removed,
            "bytes_freed": result.bytes_freed,
            "status": "gc_complete"
        }))
    }

    async fn do_status(&self, repo_root: &str) -> anyhow::Result<Value> {
        let checkpoints = self.store.list_checkpoints(repo_root).await?;
        let latest = checkpoints.first();
        Ok(json!({
            "repo_root": repo_root,
            "checkpoint_count": checkpoints.len(),
            "latest_checkpoint_id": latest.map(|c| &c.checkpoint_id),
            "latest_created_at": latest.map(|c| &c.created_at),
            "latest_label": latest.and_then(|c| c.label.as_deref())
        }))
    }

    async fn do_info(&self, checkpoint_id: &str) -> anyhow::Result<Value> {
        let info = self
            .store
            .get_checkpoint(checkpoint_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Checkpoint not found: {}", checkpoint_id))?;

        Ok(serde_json::to_value(&info)?)
    }
}
