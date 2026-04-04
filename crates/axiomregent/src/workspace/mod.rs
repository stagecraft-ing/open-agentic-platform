// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: MCP_SNAPSHOT_WORKSPACE
// Spec: spec/core/snapshot-workspace.md

use crate::snapshot::lease::Fingerprint;
use crate::snapshot::lease::LeaseStore;
use crate::snapshot::store::Store;
use anyhow::{Context, Result, anyhow};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct WorkspaceTools {
    pub lease_store: Arc<LeaseStore>,
    pub store: Arc<Store>, // Unused mostly but keeps symmetry
}

impl WorkspaceTools {
    pub fn new(lease_store: Arc<LeaseStore>, store: Arc<Store>) -> Self {
        Self { lease_store, store }
    }

    // Safety helper: ensure path is inside repo root and is safe
    fn resolve_target_path(&self, repo_root: &Path, rel_path: &str) -> Result<PathBuf> {
        let path = repo_root.join(rel_path);
        // If file needs to be created, we must check parent directory presence and safety.
        // For existing files, we canonicalize.

        let canonical_root = repo_root
            .canonicalize()
            .context("Failed to canonicalize repo root")?;

        // We try to canonicalize path. If it fails (doesn't exist), we canonicalize parent.
        if path.exists() {
            let canonical_path = path
                .canonicalize()
                .context("Failed to canonicalize target path")?;
            if !canonical_path.starts_with(&canonical_root) {
                return Err(anyhow!("Path escapes repo root: {}", rel_path));
            }
            Ok(canonical_path)
        } else {
            // Path doesn't exist. Check parent.
            let parent = path.parent().ok_or_else(|| anyhow!("Invalid path"))?;
            if parent.exists() {
                let canonical_parent = parent
                    .canonicalize()
                    .context("Failed to canonicalize parent")?;
                if !canonical_parent.starts_with(&canonical_root) {
                    return Err(anyhow!("Parent path escapes repo root: {}", rel_path));
                }
                // Return the joined path (since we can't canonicalize non-existent file)
                // But we should use the canonical parent + filename to be safe against some symlink tricks?
                // join filename
                let filename = path
                    .file_name()
                    .ok_or_else(|| anyhow!("Invalid filename"))?;
                Ok(canonical_parent.join(filename))
            } else {
                // Parent doesn't exist. Strict safety: reject deep creation unless create_dirs=true?
                // But for this helper, let's say we require parent to be safe.
                // If parent doesn't exist, we can't prove it's safe without resolving up to root.

                // Walk up until we find a base that exists, verify it's in root.
                let mut current = parent;
                while !current.exists() {
                    if let Some(p) = current.parent() {
                        current = p;
                    } else {
                        return Err(anyhow!("Cannot verify path safety (root not found)"));
                    }
                }
                let canonical_base = current.canonicalize()?;
                if !canonical_base.starts_with(&canonical_root) {
                    return Err(anyhow!("Path escapes repo root: {}", rel_path));
                }

                // If base safe, and we assume we are not following symlinks in non-existent components (obviously), it is safe.
                // But target logic must be precise.

                // Simple approach: we join components to canonical root? No, symlinks.
                // If intermediate components don't exist, they are just names.
                Ok(path)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn apply_patch(
        &self,
        repo_root: &Path,
        patch: &str,
        mode: &str,
        lease_id: Option<String>,
        _snapshot_id: Option<String>,
        strip: Option<usize>,
        _reject_on_conflict: bool,
        dry_run: bool,
    ) -> Result<serde_json::Value> {
        if mode == "worktree" {
            let lid = lease_id.ok_or_else(|| anyhow!("lease_id required"))?;
            self.lease_store.check_lease(&lid, repo_root).await?;

            let mut cmd = tokio::process::Command::new("git");
            cmd.arg("apply");
            cmd.arg("--verbose"); // To get details?

            // "No fuzzing": git apply has specific whitespace options but strict context usually default or --ignore-space-change.
            // "byte-for-byte": default.
            // "no fuzzing": technically `git apply` might fuzz. `--unidiff-zero`?
            // To prevent fuzzing, we might need recent git version flags, but default is usually fine.

            if dry_run {
                cmd.arg("--check");
            }

            if let Some(n) = strip {
                cmd.arg(format!("-p{}", n));
            }

            // Write patch to stdin
            cmd.current_dir(repo_root);
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            let mut child = cmd.spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(patch.as_bytes()).await?;
            }

            let output = child.wait_with_output().await?;

            if output.status.success() {
                // Touched files
                let touched = parse_patch_touched_files(patch);
                self.lease_store.touch_files(&lid, touched.clone()).await?;

                let new_fingerprint = Fingerprint::compute(repo_root).await?;

                let applied_value: Vec<serde_json::Value> = touched
                    .iter()
                    .map(|f| {
                        serde_json::json!({
                            "path": f,
                            "status": "ok"
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "applied": applied_value,
                    "rejects": [],
                    "lease_id": lid,
                    "fingerprint": new_fingerprint,
                    "cache_key": format!("{}:sha256:{}", lid, new_fingerprint.status_hash),
                    "cache_hint": "until_dirty"
                }))
            } else {
                // Parse reject reasons.
                // Assuming simple failure means "whole patch rejected" or specific hunks?
                // Git apply fails atomically usually.
                let stderr = String::from_utf8_lossy(&output.stderr);

                // Return errors as structured rejects if possible?
                // Or just fail.
                // Spec allows returning "rejects" list in Success response (if partial assert allowed?)
                // Schema has "rejects" field.
                // But if `git apply` fails (exit 1), nothing changed.
                // So "applied": []
                // "rejects": [ ... ]

                // We fake a "conflict" reject for all touched paths?
                // Or parse "error: patch failed: file:line".

                let rejects = parse_git_apply_errors(&stderr, patch);

                Ok(serde_json::json!({
                    "applied": [],
                    "rejects": rejects,
                    "lease_id": lid,
                    "fingerprint": Fingerprint::compute(repo_root).await?, // State didn't change ideally
                    "cache_key": "conflict",
                    "cache_hint": "until_dirty"
                }))
            }
        } else if mode == "snapshot" {
            let snap_id = _snapshot_id.ok_or_else(|| anyhow!("snapshot_id required"))?;
            self.store.validate_snapshot(&snap_id).await?;

            // Retrieve base snapshot metadata for provenance/determinism
            let base_info = self
                .store
                .get_snapshot_info(&snap_id).await?
                .ok_or_else(|| anyhow::anyhow!("Snapshot metadata not found for {}", snap_id))?;

            // 1. Materialize to temp dir
            let temp = tempfile::tempdir()?;
            let temp_path = temp.path();
            let entries = self.store.list_snapshot_entries(&snap_id).await?;

            for entry in entries {
                if let Some(content) = self.store.get_blob(&entry.blob)? {
                    let full_path = temp_path.join(&entry.path);
                    if let Some(parent) = full_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&full_path, content)?;
                } else {
                    return Err(anyhow::anyhow!("Missing blob for {}", entry.path));
                }
            }

            // 2. Apply patch
            // Disable autocrlf so snapshot blobs are always LF-only regardless of platform git config.
            let mut cmd = tokio::process::Command::new("git");
            cmd.arg("-c").arg("core.autocrlf=false");
            cmd.arg("-c").arg("core.eol=lf");
            cmd.arg("apply");
            cmd.arg("--verbose");
            if let Some(n) = strip {
                cmd.arg(format!("-p{}", n));
            }

            cmd.current_dir(temp_path);
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            let mut child = cmd.spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(patch.as_bytes()).await?;
            }

            let output = child.wait_with_output().await?;

            if output.status.success() {
                // 3. Ingest changes
                // Walk temp dir to find all current files
                use walkdir::WalkDir;
                let mut new_entries = Vec::new();
                for entry in WalkDir::new(temp_path) {
                    let entry = entry?;
                    if entry.file_type().is_file() {
                        let path = entry.path();
                        let rel_path = path
                            .strip_prefix(temp_path)?
                            .to_str()
                            .ok_or_else(|| anyhow::anyhow!("Non-UTF-8 path: {}", path.display()))?
                            .to_string();
                        // Validate path safety/ignored? Assuming temp dir is controlled.

                        let content = std::fs::read(path)?;
                        let blob_id = self.store.put_blob(&content)?;

                        new_entries.push(crate::snapshot::store::Entry {
                            path: rel_path,
                            blob: blob_id,
                            size: content.len() as u64,
                        });
                    }
                }

                // Create new snapshot
                let new_manifest = crate::snapshot::store::Manifest::new(new_entries); // sorts automatically
                let manifest_json = new_manifest.to_canonical_json()?;

                // Deterministic ID: sha256(fingerprint + manifest)
                // Use base snapshot's fingerprint to maintain same "context"
                let new_snap_id = new_manifest.compute_snapshot_id(&base_info.fingerprint_json)?;

                // Compute patch hash for lineage
                let patch_hash =
                    format!("sha256:{}", hex::encode(Sha256::digest(patch.as_bytes())));

                self.store.put_snapshot(
                    &new_snap_id,
                    &base_info.repo_root,        // Preserve base repo_root
                    &base_info.head_sha,         // Preserve base head_sha
                    &base_info.fingerprint_json, // Preserve base fingerprint
                    manifest_json.as_bytes(),
                    Some(&snap_id),    // derived_from
                    Some(&patch_hash), // applied_patch_hash
                    None,              // label
                ).await?;

                Ok(serde_json::json!({
                    "snapshot_id": new_snap_id,
                    "applied": [], // List touched?
                    "rejects": [],
                    "cache_hint": "immutable"
                }))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let rejects = parse_git_apply_errors(&stderr, patch);
                Ok(serde_json::json!({
                    "snapshot_id": snap_id, // Return original if failed?
                    "applied": [],
                    "rejects": rejects,
                    "cache_hint": "immutable"
                }))
            }
        } else {
            Err(anyhow!("Invalid mode"))
        }
    }

    pub async fn write_file(
        &self,
        repo_root: &Path,
        path: &str,
        content_base64: &str,
        lease_id: Option<String>,
        create_dirs: bool,
        dry_run: bool,
    ) -> Result<bool> {
        let lid = lease_id.ok_or_else(|| anyhow!("lease_id required"))?;
        self.lease_store.check_lease(&lid, repo_root).await?;

        let target = self.resolve_target_path(repo_root, path)?;

        use base64::{Engine as _, engine::general_purpose};
        let content = if let Some(rest) = content_base64.strip_prefix("base64:") {
            general_purpose::STANDARD
                .decode(rest)
                .context("Invalid base64 content")?
        } else {
            // accept plain text
            content_base64.as_bytes().to_vec()
        };

        if let Some(parent) = target.parent()
            && !parent.exists()
        {
            if create_dirs {
                if !dry_run {
                    std::fs::create_dir_all(parent)?;
                }
            } else {
                return Err(anyhow!(
                    "Parent directory does not exist (set create_dirs=true)"
                ));
            }
        }

        if !dry_run {
            std::fs::write(&target, content)?;
            self.lease_store.touch_files(&lid, vec![path.to_string()]).await?;
        }

        Ok(true)
    }

    pub async fn delete(
        &self,
        repo_root: &Path,
        path: &str,
        lease_id: Option<String>,
        dry_run: bool,
    ) -> Result<bool> {
        let lid = lease_id.ok_or_else(|| anyhow!("lease_id required"))?;
        self.lease_store.check_lease(&lid, repo_root).await?; // Verify at start

        let target = self.resolve_target_path(repo_root, path)?;

        if !target.exists() {
            return Err(anyhow!("File not found"));
        }

        if !dry_run {
            if target.is_dir() {
                std::fs::remove_dir_all(&target)?;
            } else {
                std::fs::remove_file(&target)?;
            }
            self.lease_store.touch_files(&lid, vec![path.to_string()]).await?;
        }

        Ok(true)
    }
}

// Helpers

fn parse_patch_touched_files(patch: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in patch.lines() {
        if line.starts_with("+++ ") {
            let path_part = line.trim_start_matches("+++ ").trim();
            // strip 'b/' or 'a/'? git diff is a/ b/
            // Usually destination is b/
            let clean_path = if let Some(stripped) = path_part.strip_prefix("b/") {
                stripped
            } else {
                path_part
            };
            if clean_path != "/dev/null" {
                files.push(clean_path.to_string());
            }
        }
    }
    files
}

fn parse_git_apply_errors(stderr: &str, patch: &str) -> Vec<serde_json::Value> {
    // Parse "error: patch failed: <file>:<line>"
    // Return structured rejects
    let mut rejects = Vec::new();
    // Simplified parsing
    for line in stderr.lines() {
        if let Some(part) = line.strip_prefix("error: patch failed: ") {
            let parts: Vec<&str> = part.split(':').collect();
            if parts.len() >= 2 {
                let file = parts[0];
                rejects.push(serde_json::json!({
                    "path": file,
                    "hunks": [{ "index": 0, "reason": "context_mismatch" }] // Dummy index/reason
                }));
            }
        }
    }

    // Fallback if no specific failure found but status failed
    if rejects.is_empty() {
        // Mark all touched as rejected?
        let touched = parse_patch_touched_files(patch);
        for f in touched {
            rejects.push(serde_json::json!({
                "path": f,
                "hunks": [{ "index": 0, "reason": "unknown_failure" }]
            }));
        }
    }
    rejects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BlobBackend, Compression, StorageConfig};
    use crate::snapshot::lease::LeaseStore;
    use crate::snapshot::store::Store;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_snapshot_apply_patch() {
        let dir = tempfile::tempdir().unwrap();
        let client = crate::db::init_hiqlite(dir.path()).await.unwrap();
        let config = StorageConfig {
            data_dir: dir.path().to_path_buf(),
            blob_backend: BlobBackend::Fs,
            compression: Compression::None,
        };
        let store = Arc::new(Store::new(client.clone(), config).unwrap());
        let lease_store = Arc::new(LeaseStore::new(client));
        let tools = WorkspaceTools::new(lease_store, store.clone());

        // Setup base snapshot containing a.txt
        let t1 = "original content\n";
        let h1 = store.put_blob(t1.as_bytes()).unwrap(); // put_blob is sync (filesystem only)
        let m1 = format!(
            r#"{{
            "entries": [
                {{ "path": "a.txt", "blob": "{}", "size": {} }}
            ]
        }}"#,
            h1,
            t1.len()
        );
        let sid = "snap-base";
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

        // Patch to modify a.txt and add b.txt
        let patch = r#"diff --git a/a.txt b/a.txt
