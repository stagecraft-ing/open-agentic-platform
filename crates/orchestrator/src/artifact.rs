// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/044-multi-agent-orchestration/spec.md, specs/094-unified-artifact-store/spec.md

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Default artifact root when `OAP_ARTIFACT_DIR` is unset (044 FR-003).
pub const DEFAULT_ARTIFACT_DIR: &str = "/tmp/oap-artifacts";

/// Resolves `$OAP_ARTIFACT_DIR/<run_id>/<step_id>/...` paths (044).
#[derive(Clone, Debug)]
pub struct ArtifactManager {
    base_dir: PathBuf,
}

impl ArtifactManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Reads `OAP_ARTIFACT_DIR` or returns [`DEFAULT_ARTIFACT_DIR`].
    pub fn from_env() -> Self {
        let base = std::env::var("OAP_ARTIFACT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_ARTIFACT_DIR));
        Self::new(base)
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn run_dir(&self, run_id: Uuid) -> PathBuf {
        self.base_dir.join(run_id.to_string())
    }

    pub fn step_dir(&self, run_id: Uuid, step_id: &str) -> PathBuf {
        self.run_dir(run_id).join(step_id)
    }

    /// Absolute path for an output artifact file name within a step (044).
    pub fn output_artifact_path(&self, run_id: Uuid, step_id: &str, filename: &str) -> PathBuf {
        self.step_dir(run_id, step_id).join(filename)
    }

    /// Deletes `<base>/<run_id>/` recursively (044 `cleanup_artifacts`).
    pub fn cleanup_run(&self, run_id: Uuid) -> std::io::Result<()> {
        let dir = self.run_dir(run_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    /// Ensures the step directory exists.
    pub fn ensure_step_dir(&self, run_id: Uuid, step_id: &str) -> std::io::Result<PathBuf> {
        let p = self.step_dir(run_id, step_id);
        std::fs::create_dir_all(&p)?;
        Ok(p)
    }

    /// SHA-256 hash of the file at `path`, hex-encoded (64 chars).
    ///
    /// Spec 082 FR-001.
    pub fn hash_artifact(path: &Path) -> std::io::Result<String> {
        let mut hasher = Sha256::new();
        let mut file = std::fs::File::open(path)?;
        std::io::copy(&mut file, &mut hasher)?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Returns `true` if the file at `path` hashes to `expected_hash`.
    ///
    /// Spec 082 FR-001.
    pub fn verify_artifact(path: &Path, expected_hash: &str) -> std::io::Result<bool> {
        let actual = Self::hash_artifact(path)?;
        Ok(actual == expected_hash)
    }

    /// Promote completed step artifacts to the content-addressed store (094 Slice 2).
    ///
    /// For each output file, copies it into the CAS with deduplication.
    /// Returns metadata for each promoted artifact.
    pub fn promote_to_cas(
        &self,
        run_id: Uuid,
        step_id: &str,
        output_names: &[String],
        cas: &ContentAddressedStore,
    ) -> std::io::Result<Vec<CasArtifact>> {
        let mut promoted = Vec::new();
        for name in output_names {
            let source = self.output_artifact_path(run_id, step_id, name);
            if source.exists() {
                let artifact = cas.store(&source)?;
                promoted.push(artifact);
            }
        }
        Ok(promoted)
    }
}

// ---------------------------------------------------------------------------
// Content-addressed store (094 Slice 2)
// ---------------------------------------------------------------------------

/// Default CAS root when `OAP_ARTIFACT_STORE` is unset.
pub const DEFAULT_CAS_DIR: &str = ".oap/artifact-store";

/// Metadata about an artifact stored in the CAS.
#[derive(Debug, Clone)]
pub struct CasArtifact {
    pub content_hash: String,
    pub storage_path: PathBuf,
    pub filename: String,
    pub size_bytes: u64,
}

/// Content-addressed store with two-char prefix sharding (094).
///
/// Layout: `<base>/<hash[0..2]>/<hash>/<filename>`
///
/// Identical to `factory_engine::LocalArtifactStore` but owned by the orchestrator
/// to avoid circular crate dependencies.
#[derive(Clone, Debug)]
pub struct ContentAddressedStore {
    base_dir: PathBuf,
}

impl ContentAddressedStore {
    pub fn new(base_dir: impl Into<PathBuf>) -> std::io::Result<Self> {
        let base_dir = base_dir.into();
        std::fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    /// Create from `OAP_ARTIFACT_STORE` env var, falling back to `~/.oap/artifact-store`.
    pub fn from_env() -> std::io::Result<Self> {
        let base = std::env::var("OAP_ARTIFACT_STORE")
            .ok()
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(DEFAULT_CAS_DIR)
            });
        Self::new(base)
    }

    /// Store an artifact by content hash with deduplication.
    pub fn store(&self, source_path: &Path) -> std::io::Result<CasArtifact> {
        let content_hash = ArtifactManager::hash_artifact(source_path)?;
        let size_bytes = std::fs::metadata(source_path)?.len();
        let filename = source_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "artifact".into());

        let target = self.artifact_path(&content_hash, &filename)?;

        if !target.exists() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let tmp = target.with_extension("tmp");
            std::fs::copy(source_path, &tmp)?;
            std::fs::rename(&tmp, &target)?;
        }

        Ok(CasArtifact {
            content_hash,
            storage_path: target,
            filename,
            size_bytes,
        })
    }

    /// Check whether an artifact with this hash exists.
    pub fn exists(&self, content_hash: &str) -> bool {
        if !content_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
        let prefix = &content_hash[..2.min(content_hash.len())];
        self.base_dir.join(prefix).join(content_hash).exists()
    }

    /// Retrieve a stored artifact to the target path.
    pub fn retrieve(
        &self,
        content_hash: &str,
        filename: &str,
        target_path: &Path,
    ) -> std::io::Result<bool> {
        let source = self.artifact_path(content_hash, filename)?;
        if !source.exists() {
            return Ok(false);
        }
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(&source, target_path)?;
        Ok(true)
    }

    fn artifact_path(&self, content_hash: &str, filename: &str) -> std::io::Result<PathBuf> {
        if !content_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "content_hash must contain only hex characters",
            ));
        }
        if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "filename must not contain path separators or '..'",
            ));
        }
        let prefix = &content_hash[..2.min(content_hash.len())];
        Ok(self.base_dir.join(prefix).join(content_hash).join(filename))
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

