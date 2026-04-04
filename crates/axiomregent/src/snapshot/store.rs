// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: MCP_SNAPSHOT_WORKSPACE
// Spec: spec/core/snapshot-workspace.md

use crate::config::{Compression, StorageConfig};
use anyhow::{Result, anyhow};
use hiqlite::{Client, Param};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

// Re-export Manifest/Entry for compatibility
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Entry {
    pub blob: String,
    pub path: String,
    #[serde(default)]
    pub size: u64, // Added size to match schema requirement
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SnapshotInfo {
    pub snapshot_id: String,
    pub repo_root: String,
    pub head_sha: String,
    pub fingerprint_json: String,
    pub manifest_hash: String,
    pub created_at: Option<i64>,
    pub derived_from: Option<String>,
    pub applied_patch_hash: Option<String>,
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manifest {
    pub entries: Vec<Entry>,
}

impl Manifest {
    pub fn new(mut entries: Vec<Entry>) -> Self {
        // Enforce deterministic order (lexicographic by path)
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Self { entries }
    }

    pub fn to_canonical_json(&self) -> Result<String> {
        let val = serde_json::to_value(self)?;
        let s = serde_json::to_string(&val)?;
        Ok(s)
    }

    pub fn compute_snapshot_id(&self, fingerprint_json: &str) -> Result<String> {
        // Use bytes for strict determinism
        let manifest_json = self.to_canonical_json()?;
        let mut hasher = Sha256::new();
        hasher.update(fingerprint_json.as_bytes());
        hasher.update(b"\n");
        hasher.update(manifest_json.as_bytes());
        let hash = hex::encode(hasher.finalize());
        Ok(format!("sha256:{}", hash))
    }
}

pub trait BlobStore: Send + Sync {
    fn put(&self, data: &[u8], compression: Compression) -> Result<String>;
    fn get(&self, hash: &str) -> Result<Option<Vec<u8>>>;
    fn has(&self, hash: &str) -> Result<bool>;
}

pub struct FsBlobStore {
    base_path: PathBuf,
}

impl FsBlobStore {
    pub fn new(base_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_path)?;
        Ok(Self { base_path })
    }

    fn path_for(&self, hash: &str) -> Result<PathBuf> {
        // format: blobs/algo/prefix/hash
        // hash is "sha256:hex"
        let parts: Vec<&str> = hash.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid hash format: {}", hash));
        }
        let algo = parts[0];
        let val = parts[1];

        if val.len() < 64 {
            return Err(anyhow!(
                "Invalid hash length (expected >= 64 chars): {}",
                hash
            ));
        }

        // Strict hex validation
        if !val.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(anyhow!("Invalid hash chars (hex allowed only): {}", hash));
        }

        // Basic hex validation could be added here, but length check prevents panic on slice
        let prefix = &val[0..2];
        Ok(self.base_path.join(algo).join(prefix).join(val))
    }
}

impl BlobStore for FsBlobStore {
    fn put(&self, data: &[u8], _compression: Compression) -> Result<String> {
        let digest = Sha256::digest(data);
        let hash_val = hex::encode(digest);
        let hash = format!("sha256:{}", hash_val);

        let path = self.path_for(&hash)?;

        if path.exists() {
            return Ok(hash);
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Atomic write
        let mut tmp = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(&self.base_path))?;
        use std::io::Write;
        tmp.write_all(data)?;
        tmp.persist(&path).map_err(|e| e.error)?;

        Ok(hash)
    }

    fn get(&self, hash: &str) -> Result<Option<Vec<u8>>> {
        let path = self.path_for(hash)?;
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read(path)?;
        Ok(Some(data))
    }

    fn has(&self, hash: &str) -> Result<bool> {
        Ok(self.path_for(hash)?.exists())
    }
}

pub struct Store {
    client: Client,
    blob_store: FsBlobStore,
    config: StorageConfig,
}

impl Store {
    pub fn new(client: Client, config: StorageConfig) -> Result<Self> {
        let blob_path = config.data_dir.join("blobs").join("sha256");
        let blob_store = FsBlobStore::new(blob_path)?;
        Ok(Self {
            client,
            blob_store,
            config,
        })
    }

