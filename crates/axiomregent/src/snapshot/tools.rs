// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: MCP_SNAPSHOT_WORKSPACE
// Spec: spec/core/snapshot-workspace.md

use crate::snapshot::lease::{Fingerprint, LeaseStore};
use crate::snapshot::store::{Entry, Manifest, Store};
use anyhow::{Result, anyhow};
use base64::Engine;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir;

pub struct SnapshotTools {
    lease_store: Arc<LeaseStore>,
    store: Arc<Store>,
}

impl SnapshotTools {
    pub fn new(lease_store: Arc<LeaseStore>, store: Arc<Store>) -> Self {
        Self { lease_store, store }
    }

    // --- Helpers ---

    async fn check_lease(&self, lease_id: Option<&str>, repo_root: &Path) -> Result<String> {
        // If lease_id provided, check it.
        // If not, spec says "All hybrid read tools ... issue lease when missing".
        // BUT "Validation: Every worktree-mode request with a lease_id validates it".
        // So if missing, we issue new.
        if let Some(lid) = lease_id {
            self.lease_store.check_lease(lid, repo_root).await?;
            Ok(lid.to_string())
        } else {
            // Issue new lease
            let fp = Fingerprint::compute(repo_root).await?;
            Ok(self.lease_store.issue(fp).await?)
        }
    }

    fn resolve_path(&self, repo_root: &Path, rel_path: &str) -> Result<PathBuf> {
        let path = repo_root.join(rel_path);
        let canonical_root = repo_root.canonicalize()?;

        // Use path.canonicalize() if it exists?
        // For read tools (list, file, grep), we usually expect existence or just walking.
        // Spec says "Paths MUST be repo-relative... MUST NOT contain .. or absolute roots".
        // Safety check first?

        // Simple security check on string:
        if rel_path.contains("..") || rel_path.starts_with('/') {
            return Err(anyhow!(
                "Invalid path (traversal or absolute): {}",
                rel_path
            ));
        }

        // Canonical check if exists
        if path.exists() {
            let c = path.canonicalize()?;
            if !c.starts_with(&canonical_root) {
                return Err(anyhow!("Path escapes repo root"));
            }
            Ok(c)
        } else {
            // If doesn't exist, basic join check?
            // Join already done. Just return it?
            Ok(path)
        }
    }

    // --- Tools ---

