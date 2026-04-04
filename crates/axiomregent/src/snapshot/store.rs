// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: MCP_SNAPSHOT_WORKSPACE
// Spec: spec/core/snapshot-workspace.md

use crate::config::{BlobBackend, Compression, StorageConfig};
use anyhow::{Result, anyhow};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
    conn: Arc<Mutex<Connection>>,
    blob_store: Box<dyn BlobStore>,
    config: StorageConfig,
}

impl Store {
    pub fn new(config: StorageConfig) -> Result<Self> {
        let db_path = config.data_dir.join("store.sqlite");
        if let Some(p) = db_path.parent() {
            fs::create_dir_all(p)?;
        }

        let conn = Connection::open(&db_path)?;
        Self::migrate(&conn)?;

        let blob_store: Box<dyn BlobStore> = match config.blob_backend {
            BlobBackend::Fs => {
                let blobs_dir = config.data_dir.join("blobs");
                Box::new(FsBlobStore::new(blobs_dir)?)
            }
            BlobBackend::Db => {
                return Err(anyhow!(
                    "SqliteBlobStore backend is not yet implemented; use the default Fs backend (BlobBackend::Fs)"
                ));
            }
        };

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            blob_store,
            config,
        })
    }

    fn migrate(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS blobs (
                hash TEXT PRIMARY KEY,
                size_bytes INTEGER NOT NULL,
                compression TEXT NOT NULL,
                storage TEXT NOT NULL,
                refcount INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS snapshots (
                snapshot_id TEXT PRIMARY KEY,
                repo_root TEXT NOT NULL,
                head_sha TEXT NOT NULL,
                fingerprint_json TEXT NOT NULL,
                manifest_hash TEXT NOT NULL,
                manifest_bytes BLOB,
                created_at INTEGER,
                derived_from TEXT,
                applied_patch_hash TEXT,
                label TEXT
            );
            
            CREATE TABLE IF NOT EXISTS manifest_entries (
                snapshot_id TEXT NOT NULL,
                path TEXT NOT NULL,
                blob_hash TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                PRIMARY KEY (snapshot_id, path)
            );

            CREATE TABLE IF NOT EXISTS leases (
                lease_id TEXT PRIMARY KEY,
                repo_root TEXT NOT NULL,
                fingerprint_json TEXT NOT NULL,
                touched_json TEXT NOT NULL,
                issued_at INTEGER
            );
            "#,
        )?;
        Ok(())
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
        // Wait, spec says "hash of raw bytes" usually?
        // "content-addressed" implies hash is of content.
        // If we store compressed, we usually want hash of *original* content to look it up?
        // But FsBlobStore writes to `path_for(hash)`.
        // If `hash` is sha256(compressed), then we have a CompressedBlobStore.
        // But if we want `get(original_hash)` to work, we have a problem if we only store compressed.

        // Correct approach for opaque blob store:
        // 1. Compute hash of ORIGINAL data (ID).
        // 2. We want to store `compressed` data but indexed by `original_hash`?
        // FsBlobStore::put computes hash internally.

        // If FsBlobStore computes hash of what it gets, then we get hash(compressed).
        // That breaks "content-addressed" if we expect hash(original).

        // Solution:
        // We need explicit `put_with_hash` or `put` returns the hash of what was stored.
        // If we use hash(compressed), then the Manifest must store hash(compressed).
        // But `Entry` usually stores hash of content?
        // The user said: "get() must decompress based on SQLite metadata".
        // If I request `get(H)`, and H is hash(original), and DB says "H is zstd",
        // then `FsBlobStore` must have stored it under H?
        // But `FsBlobStore::put` derives path from data.

        // If `FsBlobStore` enforces "path = hash(content)", then storing compressed data under `hash(original)` violates that if checked.
        // But `FsBlobStore` *implementation* currently does: `let digest = Sha256::digest(data); ... path_for(hash)`.
        // So it stores under `hash(compressed)`.

        // This means if we compress, the "blob hash" returned is the hash of the compressed bytes.
        // The Manifest will contain `hash(compressed)`.
        // When we read, we get `hash(compressed)`, we read bytes, we see in DB that `hash(compressed)` is zstd, we decompress.
        // The decompressed data has `hash(original)`.
        // This is fine! The "blob pointer" in manifest points to the physical blob.

        let hash = self
            .blob_store
            .put(&stored_data, self.config.compression.clone())?;

        // Update metadata
        let conn = self.conn.lock().unwrap();
        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        conn.execute(
            "INSERT OR IGNORE INTO blobs (hash, size_bytes, compression, storage, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                hash,
                stored_data.len() as i64,
                alg,
                match self.config.blob_backend { BlobBackend::Fs => "fs", BlobBackend::Db => "db" },
                now_epoch
            ]
        )?;

        Ok(hash)
    }

    // Snapshot Metadata & Manifest
    // Replaces the legacy put_snapshot with a full version
    #[allow(clippy::too_many_arguments)]
    pub fn put_snapshot(
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

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // 1. Check for overwrite and decrement refcounts
        // (If we were smarter we would check diff and only decrement unique removed blobs,
        // but for now strict "remove old usage, add new usage" is safe and simple)
        let exists: Option<String> = tx
            .query_row(
                "SELECT snapshot_id FROM snapshots WHERE snapshot_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;

        if exists.is_some() {
            // Decrement contents of old snapshot
            let mut stmt =
                tx.prepare("SELECT blob_hash FROM manifest_entries WHERE snapshot_id = ?1")?;
            let blobs = stmt.query_map(params![id], |row| row.get::<_, String>(0))?;

            // Gather to avoid borrow issues while executing updates
            let blob_hashes: Vec<String> = blobs.collect::<Result<_, _>>()?;
            drop(stmt);

            for hash in blob_hashes {
                tx.execute(
                    "UPDATE blobs SET refcount = MAX(0, refcount - 1) WHERE hash = ?1",
                    params![hash],
                )?;
            }
        }

        // 2. Insert/Replace snapshot
        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        tx.execute(
            "INSERT OR REPLACE INTO snapshots (snapshot_id, repo_root, head_sha, fingerprint_json, manifest_hash, manifest_bytes, created_at, derived_from, applied_patch_hash, label) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id,
                repo_root,
                head_sha,
                fingerprint_json,
                manifest_hash,
                manifest_bytes,
                now_epoch,
                derived_from,
                applied_patch_hash,
                label
            ]
        )?;

        // 3. Clear old entries
        tx.execute(
            "DELETE FROM manifest_entries WHERE snapshot_id = ?1",
            params![id],
        )?;

        // 4. Insert new entries
        let mut stmt = tx.prepare("INSERT INTO manifest_entries (snapshot_id, path, blob_hash, size_bytes) VALUES (?1, ?2, ?3, ?4)")?;
        for entry in &manifest.entries {
            stmt.execute(params![id, entry.path, entry.blob, entry.size])?;

            // Refcount increment
            let row_count = tx.execute(
                "UPDATE blobs SET refcount = refcount + 1 WHERE hash = ?1",
                params![entry.blob],
            )?;
            if row_count == 0 {
                // If blob is missing in DB (e.g. corruption or out of sync), we should probably fail?
                // Or implicitly trust it exists in FS?
                // Robustness: Fail to ensure we don't have dangling references in manifest.
                return Err(anyhow!("Referenced blob not found in DB: {}", entry.blob));
            }
        }
        drop(stmt);

        tx.commit()?;

        Ok(())
    }

    pub fn get_snapshot(&self, id: &str) -> Result<Option<Vec<u8>>> {
        let conn = self.conn.lock().unwrap();
        // Try to get manifest_bytes from snapshots table
        let mut stmt =
            conn.prepare("SELECT manifest_bytes FROM snapshots WHERE snapshot_id = ?1")?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            let bytes: Vec<u8> = row.get(0)?;
            return Ok(Some(bytes));
        }

        Ok(None)
    }

    pub fn get_snapshot_info(&self, id: &str) -> Result<Option<SnapshotInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT snapshot_id, repo_root, head_sha, fingerprint_json, manifest_hash, created_at, derived_from, applied_patch_hash, label FROM snapshots WHERE snapshot_id = ?1")?;
        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(SnapshotInfo {
                snapshot_id: row.get(0)?,
                repo_root: row.get(1)?,
                head_sha: row.get(2)?,
                fingerprint_json: row.get(3)?,
                manifest_hash: row.get(4)?,
                created_at: row.get(5)?,
                derived_from: row.get(6)?,
                applied_patch_hash: row.get(7)?,
                label: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    // List entries from DB (faster than parsing manifest JSON)
    pub fn list_snapshot_entries(&self, id: &str) -> Result<Vec<Entry>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT path, blob_hash, size_bytes FROM manifest_entries WHERE snapshot_id = ?1 ORDER BY path ASC")?;
        let rows = stmt.query_map(params![id], |row| {
            Ok(Entry {
                path: row.get(0)?,
                blob: row.get(1)?,
                size: row.get(2)?,
            })
        })?;

        let mut entries = Vec::new();
        for r in rows {
            entries.push(r?);
        }

        // If empty, maybe we have the snapshot but not entries? (e.g. legacy or not populated)
        // Fallback to parsing manifest_bytes?
        if entries.is_empty()
            && let Some(bytes) = self.get_snapshot(id)?
        {
            let manifest: Manifest = serde_json::from_slice(&bytes)?;
            return Ok(manifest.entries);
        }

        Ok(entries)
    }

    pub fn get_blob(&self, hash: &str) -> Result<Option<Vec<u8>>> {
        // Read bytes from backend
        let maybe_bytes = self.blob_store.get(hash)?;
        if let Some(mut bytes) = maybe_bytes {
            // Check compression in DB
            let conn = self.conn.lock().unwrap();
            let compression: Option<String> = conn
                .query_row(
                    "SELECT compression FROM blobs WHERE hash = ?1",
                    params![hash],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(s) = compression
                && s == "zstd"
            {
                // Decompress
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

    pub fn validate_snapshot(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // 1. Check snapshot existence
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM snapshots WHERE snapshot_id = ?1",
                params![id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !exists {
            return Err(anyhow!("Snapshot not found: {}", id));
        }

        // 2. Load manifest entries
        // We use list_snapshot_entries logic but inline to avoid lock issues if we were holding it?
        // Actually list_snapshot_entries takes a lock. We currently hold a lock `conn`.
        // We should drop lock before calling other methods if they take lock.
        drop(conn);

        let entries = self.list_snapshot_entries(id)?;

        // 3. Verify entries are sorted and unique
        // list_snapshot_entries sorts by path. We just need to check unicity?
        // SQLite PK is (snapshot_id, path), so uniqueness of path is enforced by DB.
        // We just check that paths are valid?
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

        // 4. Verify blobs exist
        // We can do a batched check or one by one.
        // For now, one by one check "has".
        for entry in &entries {
            // Check DB
            let conn = self.conn.lock().unwrap();
            let blob_exists: bool = conn
                .query_row(
                    "SELECT 1 FROM blobs WHERE hash = ?1",
                    params![entry.blob],
                    |_| Ok(true),
                )
                .optional()?
                .unwrap_or(false);
            drop(conn);

            if !blob_exists {
                return Err(anyhow!(
                    "Snapshot corrupt: missing blob DB entry for {}",
                    entry.blob
                ));
            }

            // Check backend
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

    #[test]
    fn test_validate_snapshot_missing_blob() {
        let dir = tempfile::tempdir().unwrap();
        let config = StorageConfig {
            data_dir: dir.path().to_path_buf(),
            blob_backend: BlobBackend::Fs,
            compression: Compression::None,
        };
        let store = Store::new(config).unwrap();

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
            .unwrap();

        // Valid
        assert!(store.validate_snapshot(sid).is_ok());

        // Corrupt backend: delete blob file
        let blob_path = dir
            .path()
            .join("blobs")
            .join(blob_hash.split(':').next().unwrap())
            .join(&blob_hash.split(':').nth(1).unwrap()[..2])
            .join(blob_hash.split(':').nth(1).unwrap());
        if blob_path.exists() {
            std::fs::remove_file(blob_path).unwrap();
        } else {
            // It might be nested differently, let's use internal knowledge or just iterate
            // FsBlobStore::path_for logic: algo/prefix/hash
            let parts: Vec<&str> = blob_hash.split(':').collect();
            let path = dir
                .path()
                .join("blobs")
                .join(parts[0])
                .join(&parts[1][0..2])
                .join(parts[1]);
            std::fs::remove_file(path).unwrap();
        }

        // Should fail
        assert!(store.validate_snapshot(sid).is_err());
    }
}