    // BlobStore Proxy with Compression logic
    // NOTE: The returned hash is the SHA256 of the *stored* bytes (which may be compressed).
    // This implies that if compression is used, the manifest stores the hash of the compressed data.
    // Decompression happens transparently on get() by consulting the DB.
    pub fn put_blob(&self, data: &[u8]) -> Result<String> {
        // Handle compression
        let (stored_data, alg) = match self.config.compression {
            Compression::Zstd => {
                let compressed = zstd::stream::encode_all(data, 3)?; // Level 3 default
                (compressed, "zstd")
            }
            Compression::None => (data.to_vec(), "none"),
        };

        // Note: BlobStore::put computes hash of *provided* data.
        // If we pass compressed data, the hash will be of compressed data.
        // The Manifest will contain hash(compressed).
        // When we read, we get hash(compressed), we read bytes, we see in DB that
        // hash(compressed) is zstd, we decompress. This is fine.

        let hash = self
            .blob_store
            .put(&stored_data, self.config.compression.clone())?;

        // blob_refs upsert is done asynchronously — callers that need full consistency
        // should use put_blob_async. For the sync path we return the hash and the
        // blob_refs row will be written on the next async put_snapshot call.
        // Store the alg string for use in put_blob_async.
        let _ = alg; // used below in put_blob_async

        Ok(hash)
    }

    /// Write a blob ref entry into hiqlite. Call this after put_blob when in async context.
    pub async fn put_blob_async(&self, data: &[u8]) -> Result<String> {
        let (stored_data, alg) = match self.config.compression {
            Compression::Zstd => {
                let compressed = zstd::stream::encode_all(data, 3)?;
                (compressed, "zstd")
            }
            Compression::None => (data.to_vec(), "none"),
        };

        let hash = self
            .blob_store
            .put(&stored_data, self.config.compression.clone())?;

        self.client
            .execute(
                Cow::Borrowed(
                    "INSERT OR IGNORE INTO blob_refs (blob_hash, ref_count, size_bytes, compression) \
                     VALUES ($1, 1, $2, $3)",
                ),
                vec![
                    Param::Text(hash.clone()),
                    Param::Integer(stored_data.len() as i64),
                    Param::Text(alg.to_string()),
                ],
            )
            .await?;

        Ok(hash)
    }