    #[allow(clippy::too_many_arguments)]
    pub async fn snapshot_list(
        &self,
        repo_root: &Path,
        path: &str,
        mode: &str,
        lease_id: Option<String>,
        snapshot_id: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<serde_json::Value> {
        let repo_root = repo_root.canonicalize()?;
        let target_path = self.resolve_path(&repo_root, path)?;

        let limit = limit.unwrap_or(1000); // Default limit? Or unlimited? Let's say 1000 safety cap.
        let offset = offset.unwrap_or(0);

        if mode == "worktree" {
            let lid = self.check_lease(lease_id.as_deref(), &repo_root).await?;

            // Walk dir efficiently
            let mut entries = Vec::new();
            let total;

            if target_path.exists() {
                // If file, just that.
                if target_path.is_file() {
                    // Single file, offset 0?
                    total = 1;

                    if offset == 0 {
                        entries.push(json!({
                            "path": path,
                            "type": "file",
                        }));
                        self.lease_store.touch_files(&lid, vec![path.to_string()]).await?;
                    }
                } else {
                    // Dir
                    let mut raw_entries = Vec::new();
                    for entry in std::fs::read_dir(&target_path)? {
                        let entry = entry?;
                        let ftype = entry.file_type()?;
                        let fname = entry.file_name();
                        let fname_str = fname.to_string_lossy();

                        if fname_str.starts_with('.') && fname_str != ".gitignore" {
                            continue;
                        }

                        let rel = if path.is_empty() {
                            fname_str.to_string()
                        } else {
                            format!("{}/{}", path, fname_str)
                        };

                        let type_str = if ftype.is_dir() { "dir" } else { "file" };
                        raw_entries.push((rel, type_str.to_string()));
                    }

                    // Sort BEFORE paging for determinism
                    raw_entries.sort_by(|a, b| a.0.cmp(&b.0));

                    // Total count
                    total = raw_entries.len();

                    // Collect touches to batch
                    let mut files_to_touch = Vec::new();

                    // Apply paging
                    let end = std::cmp::min(offset + limit, total);
                    if offset < total {
                        for (rel, type_str) in &raw_entries[offset..end] {
                            entries.push(json!({
                                "path": rel,
                                "type": type_str,
                            }));

                            // Touch child if file?
                            if type_str == "file" {
                                files_to_touch.push(rel.clone());
                            }
                        }
                    }

                    if !files_to_touch.is_empty() {
                        self.lease_store.touch_files(&lid, files_to_touch).await?;
                    }
                }
            } else {
                total = 0;
            }

            // Re-sort handled above

            let fp = self.lease_store.get_fingerprint(&lid).await
                .ok_or_else(|| anyhow!("Lease expired unexpectedly after validation"))?;

            Ok(json!({
                "snapshot_id": format!("sha256:{}", fp.status_hash),
                "path": path,
                "mode": "worktree",
                "entries": entries,
                "total": total,
                "truncated": (offset + limit) < total,
                "lease_id": lid,
                "fingerprint": fp,
                "cache_key": format!("{}:sha256:{}", lid, fp.status_hash),
                "cache_hint": "until_dirty"
            }))
        } else if mode == "snapshot" {
            // Snapshot mode
            let snap_id =
                snapshot_id.ok_or_else(|| anyhow!("snapshot_id required for snapshot mode"))?;

            self.store.validate_snapshot(&snap_id).await?;
            let manifest_entries = self.store.list_snapshot_entries(&snap_id).await?;

            let mut result_entries = Vec::new();
            let mut dirs_seen = std::collections::HashSet::new();

            // We need to filter ALL first to sort and page deterministically.
            // Streaming/iterator approach is harder with sort requirement unless source is sorted.
            // Manifest is sorted by path? Yes.
            // But we filter by prefix.

            let mut temp_entries = Vec::new();

            for entry in manifest_entries {
                if path.is_empty() || entry.path.starts_with(path) {
                    let relative = if path.is_empty() {
                        entry.path.clone()
                    } else {
                        if entry.path == path {
                            continue;
                        }
                        if !entry.path.starts_with(&format!("{}/", path)) {
                            continue;
                        }
                        entry
                            .path
                            .strip_prefix(&format!("{}/", path))
                            .unwrap_or(&entry.path)
                            .to_string()
                    };

                    if let Some((dir, _)) = relative.split_once('/') {
                        if dirs_seen.insert(dir.to_string()) {
                            temp_entries.push(json!({
                                 "path": if path.is_empty() { dir.to_string() } else { format!("{}/{}", path, dir) },
                                 "type": "dir"
                             }));
                        }
                    } else {
                        temp_entries.push(json!({
                            "path": entry.path,
                            "type": "file",
                            "size": entry.size,
                            "sha": entry.blob
                        }));
                    }
                }
            }

            temp_entries.sort_by(|a, b| {
                let pa = a.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let pb = b.get("path").and_then(|v| v.as_str()).unwrap_or("");
                pa.cmp(pb)
            });

            let total = temp_entries.len();
            let end = std::cmp::min(offset + limit, total);

            if offset < total {
                result_entries.extend_from_slice(&temp_entries[offset..end]);
            }

            Ok(json!({
                "snapshot_id": snap_id,
                "path": path,
                "mode": "snapshot",
                "entries": result_entries,
                "truncated": (offset + limit) < total, // Simple check
                "total": total,
                "cache_key": snap_id,
                "cache_hint": "immutable"
            }))
        } else {
            Err(anyhow!("Invalid mode"))
        }
    }

    pub async fn snapshot_create(
        &self,
        repo_root: &Path,
        lease_id: Option<String>,
        paths: Option<Vec<String>>,
    ) -> Result<serde_json::Value> {
        // Must have lease or issue one for "touched" set?
        // If paths provided, explicit. If not, touched.

        let repo_root = repo_root.canonicalize()?;
        let mut lid = lease_id.clone();

        // If no lease and no paths, error? Or issue new lease and capture nothing (empty)?
        // Using "touched" implies we had a session.
        // If lease provided, check it.
        if let Some(ref l) = lid {
            self.lease_store.check_lease(l, &repo_root).await?;
        } else {
            // Issue
            let fp = Fingerprint::compute(&repo_root).await?;
            lid = Some(self.lease_store.issue(fp).await?);
        }
        let lid_str = lid.unwrap();

        let files_to_capture = if let Some(p) = paths {
            p
        } else {
            // Use touched
            self.lease_store
                .get_touched_files(&lid_str)
                .await
                .unwrap_or_default()
        };

        let mut entries = Vec::new();
        for path_str in files_to_capture {
            // Validate path format
            Store::validate_path(&path_str)?;

            let p = self.resolve_path(&repo_root, &path_str)?;
            if p.is_file() {
                let content = std::fs::read(&p)?;
                let blob_hash = self.store.put_blob(&content)?;
                entries.push(Entry {
                    path: path_str,
                    blob: blob_hash,
                    size: content.len() as u64,
                });
            }
        }

        let manifest = Manifest::new(entries);
        let fp = self.lease_store.get_fingerprint(&lid_str).await
            .ok_or_else(|| anyhow!("Lease expired unexpectedly after validation"))?;
        let fp_json = fp.to_canonical_json()?;
        let snap_id = manifest.compute_snapshot_id(&fp_json)?;

        // Store manifest
        let manifest_bytes = manifest.to_canonical_json()?.into_bytes();
        self.store.put_snapshot(
            &snap_id,
            &repo_root.to_string_lossy(),
            &fp.head_oid,
            &fp_json,
            &manifest_bytes,
            None,
            None,
            None,
        ).await?;

        Ok(json!({
            "snapshot_id": snap_id,
            "repo_root": repo_root.to_string_lossy(),
            "head_sha": fp.head_oid,
            "cache_key": snap_id,
            "cache_hint": "immutable"
        }))
    }

    pub async fn snapshot_file(
        &self,
        repo_root: &Path,
        path: &str,
        mode: &str,
        lease_id: Option<String>,
        snapshot_id: Option<String>,
    ) -> Result<serde_json::Value> {
        let repo_root = repo_root.canonicalize()?;

        if mode == "worktree" {
            let lid = self.check_lease(lease_id.as_deref(), &repo_root).await?;
            let target_path = self.resolve_path(&repo_root, path)?;

            if !target_path.exists() || !target_path.is_file() {
                return Err(anyhow!("File not found or not a file: {}", path));
            }

            let content = std::fs::read(&target_path)?;
            // base64 encode
            use base64::{Engine as _, engine::general_purpose};
            let encoded = general_purpose::STANDARD.encode(&content);
            let blob_hash = format!("sha256:{}", hex::encode(Sha256::digest(&content))); // Optional return?

            self.lease_store.touch_files(&lid, vec![path.to_string()]).await?;
            let fp = self.lease_store.get_fingerprint(&lid).await
                .ok_or_else(|| anyhow!("Lease expired unexpectedly after validation"))?;

            // detect kind?
            // Simple heuristic or just "text" vs "binary"?
            // Spec says "kind" enum [text, binary].
            // We can check for null bytes?
            let kind = if content.contains(&0) {
                "binary"
            } else {
                "text"
            };

            Ok(json!({
                "snapshot_id": format!("sha256:{}", fp.status_hash),
                "path": path,
                "mode": "worktree",
                "content": format!("base64:{}", encoded),
                "kind": kind,
                "size": content.len(),
                "sha": blob_hash,
                "lease_id": lid,
                "fingerprint": fp,
                "cache_key": format!("{}:sha256:{}", lid, fp.status_hash), // Simple cache key
                "cache_hint": "until_dirty"
            }))
        } else if mode == "snapshot" {
            let snap_id =
                snapshot_id.ok_or_else(|| anyhow!("snapshot_id required for snapshot mode"))?;

            // Validate snapshot integrity first
            self.store.validate_snapshot(&snap_id).await?;

            // retrieve manifest
            let manifest_entries = self.store.list_snapshot_entries(&snap_id).await?;

            // find entry
            let entry = manifest_entries
                .iter()
                .find(|e| e.path == path)
                .ok_or_else(|| anyhow!("File not found in snapshot: {}", path))?;

            let content = self.store.get_blob(&entry.blob)?.ok_or_else(|| {
                anyhow!(
                    "Snapshot corrupted: referenced blob {} not found in store",
                    entry.blob
                )
            })?;

            use base64::{Engine as _, engine::general_purpose};
            let encoded = general_purpose::STANDARD.encode(&content);
            let kind = if content.contains(&0) {
                "binary"
            } else {
                "text"
            };

            Ok(json!({
                "snapshot_id": snap_id,
                "path": path,
                "mode": "snapshot",
                "content": format!("base64:{}", encoded),
                "kind": kind,
                "size": content.len(),
                "sha": entry.blob,
                "cache_key": entry.blob, // Blob hash is good cache key
                "cache_hint": "immutable"
            }))
        } else {
            Err(anyhow!("Invalid mode"))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn snapshot_grep(
        &self,
        repo_root: &Path,
        pattern: &str, // regex
        paths: Option<Vec<String>>,
        mode: &str,
        lease_id: Option<String>,
        snapshot_id: Option<String>,
        case_insensitive: bool,
    ) -> Result<serde_json::Value> {
        let repo_root = repo_root.canonicalize()?;

        let mut builder = regex::RegexBuilder::new(pattern);
        if case_insensitive {
            builder.case_insensitive(true);
        }
        let re = builder
            .build()
            .map_err(|e| anyhow!("Invalid regex: {}", e))?;

        if mode == "worktree" {
            let lid = self.check_lease(lease_id.as_deref(), &repo_root).await?;

            let roots = if let Some(p) = paths {
                p.iter()
                    .map(|s| self.resolve_path(&repo_root, s))
                    .collect::<Result<Vec<_>>>()?
            } else {
                vec![repo_root.clone()]
            };

            // Using BTreeMap to keep file order if we wanted encoded order,
            // but for now Vec is fine if we push in order.
            // Actually, we process files in order.
            let mut matches: Vec<serde_json::Value> = Vec::new();
            let mut candidates_touched = Vec::new();
            let mut truncated = false;
            let mut total_matches = 0;

            for root in roots {
                if truncated {
                    break;
                }
                for entry in walkdir::WalkDir::new(&root).sort_by_file_name() {
                    let entry = entry?;
                    if entry.file_type().is_file() {
                        let path = entry.path();
                        let rel = path.strip_prefix(&repo_root)?.to_string_lossy().to_string();

                        candidates_touched.push(rel.clone());

                        let mut f = std::fs::File::open(path)?;
                        let mut buffer = [0; 512];
                        let n = std::io::Read::read(&mut f, &mut buffer)?;
                        if buffer[..n].contains(&0) {
                            continue;
                        }

                        let content = std::fs::read_to_string(path)?;
                        let mut file_lines = Vec::new();

                        for (i, line) in content.lines().enumerate() {
                            if re.is_match(line) {
                                file_lines.push(json!({
                                    "line": i + 1,
                                    "col": 1, // Regex doesn't give col easily without more work, stub 1
                                    "text": line
                                }));
                                total_matches += 1;
                                if total_matches >= 100 {
                                    truncated = true;
                                    break;
                                }
                            }
                        }

                        if !file_lines.is_empty() {
                            matches.push(json!({
                                "path": rel,
                                "lines": file_lines
                            }));
                        }

                        if truncated {
                            break;
                        }
                    }
                }
            }
            self.lease_store.touch_files(&lid, candidates_touched).await?;

            let fp = self.lease_store.get_fingerprint(&lid).await
                .ok_or_else(|| anyhow!("Lease expired unexpectedly after validation"))?;

            Ok(json!({
                "snapshot_id": format!("sha256:{}", fp.status_hash), // Stable worktree ID
                "query": pattern,
                "mode": "worktree",
                "matches": matches,
                "truncated": truncated,
                "lease_id": lid,
                "fingerprint": fp,
                "cache_key": format!("{}:grep:{}:sha256:{}", lid, pattern, fp.status_hash),
                "cache_hint": "until_dirty"
            }))
        } else {
            let sid =
                snapshot_id.ok_or_else(|| anyhow!("snapshot_id required in snapshot mode"))?;
            // Validate snapshot integrity first
            self.store.validate_snapshot(&sid).await?;

            let manifest_entries = self.store.list_snapshot_entries(&sid).await?;

            let mut matches: Vec<serde_json::Value> = Vec::new();
            let mut truncated = false;
            let mut total_matches = 0;

            // Filter by paths if provided
            let mut candidate_entries = Vec::new();
            if let Some(ref p_list) = paths {
                for entry in manifest_entries {
                    // Check if entry.path is in p_list or starts with one of them + /
                    // Simple prefix check isn't enough because "src/foobar" shouldn't match "src/foo".
                    // Logic: for each p in p_list:
                    // if p == entry.path OR entry.path.starts_with(p + "/")
                    let mut keep = false;
                    for p in p_list {
                        if entry.path == *p || entry.path.starts_with(&format!("{}/", p)) {
                            keep = true;
                            break;
                        }
                    }
                    if keep {
                        candidate_entries.push(entry);
                    }
                }
            } else {
                candidate_entries = manifest_entries;
            }

            // Iterate candidates (already sorted by manifest order)
            for entry in candidate_entries {
                if truncated {
                    break;
                }

                // Get blob content
                if let Some(content) = self.store.get_blob(&entry.blob)? {
                    // Binary check
                    if content.iter().take(512).any(|&b| b == 0) {
                        continue;
                    }

                    // Decode as string
                    // If valid utf8?
                    if let Ok(text) = String::from_utf8(content) {
                        let mut file_lines = Vec::new();
                        for (i, line) in text.lines().enumerate() {
                            if re.is_match(line) {
                                file_lines.push(json!({
                                    "line": i + 1,
                                    "col": 1,
                                    "text": line
                                }));
                                total_matches += 1;
                                if total_matches >= 100 {
                                    truncated = true;
                                    break;
                                }
                            }
                        }

                        if !file_lines.is_empty() {
                            matches.push(json!({
                                "path": entry.path,
                                "lines": file_lines
                            }));
                        }
                    }
                } else {
                    // Missing blob? store.get_blob returns None if not found in db/fs.
                    // validate_snapshot should have caught this, but safeguard:
                    return Err(anyhow!("Snapshot missing blob: {}", entry.blob));
                }
            }

            Ok(json!({
                "snapshot_id": sid,
                "query": pattern,
                "mode": "snapshot",
                "matches": matches,
                "truncated": truncated,
                "cache_key": sid, // In snapshot mode, result stable for (sid, pattern)
                "cache_hint": "immutable"
            }))
        }
    }

    pub async fn snapshot_diff(
        &self,
        repo_root: &Path,
        path: &str,
        mode: &str,
        lease_id: Option<String>,
        snapshot_id: Option<String>,
        from_snapshot_id: Option<String>,
    ) -> Result<serde_json::Value> {
        let repo_root = repo_root.canonicalize()?;

        if mode == "worktree" {
            let lid = self.check_lease(lease_id.as_deref(), &repo_root).await?;
            let _target_path = self.resolve_path(&repo_root, path)?;

            // `git diff -- path`
            // touches target
            self.lease_store.touch_files(&lid, vec![path.to_string()]).await?;

            // run git diff
            let output = tokio::process::Command::new("git")
                .args(["diff", "--", path])
                .current_dir(&repo_root)
                .output()
                .await?;

            let diff_text = String::from_utf8_lossy(&output.stdout).to_string();
            let fp = self.lease_store.get_fingerprint(&lid).await
                .ok_or_else(|| anyhow!("Lease expired unexpectedly after validation"))?;

            Ok(json!({
                "diff": diff_text,
                "lease_id": lid,
                "fingerprint": fp,
                "cache_key": format!("{}:diff:sha256:{}", lid, fp.status_hash),
                "cache_hint": "until_dirty"
            }))
        } else if mode == "snapshot" {
            let sid =
                snapshot_id.ok_or_else(|| anyhow!("snapshot_id required in snapshot mode"))?;
            self.store.validate_snapshot(&sid).await?;

            // Get content from target snapshot
            let mut target_content = String::new();
            let mut target_found = false;

            // Find entry in snapshot
            if let Ok(entries) = self.store.list_snapshot_entries(&sid).await
                && let Some(entry) = entries.iter().find(|e| e.path == path)
                && let Some(blob) = self.store.get_blob(&entry.blob)?
            {
                // Try decode utf8, if binary?
                // similar can diff bytes but usually we diff text.
                // If binary, return empty or "Binary files differ"?
                if blob.iter().take(512).any(|&b| b == 0) {
                    return Ok(json!({
                        "diff": format!("Binary files a/{} and b/{} differ", path, path),
                        "snapshot_id": sid,
                        "cache_hint": "immutable"
                    }));
                }
                target_content = String::from_utf8_lossy(&blob).to_string();
                target_found = true;
            }

            // Get content from base snapshot (if provided)
            let mut base_content = String::new();
            let mut base_found = false;

            if let Some(from_sid) = from_snapshot_id {
                self.store.validate_snapshot(&from_sid).await?;
                if let Ok(entries) = self.store.list_snapshot_entries(&from_sid).await
                    && let Some(entry) = entries.iter().find(|e| e.path == path)
                    && let Some(blob) = self.store.get_blob(&entry.blob)?
                {
                    if blob.iter().take(512).any(|&b| b == 0) {
                        // Base is binary.
                        // If target also binary, handled above?
                        // If target text, base binary -> "Binary files differ"?
                        return Ok(json!({
                            "diff": format!("Binary files a/{} and b/{} differ", path, path),
                            "snapshot_id": sid,
                            "cache_hint": "immutable"
                        }));
                    }
                    base_content = String::from_utf8_lossy(&blob).to_string();
                    base_found = true;
                }
            }

            // If neither found, empty diff (or error?)
            if !target_found && !base_found {
                return Err(anyhow!("Path not found in either snapshot: {}", path));
            }

            // Compute diff
            use similar::{ChangeTag, TextDiff};

            let diff = TextDiff::from_lines(&base_content, &target_content);
            let mut diff_str = String::new();

            // Imitate git unified diff header
            // "diff --git a/path b/path"
            // "index ..."
            // "--- a/path"
            // "+++ b/path"

            // Only if different
            if base_content != target_content {
                diff_str.push_str(&format!("diff --git a/{} b/{}\n", path, path));
                // index? stub
                if !base_found {
                    diff_str.push_str(&format!(
                        "new file mode 100644\n--- /dev/null\n+++ b/{}\n",
                        path
                    ));
                } else if !target_found {
                    diff_str.push_str(&format!(
                        "deleted file mode 100644\n--- a/{}\n+++ /dev/null\n",
                        path
                    ));
                } else {
                    diff_str.push_str(&format!("--- a/{}\n+++ b/{}\n", path, path));
                }

                for group in diff.grouped_ops(3) {
                    for op in group {
                        for change in diff.iter_changes(&op) {
                            let sign = match change.tag() {
                                ChangeTag::Delete => "-",
                                ChangeTag::Insert => "+",
                                ChangeTag::Equal => " ",
                            };
                            diff_str.push_str(&format!("{}{}", sign, change));
                        }
                    }
                }
            }

            Ok(json!({
                "diff": diff_str,
                "snapshot_id": sid,
                "cache_key": sid,
                "cache_hint": "immutable"
            }))
        } else {
            Err(anyhow!("Invalid mode"))
        }
    }
    pub async fn snapshot_changes(
        &self,
        _repo_root: &Path,
        snapshot_id: Option<String>,
        from_snapshot_id: Option<String>,
    ) -> Result<serde_json::Value> {
        let snap_id = snapshot_id.ok_or_else(|| anyhow!("snapshot_id required"))?;
        self.store.validate_snapshot(&snap_id).await?;

        let mut changes = Vec::new();

        if let Some(from_sid) = from_snapshot_id {
            self.store.validate_snapshot(&from_sid).await?;

            let target_entries = self.store.list_snapshot_entries(&snap_id).await?;
            let base_entries = self.store.list_snapshot_entries(&from_sid).await?;

            // Use maps for easier lookup? list_snapshot_entries returns sorted Vec<Entry>.
            // Since sorted, we can iterate in parallel or use map. Map is easier.
            use std::collections::HashMap;
            let target_map: HashMap<_, _> =
                target_entries.iter().map(|e| (&e.path, &e.blob)).collect();
            let base_map: HashMap<_, _> = base_entries.iter().map(|e| (&e.path, &e.blob)).collect();

            // Check for modified and added
            for (path, blob) in &target_map {
                match base_map.get(path) {
                    Some(base_blob) => {
                        if blob != base_blob {
                            changes.push(json!({ "path": path, "type": "modified" }));
                        }
                    }
                    None => {
                        changes.push(json!({ "path": path, "type": "added" }));
                    }
                }
            }

            // Check for deleted
            for path in base_map.keys() {
                if !target_map.contains_key(path) {
                    changes.push(json!({ "path": path, "type": "deleted" }));
                }
            }
        }

        // Sort changes by path for determinism
        changes.sort_by(|a, b| {
            let pa = a["path"].as_str().unwrap();
            let pb = b["path"].as_str().unwrap();
            pa.cmp(pb)
        });

        Ok(json!({
             "snapshot_id": snap_id,
             "files_changed": changes,
             "cache_hint": "immutable"
        }))
    }

    pub async fn snapshot_export(
        &self,
        _repo_root: &Path,
        snapshot_id: Option<String>,
    ) -> Result<serde_json::Value> {
        let snap_id = snapshot_id.ok_or_else(|| anyhow!("snapshot_id required"))?;
        // Validate snapshot integrity first
        self.store.validate_snapshot(&snap_id).await?;

        let entries = self.store.list_snapshot_entries(&snap_id).await?;
        let mut tar_builder = tar::Builder::new(Vec::new());

        // Sort entries by path for deterministic order (list_snapshot_entries already sorts, but verify?)
        // Store implementation ensures sort.

        let mut included_files = 0;
        let mut total_bytes = 0;

        for entry in entries {
            if let Some(content) = self.store.get_blob(&entry.blob)? {
                let mut header = tar::Header::new_gnu();
                header.set_size(content.len() as u64);
                header.set_mode(0o644); // Regular file
                header.set_mtime(0); // Epoch for determinism
                header.set_uid(0);
                header.set_gid(0);
                header.set_cksum();

                // Use entry.path as path in tar
                // Ensure no leading slash for tar safety?
                // entry.path is relative "a/b/c.txt".
                tar_builder.append_data(&mut header, &entry.path, &content[..])?;

                included_files += 1;
                total_bytes += content.len();
            } else {
                return Err(anyhow!("Missing blob for {}", entry.path));
            }
        }

        // Include manifest.json?
        // Maybe useful metadata.
        // Let's stick to just the files for now - user wants the WORKSPACE state exported.

        let tar_bytes = tar_builder.into_inner()?;
        let encoded_bundle = base64::engine::general_purpose::STANDARD.encode(&tar_bytes);

        Ok(json!({
            "snapshot_id": snap_id,
            "format": "tar",
            "summary": {
                "included_files": included_files,
                "included_bytes": total_bytes,
                "truncated": false
            },
            "bundle": format!("base64:{}", encoded_bundle),
            "cache_key": snap_id,
            "cache_hint": "immutable"
        }))
    }

    pub async fn snapshot_info(
        &self,
        repo_root: &Path,
        snapshot_id: Option<String>,
    ) -> Result<serde_json::Value> {
        // If snapshot_id provided, return details about THAT snapshot (metadata, lineage)
        if let Some(sid) = snapshot_id {
            let info = self
                .store
                .get_snapshot_info(&sid).await?
                .ok_or_else(|| anyhow!("Snapshot not found: {}", sid))?;

            // Retrieve stats via manifest? Or just computed?
            // Store saves manifest_hash but not stats directly in snapshots table (only in blobs refcounts or separate entry count).
            // We can do a quick list to get count/size if needed, but SnapshotInfo doesn't have it.
            // Let's rely on info structure.

            Ok(json!({
                "snapshot_id": info.snapshot_id,
                "repo_root": info.repo_root,
                "head_sha": info.head_sha,
                "created_at": info.created_at,
                "manifest_hash": info.manifest_hash,
                "derived_from": info.derived_from,
                "applied_patch_hash": info.applied_patch_hash,
                "label": info.label,
                "cache_hint": "immutable"
            }))
        } else {
            // Return current worktree fingerprint with actual file stats.
            let fp = tokio::runtime::Handle::current()
                .block_on(Fingerprint::compute(repo_root))?;
            let (files, bytes) = compute_worktree_stats(repo_root);
            Ok(json!({
                "fingerprint": fp,
                "manifest_stats": {
                    "files": files,
                    "bytes": bytes
                },
                "cache_hint": "until_dirty"
            }))
        }
    }
}

/// Walk repo root (excluding .git and .axiomregent) and return (file_count, total_bytes).
fn compute_worktree_stats(repo_root: &Path) -> (u64, u64) {
    let mut files = 0u64;
    let mut bytes = 0u64;
    let walker = walkdir::WalkDir::new(repo_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != ".axiomregent"
        });
    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            files += 1;
            if let Ok(meta) = entry.metadata() {
                bytes += meta.len();
            }
        }
    }
    (files, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BlobBackend, Compression, StorageConfig};
    use crate::snapshot::lease::LeaseStore;