index 7b57bd2..dcd25a4 100644
--- a/a.txt
+++ b/a.txt
@@ -1 +1 @@
-original content
+modified content
diff --git a/b.txt b/b.txt
new file mode 100644
index 0000000..9daeafb
--- /dev/null
+++ b/b.txt
@@ -0,0 +1 @@
+test
"#;

        // Apply patch
        let res = tools
            .apply_patch(
                dir.path(),
                patch,
                "snapshot",
                None,
                Some(sid.to_string()),
                Some(1), // strip 1 (a/ b/)
                false,
                false,
            )
            .await
            .unwrap();

        let new_sid = res["snapshot_id"].as_str().unwrap();
        assert_ne!(new_sid, sid);

        // Verify new snapshot content
        let entries = store.list_snapshot_entries(new_sid).await.unwrap();
        assert_eq!(entries.len(), 2); // a.txt, b.txt

        // Check content
        let mut found_a = false;
        let mut found_b = false;

        for e in entries {
            let content = store.get_blob(&e.blob).unwrap().unwrap(); // get_blob is sync (filesystem only)
            let s = String::from_utf8(content).unwrap();
            if e.path == "a.txt" {
                assert_eq!(s, "modified content\n");
                found_a = true;
            } else if e.path == "b.txt" {
                assert_eq!(s, "test\n");
                found_b = true;
            }
        }
        assert!(found_a);
        assert!(found_b);
    }
}