// ---------------------------------------------------------------------------
// Artifact metadata store (094 Slice 3)
// ---------------------------------------------------------------------------

/// Metadata about a stored artifact with provenance (094).
#[derive(Debug, Clone)]
pub struct ArtifactRecord {
    pub content_hash: String,
    pub filename: String,
    pub step_id: String,
    pub workflow_id: String,
    pub workspace_id: Option<String>,
    pub created_at: String,
    pub size_bytes: u64,
    pub content_type: Option<String>,
    pub producer_agent: Option<String>,
}

/// Provenance relationship for artifact lineage (094 Slice 4).
#[derive(Debug, Clone)]
pub struct ArtifactLineage {
    pub content_hash: String,
    pub relationship: LineageRelation,
    pub workflow_id: String,
    pub step_id: String,
    pub agent_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineageRelation {
    ProducedBy,
    ConsumedBy,
}

impl LineageRelation {
    fn as_str(&self) -> &str {
        match self {
            LineageRelation::ProducedBy => "produced_by",
            LineageRelation::ConsumedBy => "consumed_by",
        }
    }
}

/// SQLite-backed metadata store for artifact records and lineage (094 Slices 3-4).
#[cfg(feature = "local-sqlite")]
pub struct ArtifactMetadataStore {
    conn: rusqlite::Connection,
}

#[cfg(feature = "local-sqlite")]
impl ArtifactMetadataStore {
    /// Open or create the metadata DB at `path`.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS artifact_records (
                content_hash TEXT NOT NULL,
                filename TEXT NOT NULL,
                step_id TEXT NOT NULL,
                workflow_id TEXT NOT NULL,
                workspace_id TEXT,
                created_at TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                content_type TEXT,
                producer_agent TEXT,
                PRIMARY KEY (content_hash, workflow_id, step_id, filename)
            );
            CREATE TABLE IF NOT EXISTS artifact_lineage (
                content_hash TEXT NOT NULL,
                relationship TEXT NOT NULL,
                workflow_id TEXT NOT NULL,
                step_id TEXT NOT NULL,
                agent_id TEXT,
                created_at TEXT NOT NULL,
                PRIMARY KEY (content_hash, relationship, workflow_id, step_id)
            );
            CREATE INDEX IF NOT EXISTS idx_artifact_workspace
                ON artifact_records(workspace_id);
            CREATE INDEX IF NOT EXISTS idx_artifact_workflow
                ON artifact_records(workflow_id);
            CREATE INDEX IF NOT EXISTS idx_lineage_hash
                ON artifact_lineage(content_hash);",
        )?;
        Ok(Self { conn })
    }

    /// Record metadata for a stored artifact.
    pub fn record_artifact(&self, record: &ArtifactRecord) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO artifact_records
             (content_hash, filename, step_id, workflow_id, workspace_id,
              created_at, size_bytes, content_type, producer_agent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                record.content_hash,
                record.filename,
                record.step_id,
                record.workflow_id,
                record.workspace_id,
                record.created_at,
                record.size_bytes as i64,
                record.content_type,
                record.producer_agent,
            ],
        )?;
        Ok(())
    }

    /// Record a provenance relationship.
    pub fn record_lineage(&self, lineage: &ArtifactLineage) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO artifact_lineage
             (content_hash, relationship, workflow_id, step_id, agent_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                lineage.content_hash,
                lineage.relationship.as_str(),
                lineage.workflow_id,
                lineage.step_id,
                lineage.agent_id,
                lineage.created_at,
            ],
        )?;
        Ok(())
    }

    /// Look up artifacts by content hash.
    pub fn find_by_hash(&self, content_hash: &str) -> rusqlite::Result<Vec<ArtifactRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_hash, filename, step_id, workflow_id, workspace_id,
                    created_at, size_bytes, content_type, producer_agent
             FROM artifact_records WHERE content_hash = ?1",
        )?;
        let rows = stmt.query_map([content_hash], |row| {
            Ok(ArtifactRecord {
                content_hash: row.get(0)?,
                filename: row.get(1)?,
                step_id: row.get(2)?,
                workflow_id: row.get(3)?,
                workspace_id: row.get(4)?,
                created_at: row.get(5)?,
                size_bytes: row.get::<_, i64>(6)? as u64,
                content_type: row.get(7)?,
                producer_agent: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    /// Look up artifacts by workflow ID.
    pub fn find_by_workflow(&self, workflow_id: &str) -> rusqlite::Result<Vec<ArtifactRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_hash, filename, step_id, workflow_id, workspace_id,
                    created_at, size_bytes, content_type, producer_agent
             FROM artifact_records WHERE workflow_id = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map([workflow_id], |row| {
            Ok(ArtifactRecord {
                content_hash: row.get(0)?,
                filename: row.get(1)?,
                step_id: row.get(2)?,
                workflow_id: row.get(3)?,
                workspace_id: row.get(4)?,
                created_at: row.get(5)?,
                size_bytes: row.get::<_, i64>(6)? as u64,
                content_type: row.get(7)?,
                producer_agent: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    /// Get lineage for an artifact (who produced it, who consumed it).
    pub fn get_lineage(&self, content_hash: &str) -> rusqlite::Result<Vec<ArtifactLineage>> {
        let mut stmt = self.conn.prepare(
            "SELECT content_hash, relationship, workflow_id, step_id, agent_id, created_at
             FROM artifact_lineage WHERE content_hash = ?1 ORDER BY created_at",
        )?;
        let rows = stmt.query_map([content_hash], |row| {
            let rel: String = row.get(1)?;
            Ok(ArtifactLineage {
                content_hash: row.get(0)?,
                relationship: if rel == "produced_by" {
                    LineageRelation::ProducedBy
                } else {
                    LineageRelation::ConsumedBy
                },
                workflow_id: row.get(2)?,
                step_id: row.get(3)?,
                agent_id: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect()
    }

    /// Check whether any prior run produced an artifact with this hash.
    pub fn has_artifact(&self, content_hash: &str) -> rusqlite::Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM artifact_records WHERE content_hash = ?1",
            [content_hash],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_under_base() {
        let am = ArtifactManager::new("/tmp/oap-artifacts");
        let run = Uuid::nil();
        let p = am.output_artifact_path(run, "step-01", "out.md");
        assert!(
            p.to_string_lossy()
                .contains("00000000-0000-0000-0000-000000000000")
        );
        assert!(p.to_string_lossy().ends_with("step-01/out.md"));
    }

    #[test]
    fn hash_artifact_produces_64_char_hex() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("data.txt");
        std::fs::write(&path, "hello world\n").unwrap();
        let hash = ArtifactManager::hash_artifact(&path).unwrap();
        assert_eq!(hash.len(), 64, "SHA-256 hex should be 64 chars");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn verify_artifact_detects_match_and_mismatch() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("data.txt");
        std::fs::write(&path, "test content").unwrap();

        let hash = ArtifactManager::hash_artifact(&path).unwrap();
        assert!(ArtifactManager::verify_artifact(&path, &hash).unwrap());
        assert!(
            !ArtifactManager::verify_artifact(
                &path,
                "0000000000000000000000000000000000000000000000000000000000000000"
            )
            .unwrap()
        );
    }

    #[test]
    fn verify_artifact_detects_tampering() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("artifact.md");
        std::fs::write(&path, "original content").unwrap();
        let hash = ArtifactManager::hash_artifact(&path).unwrap();

        // Tamper with file
        std::fs::write(&path, "tampered content").unwrap();
        assert!(!ArtifactManager::verify_artifact(&path, &hash).unwrap());
    }

    // -- CAS tests (094 Slice 2) --

    #[test]
    fn cas_store_and_retrieve() {
        let store_dir = tempfile::TempDir::new().unwrap();
        let cas = ContentAddressedStore::new(store_dir.path()).unwrap();

        let src_dir = tempfile::TempDir::new().unwrap();
        let src_path = src_dir.path().join("output.md");
        std::fs::write(&src_path, "# Test\nContent here.").unwrap();

        let stored = cas.store(&src_path).unwrap();
        assert_eq!(stored.content_hash.len(), 64);
        assert!(stored.size_bytes > 0);
        assert!(cas.exists(&stored.content_hash));

        let dst_dir = tempfile::TempDir::new().unwrap();
        let dst_path = dst_dir.path().join("retrieved.md");
        assert!(
            cas.retrieve(&stored.content_hash, "output.md", &dst_path)
                .unwrap()
        );
        assert_eq!(
            std::fs::read_to_string(&dst_path).unwrap(),
            "# Test\nContent here."
        );
    }

    #[test]
    fn cas_deduplication() {
        let store_dir = tempfile::TempDir::new().unwrap();
        let cas = ContentAddressedStore::new(store_dir.path()).unwrap();

        let src_dir = tempfile::TempDir::new().unwrap();
        let p1 = src_dir.path().join("a.txt");
        let p2 = src_dir.path().join("b.txt");
        std::fs::write(&p1, "same content").unwrap();
        std::fs::write(&p2, "same content").unwrap();

        let s1 = cas.store(&p1).unwrap();
        let s2 = cas.store(&p2).unwrap();
        assert_eq!(s1.content_hash, s2.content_hash);
    }

    #[test]
    fn cas_nonexistent_hash() {
        let store_dir = tempfile::TempDir::new().unwrap();
        let cas = ContentAddressedStore::new(store_dir.path()).unwrap();
        let zero = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(!cas.exists(zero));
        let dst = store_dir.path().join("nope.txt");
        assert!(!cas.retrieve(zero, "nope.txt", &dst).unwrap());
    }

    #[test]
    fn promote_to_cas_copies_outputs() {
        let art_dir = tempfile::TempDir::new().unwrap();
        let cas_dir = tempfile::TempDir::new().unwrap();
        let am = ArtifactManager::new(art_dir.path());
        let cas = ContentAddressedStore::new(cas_dir.path()).unwrap();
        let run = Uuid::nil();

        am.ensure_step_dir(run, "step-01").unwrap();
        let out_path = am.output_artifact_path(run, "step-01", "result.md");
        std::fs::write(&out_path, "promoted content").unwrap();

        let promoted = am
            .promote_to_cas(run, "step-01", &["result.md".to_string()], &cas)
            .unwrap();
        assert_eq!(promoted.len(), 1);
        assert!(cas.exists(&promoted[0].content_hash));
    }

    #[test]
    fn cas_rejects_path_traversal_in_filename() {
        let store_dir = tempfile::TempDir::new().unwrap();
        let cas = ContentAddressedStore::new(store_dir.path()).unwrap();
        let zero = "0000000000000000000000000000000000000000000000000000000000000000";
        let dst = store_dir.path().join("out.txt");
        let result = cas.retrieve(zero, "../../etc/passwd", &dst);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path separators"));
    }

    #[test]
    fn cas_rejects_non_hex_content_hash() {
        let store_dir = tempfile::TempDir::new().unwrap();
        let cas = ContentAddressedStore::new(store_dir.path()).unwrap();
        let dst = store_dir.path().join("out.txt");
        let result = cas.retrieve("../../../etc", "file.txt", &dst);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("hex"));
    }

    // -- ArtifactMetadataStore tests (094 Slices 3-4) --

    #[cfg(feature = "local-sqlite")]
    #[test]
    fn metadata_store_record_and_find() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = ArtifactMetadataStore::open(&dir.path().join("meta.db")).unwrap();

        let record = ArtifactRecord {
            content_hash: "abc123".into(),
            filename: "output.md".into(),
            step_id: "step-01".into(),
            workflow_id: "wf-001".into(),
            workspace_id: Some("ws-001".into()),
            created_at: "2026-04-11T00:00:00Z".into(),
            size_bytes: 1024,
            content_type: Some("text/markdown".into()),
            producer_agent: Some("architect".into()),
        };
        store.record_artifact(&record).unwrap();

        let found = store.find_by_hash("abc123").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].filename, "output.md");
        assert_eq!(found[0].workspace_id.as_deref(), Some("ws-001"));

        let by_wf = store.find_by_workflow("wf-001").unwrap();
        assert_eq!(by_wf.len(), 1);

        assert!(store.has_artifact("abc123").unwrap());
        assert!(!store.has_artifact("nonexistent").unwrap());
    }

    #[cfg(feature = "local-sqlite")]
    #[test]
    fn lineage_record_and_query() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = ArtifactMetadataStore::open(&dir.path().join("meta.db")).unwrap();

        let produced = ArtifactLineage {
            content_hash: "abc123".into(),
            relationship: LineageRelation::ProducedBy,
            workflow_id: "wf-001".into(),
            step_id: "step-01".into(),
            agent_id: Some("architect".into()),
            created_at: "2026-04-11T00:00:00Z".into(),
        };
        store.record_lineage(&produced).unwrap();

        let consumed = ArtifactLineage {
            content_hash: "abc123".into(),
            relationship: LineageRelation::ConsumedBy,
            workflow_id: "wf-001".into(),
            step_id: "step-02".into(),
            agent_id: Some("implementer".into()),
            created_at: "2026-04-11T00:01:00Z".into(),
        };
        store.record_lineage(&consumed).unwrap();

        let lineage = store.get_lineage("abc123").unwrap();
        assert_eq!(lineage.len(), 2);
        assert_eq!(lineage[0].relationship, LineageRelation::ProducedBy);
        assert_eq!(lineage[1].relationship, LineageRelation::ConsumedBy);
    }
}