    async fn make_lease_store(dir: &std::path::Path) -> Arc<LeaseStore> {
        let client = crate::db::init_hiqlite(dir).await.unwrap();
        Arc::new(LeaseStore::new(client))
    }

    #[tokio::test]
    async fn test_snapshot_grep_basics() {
        let dir = tempfile::tempdir().unwrap();
        let client = crate::db::init_hiqlite(dir.path()).await.unwrap();
        let config = StorageConfig {
            data_dir: dir.path().to_path_buf(),
            blob_backend: BlobBackend::Fs,
            compression: Compression::None,
        };
        let store = Arc::new(Store::new(client.clone(), config).unwrap());
        // Lease store needed but not used for snapshot-only mode
        let lease_store = make_lease_store(dir.path()).await;
        let tools = SnapshotTools::new(lease_store, store.clone());

        // Create blobs
        let t1 = "line one\nline MATCH two\nline three";
        let h1 = store.put_blob(t1.as_bytes()).unwrap();

        let t2 = "another file\nnothing interesting here";
        let h2 = store.put_blob(t2.as_bytes()).unwrap();

        // Binary blob
        let mut b_data = vec![0u8; 10];
        b_data.extend_from_slice(b"match in binary");
        let h_bin = store.put_blob(&b_data).unwrap();

        // Create snapshot with ordered paths
        let manifest_bytes = format!(
            r#"{{
            "entries": [
                {{ "path": "a/text1.txt", "blob": "{}", "size": {} }},
                {{ "path": "b/binary.bin", "blob": "{}", "size": {} }},
                {{ "path": "c/text2.txt", "blob": "{}", "size": {} }}
            ]
        }}"#,
            h1,
            t1.len(),
            h_bin,
            b_data.len(),
            h2,
            t2.len()
        );

        let sid = "snap-grep";
        // put_snapshot uses string, need valid path?
        // put_snapshot arguments are metadata, repo_root doesn't need to exist for put_snapshot storage,
        // BUT snapshot_grep calls canonicalize(repo_root).
        // So we must pass `dir.path()` (which exists) to grep.
        store
            .put_snapshot(
                sid,
                dir.path().to_str().unwrap(),
                "h1",
                "{}",
                manifest_bytes.as_bytes(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 1. Grep all - should find matches in text1, skip binary
        let res = tools
            .snapshot_grep(
                dir.path(),
                "match",
                None,
                "snapshot",
                None,
                Some(sid.to_string()),
                true, // case insensitive
            )
            .await
            .unwrap();

        let matches = res["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1); // text1 only. binary skipped. text2 no match.
        assert_eq!(matches[0]["path"], "a/text1.txt");
        assert_eq!(matches[0]["lines"][0]["text"], "line MATCH two");

        // 2. Grep with path filter
        let res2 = tools
            .snapshot_grep(
                dir.path(),
                "match",
                Some(vec!["c".to_string()]),
                "snapshot",
                None,
                Some(sid.to_string()),
                true,
            )
            .await
            .unwrap();
        assert!(res2["matches"].as_array().unwrap().is_empty()); // c/text2 has no match

        let res3 = tools
            .snapshot_grep(
                dir.path(),
                "match",
                Some(vec!["a".to_string()]),
                "snapshot",
                None,
                Some(sid.to_string()),
                true,
            )
            .await
            .unwrap();
        assert_eq!(res3["matches"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_snapshot_diff() {
        let dir = tempfile::tempdir().unwrap();
        let client = crate::db::init_hiqlite(dir.path()).await.unwrap();
        let config = StorageConfig {
            data_dir: dir.path().to_path_buf(),
            blob_backend: BlobBackend::Fs,
            compression: Compression::None,
        };
        let store = Arc::new(Store::new(client.clone(), config).unwrap());
        let lease_store = make_lease_store(dir.path()).await;
        let tools = SnapshotTools::new(lease_store, store.clone());

        // Snapshot 1 (Base)
        // a.txt: "version1"
        // b.txt: "deleted later"
        let t1 = "version1\n";
        let h1 = store.put_blob(t1.as_bytes()).unwrap();
        let t2 = "deleted later\n";
        let h2 = store.put_blob(t2.as_bytes()).unwrap();

        let m1 = format!(
            r#"{{
            "entries": [
                {{ "path": "a.txt", "blob": "{}", "size": {} }},
                {{ "path": "b.txt", "blob": "{}", "size": {} }}
            ]
        }}"#,
            h1,
            t1.len(),
            h2,
            t2.len()
        );

        let sid1 = "snap1";
        store
            .put_snapshot(
                sid1,
                dir.path().to_str().unwrap(),
                "h1",
                "{}",
                m1.as_bytes(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Snapshot 2 (Target)
        // a.txt: "version2"
        // c.txt: "new file"
        // b.txt missing (deleted)
        let t3 = "version2\n";
        let h3 = store.put_blob(t3.as_bytes()).unwrap();
        let t4 = "new file\n";
        let h4 = store.put_blob(t4.as_bytes()).unwrap();

        let m2 = format!(
            r#"{{
            "entries": [
                {{ "path": "a.txt", "blob": "{}", "size": {} }},
                {{ "path": "c.txt", "blob": "{}", "size": {} }}
            ]
        }}"#,
            h3,
            t3.len(),
            h4,
            t4.len()
        );

        let sid2 = "snap2";
        store
            .put_snapshot(
                sid2,
                dir.path().to_str().unwrap(),
                "h2",
                "{}",
                m2.as_bytes(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 1. Diff a.txt (Modified)
        let res = tools
            .snapshot_diff(
                dir.path(),
                "a.txt",
                "snapshot",
                None,
                Some(sid2.to_string()),
                Some(sid1.to_string()),
            )
            .await
            .unwrap();
        let diff = res["diff"].as_str().unwrap();
        assert!(diff.contains("--- a/a.txt"));
        assert!(diff.contains("+++ b/a.txt"));
        assert!(diff.contains("-version1"));
        assert!(diff.contains("+version2"));

        // 2. Diff b.txt (Deleted in snap2)
        // Note: snapshot_diff iterates both snapshots to find blobs.
        // If file not in target but in base -> deleted.
        let res = tools
            .snapshot_diff(
                dir.path(),
                "b.txt",
                "snapshot",
                None,
                Some(sid2.to_string()),
                Some(sid1.to_string()),
            )
            .await
            .unwrap();
        let diff = res["diff"].as_str().unwrap();
        assert!(diff.contains("deleted file"));
        assert!(diff.contains("--- a/b.txt"));
        assert!(diff.contains("+++ /dev/null"));
        assert!(diff.contains("-deleted later"));

        // 3. Diff c.txt (New in snap2)
        let res = tools
            .snapshot_diff(
                dir.path(),
                "c.txt",
                "snapshot",
                None,
                Some(sid2.to_string()),
                Some(sid1.to_string()),
            )
            .await
            .unwrap();
        let diff = res["diff"].as_str().unwrap();
        assert!(diff.contains("new file"));
        assert!(diff.contains("--- /dev/null"));
        assert!(diff.contains("+++ b/c.txt"));
        assert!(diff.contains("+new file"));
    }

    #[tokio::test]
    async fn test_snapshot_export() {
        let dir = tempfile::tempdir().unwrap();
        let client = crate::db::init_hiqlite(dir.path()).await.unwrap();
        let config = StorageConfig {
            data_dir: dir.path().to_path_buf(),
            blob_backend: BlobBackend::Fs,
            compression: Compression::None,
        };
        let store = Arc::new(Store::new(client.clone(), config).unwrap());
        let lease_store = make_lease_store(dir.path()).await;
        let tools = SnapshotTools::new(lease_store, store.clone());

        // Create content
        let t1 = "export me";
        let h1 = store.put_blob(t1.as_bytes()).unwrap();
        let t2 = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let h2 = store.put_blob(&t2).unwrap();

        let m1 = format!(
            r#"{{
            "entries": [
                {{ "path": "a.txt", "blob": "{}", "size": {} }},
                {{ "path": "b/data.bin", "blob": "{}", "size": {} }}
            ]
        }}"#,
            h1,
            t1.len(),
            h2,
            t2.len()
        );

        let sid = "snap-export";
        store
            .put_snapshot(
                sid,
                dir.path().to_str().unwrap(),
                "h1",
                "{}",
                m1.as_bytes(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Export
        let res = tools
            .snapshot_export(dir.path(), Some(sid.to_string()))
            .await
            .unwrap();
        let bundle_b64 = res["bundle"]
            .as_str()
            .unwrap()
            .strip_prefix("base64:")
            .unwrap();
        let tar_bytes = base64::engine::general_purpose::STANDARD
            .decode(bundle_b64)
            .unwrap();

        // Verify tar content
        let mut archive = tar::Archive::new(&tar_bytes[..]);
        let mut found_a = false;
        let mut found_b = false;

        for file in archive.entries().unwrap() {
            let mut file = file.unwrap();
            let path = file.path().unwrap().into_owned();

            if path.to_str().unwrap() == "a.txt" {
                found_a = true;
                let mut s = String::new();
                use std::io::Read;
                file.read_to_string(&mut s).unwrap();
                assert_eq!(s, "export me");
                assert_eq!(file.header().mtime().unwrap(), 0); // Determinism check
            } else if path.to_str().unwrap() == "b/data.bin" {
                found_b = true;
                let mut v = Vec::new();
                use std::io::Read;
                file.read_to_end(&mut v).unwrap();
                assert_eq!(v, vec![0xDE, 0xAD, 0xBE, 0xEF]);
            }
        }

        assert!(found_a);
        assert!(found_b);

        // Determinism Check
        let res2 = tools
            .snapshot_export(dir.path(), Some(sid.to_string()))
            .await
            .unwrap();
        assert_eq!(res["bundle"], res2["bundle"]);
    }

    #[tokio::test]
    async fn test_snapshot_changes() {
        let dir = tempfile::tempdir().unwrap();
        let client = crate::db::init_hiqlite(dir.path()).await.unwrap();
        let config = StorageConfig {
            data_dir: dir.path().to_path_buf(),
            blob_backend: BlobBackend::Fs,
            compression: Compression::None,
        };
        let store = Arc::new(Store::new(client.clone(), config).unwrap());
        let lease_store = make_lease_store(dir.path()).await;
        let tools = SnapshotTools::new(lease_store, store.clone());

        // Base Snapshot
        let t1 = "base-version";
        let h1 = store.put_blob(t1.as_bytes()).unwrap();
        let t2 = "will-delete";
        let h2 = store.put_blob(t2.as_bytes()).unwrap();

        let m1 = format!(
            r#"{{
            "entries": [
                {{ "path": "a.txt", "blob": "{}", "size": {} }},
                {{ "path": "b.txt", "blob": "{}", "size": {} }}
            ]
        }}"#,
            h1,
            t1.len(),
            h2,
            t2.len()
        );

        let sid1 = "snap-base";
        store
            .put_snapshot(
                sid1,
                dir.path().to_str().unwrap(),
                "h1",
                "{}",
                m1.as_bytes(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Target Snapshot
        let t3 = "target-version"; // modified
        let h3 = store.put_blob(t3.as_bytes()).unwrap();
        let t4 = "new-file"; // added
        let h4 = store.put_blob(t4.as_bytes()).unwrap();

        let m2 = format!(
            r#"{{
            "entries": [
                {{ "path": "a.txt", "blob": "{}", "size": {} }},
                {{ "path": "c.txt", "blob": "{}", "size": {} }}
            ]
        }}"#,
            h3,
            t3.len(),
            h4,
            t4.len()
        );

        let sid2 = "snap-target";
        store
            .put_snapshot(
                sid2,
                dir.path().to_str().unwrap(),
                "h2",
                "{}",
                m2.as_bytes(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Check changes
        let res = tools
            .snapshot_changes(dir.path(), Some(sid2.to_string()), Some(sid1.to_string()))
            .await
            .unwrap();

        let changes = res["files_changed"].as_array().unwrap();
        assert_eq!(changes.len(), 3);

        // changes are sorted by path: a.txt, b.txt, c.txt
        let c0 = &changes[0];
        assert_eq!(c0["path"], "a.txt");
        assert_eq!(c0["type"], "modified");

        let c1 = &changes[1];
        assert_eq!(c1["path"], "b.txt");
        assert_eq!(c1["type"], "deleted");

        let c2 = &changes[2];
        assert_eq!(c2["path"], "c.txt");
        assert_eq!(c2["type"], "added");
    }
}
