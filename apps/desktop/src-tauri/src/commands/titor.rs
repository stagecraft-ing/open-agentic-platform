use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::command;
use tokio::sync::{Mutex, RwLock};
use titor::{CompressionStrategy, Titor, TitorBuilder};

/// Managed [`Titor`] instances keyed by canonical project root (see [`canonical_root`]).
#[derive(Clone)]
pub struct TitorState {
    instances: Arc<RwLock<HashMap<PathBuf, Arc<Mutex<Titor>>>>>,
    /// Storage path reported from [`TitorState::get_or_init`] for each root (idempotent).
    storage_paths: Arc<RwLock<HashMap<PathBuf, PathBuf>>>,
}

impl TitorState {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
            storage_paths: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Returns the [`Titor`] handle for `root_path` if [`TitorState::get_or_init`] succeeded.
    pub async fn get(&self, root_path: &str) -> Option<Arc<Mutex<Titor>>> {
        let key = canonical_root(root_path).ok()?;
        let map = self.instances.read().await;
        map.get(&key).map(Arc::clone)
    }

    /// Builds or reuses a [`Titor`] for `root_path`. Returns the storage path on success.
    pub async fn get_or_init(
        &self,
        root_path: String,
        storage_path: PathBuf,
    ) -> Result<String, String> {
        let root_key = canonical_root(&root_path)?;
        {
            let map = self.instances.read().await;
            if map.contains_key(&root_key) {
                let paths = self.storage_paths.read().await;
                if let Some(p) = paths.get(&root_key) {
                    return Ok(p.to_string_lossy().to_string());
                }
            }
        }
        let mut map = self.instances.write().await;
        if map.contains_key(&root_key) {
            let paths = self.storage_paths.read().await;
            return paths
                .get(&root_key)
                .map(|p| p.to_string_lossy().to_string())
                .ok_or_else(|| "internal: missing storage path for initialized Titor".to_string());
        }
        let titor = build_titor(root_key.clone(), storage_path.clone())?;
        map.insert(root_key.clone(), Arc::new(Mutex::new(titor)));
        let mut paths = self.storage_paths.write().await;
        paths.insert(root_key, storage_path.clone());
        drop(paths);
        drop(map);
        Ok(storage_path.to_string_lossy().to_string())
    }

    async fn require_titor(&self, root_path: &str) -> Result<Arc<Mutex<Titor>>, String> {
        let key = canonical_root(root_path)?;
        let map = self.instances.read().await;
        map.get(&key)
            .map(Arc::clone)
            .ok_or_else(|| missing_instance_error(root_path))
    }
}

impl Default for TitorState {
    fn default() -> Self {
        Self::new()
    }
}

fn build_titor(root: PathBuf, storage: PathBuf) -> Result<Titor, String> {
    TitorBuilder::new()
        .compression_strategy(CompressionStrategy::Adaptive {
            min_size: 4096,
            skip_extensions: vec![],
        })
        .build(root, storage)
        .map_err(|e| e.to_string())
}

fn canonical_root(root_path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(root_path);
    if !p.exists() {
        return Err(format!("root path does not exist: {root_path}"));
    }
    p.canonicalize()
        .map_err(|e| format!("failed to canonicalize root path: {e}"))
}

fn missing_instance_error(root_path: &str) -> String {
    format!("no Titor instance for root {root_path}; call titor_init first")
}

#[command]
pub async fn titor_init(
    state: tauri::State<'_, TitorState>,
    root_path: String,
    storage_path: Option<String>,
) -> Result<String, String> {
    let storage = PathBuf::from(storage_path.unwrap_or_else(|| format!("{root_path}/.titor")));
    state.get_or_init(root_path, storage).await
}

#[command]
pub async fn titor_checkpoint(
    state: tauri::State<'_, TitorState>,
    root_path: String,
    message: Option<String>,
) -> Result<serde_json::Value, String> {
    let t = state.require_titor(&root_path).await?;
    let mut guard = t.lock().await;
    let cp = guard.checkpoint(message).map_err(|e| e.to_string())?;
    serde_json::to_value(&cp).map_err(|e| e.to_string())
}

#[command]
pub async fn titor_list(
    state: tauri::State<'_, TitorState>,
    root_path: String,
) -> Result<serde_json::Value, String> {
    let t = state.require_titor(&root_path).await?;
    let guard = t.lock().await;
    let list = guard.list_checkpoints().map_err(|e| e.to_string())?;
    serde_json::to_value(&list).map_err(|e| e.to_string())
}

#[command]
pub async fn titor_restore(
    state: tauri::State<'_, TitorState>,
    root_path: String,
    checkpoint_id: String,
) -> Result<(), String> {
    let t = state.require_titor(&root_path).await?;
    let mut guard = t.lock().await;
    guard
        .restore(&checkpoint_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn titor_diff(
    state: tauri::State<'_, TitorState>,
    root_path: String,
    id1: String,
    id2: String,
) -> Result<serde_json::Value, String> {
    let t = state.require_titor(&root_path).await?;
    let guard = t.lock().await;
    let diff = guard.diff(&id1, &id2).map_err(|e| e.to_string())?;
    serde_json::to_value(&diff).map_err(|e| e.to_string())
}

#[command]
pub async fn titor_verify(
    state: tauri::State<'_, TitorState>,
    root_path: String,
    checkpoint_id: String,
) -> Result<serde_json::Value, String> {
    let t = state.require_titor(&root_path).await?;
    let guard = t.lock().await;
    let report = guard
        .verify_checkpoint(&checkpoint_id)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(&report).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn round_trip_init_checkpoint_list_verify_diff_restore() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("project");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("a.txt"), "v1").unwrap();
        let storage = tmp.path().join("store");

        let state = TitorState::new();
        let root_s = root.to_string_lossy().to_string();
        state
            .get_or_init(root_s.clone(), storage.clone())
            .await
            .unwrap();

        let t = state.get(&root_s).await.expect("instance");
        let id1 = {
            let mut g = t.lock().await;
            let cp = g
                .checkpoint(Some("first".to_string()))
                .expect("checkpoint");
            cp.id.clone()
        };

        fs::write(root.join("a.txt"), "v2").unwrap();
        fs::write(root.join("b.txt"), "new").unwrap();
        let id2 = {
            let mut g = t.lock().await;
            let cp = g.checkpoint(Some("second".to_string())).expect("checkpoint");
            cp.id.clone()
        };

        let list = {
            let g = t.lock().await;
            g.list_checkpoints().expect("list")
        };
        assert_eq!(list.len(), 2);

        let report = {
            let g = t.lock().await;
            g.verify_checkpoint(&id2).expect("verify")
        };
        assert!(report.is_valid(), "{report:?}");

        let diff = {
            let g = t.lock().await;
            g.diff(&id1, &id2).expect("diff")
        };
        assert!(!diff.added_files.is_empty() || !diff.modified_files.is_empty());

        {
            let mut g = t.lock().await;
            g.restore(&id1).expect("restore");
        }
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "v1");
        assert!(!root.join("b.txt").exists());

        assert!(storage.join("metadata.json").exists());
    }

    #[tokio::test]
    async fn require_titor_errors_without_init() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("proj");
        fs::create_dir_all(&root).unwrap();
        let state = TitorState::new();
        let err = state
            .require_titor(&root.to_string_lossy())
            .await
            .expect_err("expected error");
        assert!(err.contains("titor_init"), "{err}");
    }
}