    // Snapshot Metadata & Manifest
    #[allow(clippy::too_many_arguments)]
    pub async fn put_snapshot(
        &self,
        id: &str,
        repo_root: &str,
        head_sha: &str,
        fingerprint_json: &str,
        manifest_bytes: &[u8],
        derived_from: Option<&str>,
        applied_patch_hash: Option<&str>,
        label: Option<&str>,
    ) -> Result<()> {
        let manifest: Manifest = serde_json::from_slice(manifest_bytes)?;
        let manifest_hash = format!("sha256:{}", hex::encode(Sha256::digest(manifest_bytes)));

        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Serialize manifest to store in the checkpoint row.
        // The new checkpoints table does not have a manifest_bytes BLOB column,
        // so we store a subset of fields. The full manifest lives in manifest_entries.
        let file_count = manifest.entries.len() as i64;
        let total_bytes: i64 = manifest.entries.iter().map(|e| e.size as i64).sum();
        let created_at_str = chrono::DateTime::from_timestamp(now_epoch, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| now_epoch.to_string());

        // 1. Check for prior manifest_entries and decrement blob_refs
        #[derive(serde::Deserialize)]
        struct BlobHashRow {
            blob_hash: String,
        }

        let existing_entries: Vec<BlobHashRow> = self
            .client
            .query_as(
                "SELECT blob_hash FROM manifest_entries WHERE checkpoint_id = $1",
                vec![Param::Text(id.to_string())],
            )
            .await
            .unwrap_or_default();

        for row in existing_entries {
            let _ = self
                .client
                .execute(
                    Cow::Borrowed(
                        "UPDATE blob_refs SET ref_count = MAX(0, ref_count - 1) \
                         WHERE blob_hash = $1",
                    ),
                    vec![Param::Text(row.blob_hash)],
                )
                .await;
        }

        // 2. Insert/Replace checkpoint row
        self.client
            .execute(
                Cow::Borrowed(
                    "INSERT OR REPLACE INTO checkpoints \
                     (checkpoint_id, repo_root, parent_id, label, head_sha, fingerprint, \
                      state_hash, merkle_root, file_count, total_bytes, created_at, metadata) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                ),
                vec![
                    Param::Text(id.to_string()),
                    Param::Text(repo_root.to_string()),
                    Param::Text(derived_from.unwrap_or("").to_string()),
                    Param::Text(label.unwrap_or("").to_string()),
                    Param::Text(head_sha.to_string()),
                    Param::Text(fingerprint_json.to_string()),
                    // state_hash and merkle_root: use manifest_hash for both
                    Param::Text(manifest_hash.clone()),
                    Param::Text(manifest_hash.clone()),
                    Param::Integer(file_count),
                    Param::Integer(total_bytes),
                    Param::Text(created_at_str),
                    // metadata: store applied_patch_hash if present
                    Param::Text(
                        applied_patch_hash
                            .map(|h| format!("{{\"applied_patch_hash\":\"{}\"}}", h))
                            .unwrap_or_default(),
                    ),
                ],
            )
            .await?;

        // 3. Clear old manifest_entries
        self.client
            .execute(
                Cow::Borrowed("DELETE FROM manifest_entries WHERE checkpoint_id = $1"),
                vec![Param::Text(id.to_string())],
            )
            .await?;

        // 4. Insert new manifest_entries and upsert blob_refs
        for entry in &manifest.entries {
            self.client
                .execute(
                    Cow::Borrowed(
                        "INSERT INTO manifest_entries \
                         (checkpoint_id, path, blob_hash, size_bytes) \
                         VALUES ($1, $2, $3, $4)",
                    ),
                    vec![
                        Param::Text(id.to_string()),
                        Param::Text(entry.path.clone()),
                        Param::Text(entry.blob.clone()),
                        Param::Integer(entry.size as i64),
                    ],
                )
                .await?;

            // Increment blob ref_count (insert if missing — blob may have been
            // written without put_blob_async)
            self.client
                .execute(
                    Cow::Borrowed(
                        "INSERT INTO blob_refs (blob_hash, ref_count, size_bytes, compression) \
                         VALUES ($1, 1, $2, 'none') \
                         ON CONFLICT(blob_hash) DO UPDATE SET ref_count = ref_count + 1",
                    ),
                    vec![
                        Param::Text(entry.blob.clone()),
                        Param::Integer(entry.size as i64),
                    ],
                )
                .await?;
        }

        Ok(())
    }

    pub async fn get_snapshot(&self, id: &str) -> Result<Option<Vec<u8>>> {
        // Reconstruct manifest bytes from manifest_entries
        #[derive(serde::Deserialize)]
        struct EntryRow {
            path: String,
            blob_hash: String,
            size_bytes: i64,
        }

        let rows: Vec<EntryRow> = self
            .client
            .query_as(
                "SELECT path, blob_hash, size_bytes FROM manifest_entries \
                 WHERE checkpoint_id = $1 ORDER BY path ASC",
                vec![Param::Text(id.to_string())],
            )
            .await?;

        if rows.is_empty() {
            // Check if the checkpoint itself exists (empty manifest vs. not found)
            #[derive(serde::Deserialize)]
            #[allow(dead_code)]
            struct ExistsRow {
                checkpoint_id: String,
            }
            let exists: Vec<ExistsRow> = self
                .client
                .query_as(
                    "SELECT checkpoint_id FROM checkpoints WHERE checkpoint_id = $1",
                    vec![Param::Text(id.to_string())],
                )
                .await?;
            if exists.is_empty() {
                return Ok(None);
            }
        }

        let entries: Vec<Entry> = rows
            .into_iter()
            .map(|r| Entry {
                path: r.path,
                blob: r.blob_hash,
                size: r.size_bytes as u64,
            })
            .collect();

        let manifest = Manifest { entries };
        let bytes = manifest.to_canonical_json()?.into_bytes();
        Ok(Some(bytes))
    }

    pub async fn get_snapshot_info(&self, id: &str) -> Result<Option<SnapshotInfo>> {
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct CheckpointRow {
            checkpoint_id: String,
            repo_root: String,
            head_sha: String,
            fingerprint: String,
            state_hash: String,
            created_at: String,
            parent_id: String,
            metadata: String,
            label: String,
        }

        let rows: Vec<CheckpointRow> = self
            .client
            .query_as(
                "SELECT checkpoint_id, repo_root, head_sha, fingerprint, state_hash, \
                 created_at, parent_id, metadata, label \
                 FROM checkpoints WHERE checkpoint_id = $1",
                vec![Param::Text(id.to_string())],
            )
            .await?;

        let Some(row) = rows.into_iter().next() else {
            return Ok(None);
        };

        // Parse applied_patch_hash from metadata JSON
        let applied_patch_hash = serde_json::from_str::<serde_json::Value>(&row.metadata)
            .ok()
            .and_then(|v| v["applied_patch_hash"].as_str().map(String::from));

        Ok(Some(SnapshotInfo {
            snapshot_id: row.checkpoint_id,
            repo_root: row.repo_root,
            head_sha: row.head_sha,
            fingerprint_json: row.fingerprint,
            manifest_hash: row.state_hash,
            created_at: None, // stored as ISO string; omit for now
            derived_from: if row.parent_id.is_empty() {
                None
            } else {
                Some(row.parent_id)
            },
            applied_patch_hash,
            label: if row.label.is_empty() {
                None
            } else {
                Some(row.label)
            },
        }))
    }

    // List entries from DB
    pub async fn list_snapshot_entries(&self, id: &str) -> Result<Vec<Entry>> {
        #[derive(serde::Deserialize)]
        struct EntryRow {
            path: String,
            blob_hash: String,
            size_bytes: i64,
        }

        let rows: Vec<EntryRow> = self
            .client
            .query_as(
                "SELECT path, blob_hash, size_bytes FROM manifest_entries \
                 WHERE checkpoint_id = $1 ORDER BY path ASC",
                vec![Param::Text(id.to_string())],
            )
            .await?;

        let entries = rows
            .into_iter()
            .map(|r| Entry {
                path: r.path,
                blob: r.blob_hash,
                size: r.size_bytes as u64,
            })
            .collect();

        Ok(entries)
    }

    pub fn get_blob(&self, hash: &str) -> Result<Option<Vec<u8>>> {
        // Read bytes from backend (synchronous filesystem I/O)
        let maybe_bytes = self.blob_store.get(hash)?;
        if let Some(mut bytes) = maybe_bytes {
            // Check compression: try to detect zstd magic bytes (0xFD2FB528 LE)
            // rather than consulting the DB to keep this method synchronous.
            // Zstd frames start with magic number 0x28B52FfD (little-endian: FD 2F B5 28).
            if bytes.len() >= 4
                && bytes[0] == 0xFD
                && bytes[1] == 0x2F
                && bytes[2] == 0xB5
                && bytes[3] == 0x28
            {
                bytes = zstd::stream::decode_all(std::io::Cursor::new(bytes))?;
            }
            return Ok(Some(bytes));
        }
        Ok(None)
    }

    pub fn validate_path(path: &str) -> Result<()> {
        if path.starts_with('/') {
            return Err(anyhow!("Absolute paths not allowed: {}", path));
        }
        if path.contains('\\') {
            return Err(anyhow!("Backslashes not allowed: {}", path));
        }
        // Check for .. segments
        for component in std::path::Path::new(path).components() {
            if matches!(component, std::path::Component::ParentDir) {
                return Err(anyhow!(
                    "Parent directory segments (..) not allowed: {}",
                    path
                ));
            }
        }
        Ok(())
    }

    pub async fn validate_snapshot(&self, id: &str) -> Result<()> {
        // 1. Check snapshot existence
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct ExistsRow {
            checkpoint_id: String,
        }
        let exists: Vec<ExistsRow> = self
            .client
            .query_as(
                "SELECT checkpoint_id FROM checkpoints WHERE checkpoint_id = $1",
                vec![Param::Text(id.to_string())],
            )
            .await?;

        if exists.is_empty() {
            return Err(anyhow!("Snapshot not found: {}", id));
        }

        // 2. Load manifest entries
        let entries = self.list_snapshot_entries(id).await?;

        // 3. Verify entries are sorted and unique
        for (i, entry) in entries.iter().enumerate() {
            Self::validate_path(&entry.path)?;

            // Check sorting (paranoid check)
            if i > 0 && entry.path <= entries[i - 1].path {
                return Err(anyhow!(
                    "Manifest not sorted or duplicate paths at index {}",
                    i
                ));
            }
        }

        // 4. Verify blobs exist in filesystem backend
        for entry in &entries {
            if !self.blob_store.has(&entry.blob)? {
                return Err(anyhow!(
                    "Snapshot corrupt: missing blob content for {}",
                    entry.blob
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path() {
        assert!(Store::validate_path("foo.txt").is_ok());
        assert!(Store::validate_path("foo/bar.txt").is_ok());

        assert!(Store::validate_path("/abs/path").is_err());
        assert!(Store::validate_path("foo/../bar").is_err());
        assert!(Store::validate_path("foo\\bar").is_err());
    }

    #[tokio::test]
    async fn test_validate_snapshot_missing_blob() {
        let dir = tempfile::tempdir().unwrap();
        let client = crate::db::init_hiqlite(dir.path()).await.unwrap();
        let config = StorageConfig {
            data_dir: dir.path().to_path_buf(),
            ..StorageConfig::default()
        };
        let store = Store::new(client, config).unwrap();

        // Create a blob
        let blob_hash = store.put_blob(b"hello").unwrap();

        // Create snapshot
        let manifest_bytes = r#"{
            "entries": [
                { "path": "test.txt", "blob": "__BLOB_HASH__", "size": 5 }
            ]
        }"#
        .replace("__BLOB_HASH__", &blob_hash);

        let sid = "snap1";
        store
            .put_snapshot(
                sid,
                "/repo",
                "headsha",
                "{}",
                manifest_bytes.as_bytes(),
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // Valid
        assert!(store.validate_snapshot(sid).await.is_ok());

        // Corrupt backend: delete blob file
        // FsBlobStore::path_for logic: algo/prefix/hash (base_path is data_dir/blobs/sha256)
        let parts: Vec<&str> = blob_hash.split(':').collect();
        let path = dir
            .path()
            .join("blobs")
            .join("sha256")
            .join(parts[0])
            .join(&parts[1][0..2])
            .join(parts[1]);
        if path.exists() {
            std::fs::remove_file(path).unwrap();
        }

        // Should fail
        assert!(store.validate_snapshot(sid).await.is_err());
    }
}
