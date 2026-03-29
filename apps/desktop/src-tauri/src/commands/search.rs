use std::path::Path;
use std::sync::{Arc, Mutex};

use blockoli::blocks::EmbeddedBlock;
use blockoli::embeddings::encoder::Embeddings;
use blockoli::vector_store::sqlite::SQLite;
use blockoli::vector_store::vector_store::VectorStore;
use serde_json::json;
use stackwalk::{config::Config, indexer::index_directory};
use tauri::command;

/// Embedded copy of `crates/asterisk/asterisk.toml` (Rust + Python matchers).
const ASTERISK_CONFIG_TOML: &str = r#"[languages]
  [languages.python]
    [languages.python.matchers]
      import_statement = "import_from_statement"
      [languages.python.matchers.module_name]
        field_name = "module_name"
        kind = "dotted_name"

      [languages.python.matchers.object_name]
        field_name = "name"
        kind = "dotted_name"

      [languages.python.matchers.alias]
        field_name = "alias"
        kind = "identifier"

  [languages.rust]
    [languages.rust.matchers]
      import_statement = "use_declaration"
      [languages.rust.matchers.module_name]
        field_name = "path"
        kind = "identifier"

      [languages.rust.matchers.object_name]
        field_name = "name"
        kind = "identifier"

      [languages.rust.matchers.alias]
        field_name = "alias"
        kind = "identifier"
"#;

/// Managed [`VectorStore`] for semantic code search (SQLite under the app data directory).
#[derive(Clone)]
pub struct BlockoliState {
    pub store: Arc<Mutex<VectorStore>>,
}

impl BlockoliState {
    pub fn new(store: VectorStore) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
        }
    }
}

fn validate_project_name(name: &str) -> Result<(), String> {
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("project name must contain only letters, digits, and underscores".to_string());
    }
    Ok(())
}

#[command]
pub async fn stackwalk_index(
    path: String,
    config_toml: String,
) -> Result<serde_json::Value, String> {
    let config = Config::from_toml(&config_toml).map_err(|e| e.to_string())?;
    let (_blocks, _call_stack, call_graph) = index_directory(&config, &path);
    serde_json::to_value(&call_graph).map_err(|e| e.to_string())
}

#[command]
pub async fn blockoli_index_project(
    state: tauri::State<'_, BlockoliState>,
    project_name: String,
    path: String,
) -> Result<serde_json::Value, String> {
    validate_project_name(&project_name)?;
    let root = Path::new(&path);
    if !root.exists() {
        return Err(format!("path does not exist: {path}"));
    }
    if !root.is_dir() {
        return Err(format!("path is not a directory: {path}"));
    }

    let config = asterisk::config::Config::from_toml(ASTERISK_CONFIG_TOML)
        .map_err(|e| format!("asterisk config: {e}"))?;

    let path_for_blocking = path.clone();
    let embedded_result = tokio::task::spawn_blocking(move || {
        let (blocks, _, _) = asterisk::indexer::index_directory(&config, &path_for_blocking);
        let code_blocks: Vec<String> = blocks.iter().map(|b| b.content.clone()).collect();
        let vectors = Embeddings::generate_vector_set(code_blocks).map_err(|e| e.to_string())?;
        let mut embedded_blocks = Vec::with_capacity(blocks.len());
        for (i, block) in blocks.iter().enumerate() {
            embedded_blocks.push(EmbeddedBlock {
                block: block.clone(),
                vectors: vectors[i].point.to_vec(),
            });
        }
        Ok::<_, String>((embedded_blocks, blocks.len()))
    })
    .await
    .map_err(|e| format!("index task failed: {e}"))?;

    let (embedded_blocks, block_count) = embedded_result?;

    let store = state.store.clone();
    let project_name_db = project_name.clone();
    tokio::task::spawn_blocking(move || {
        let mut store = store.lock().map_err(|e| e.to_string())?;
        match &mut *store {
            VectorStore::SQLiteStore(conn) => {
                if SQLite::does_project_exist(conn, &project_name_db).map_err(|e| e.to_string())? {
                    SQLite::delete_project(conn, &project_name_db).map_err(|e| e.to_string())?;
                }
                SQLite::create_table(conn, &project_name_db).map_err(|e| e.to_string())?;
                SQLite::insert_blocks(conn, &project_name_db, embedded_blocks)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| format!("database task failed: {e}"))??;

    Ok(json!({
        "project_name": project_name,
        "total_blocks": block_count as i32,
    }))
}

#[command]
pub async fn blockoli_search(
    state: tauri::State<'_, BlockoliState>,
    project_name: String,
    query: String,
) -> Result<serde_json::Value, String> {
    validate_project_name(&project_name)?;
    let store = state.store.clone();
    tokio::task::spawn_blocking(move || {
        let store = store.lock().map_err(|e| e.to_string())?;
        match &*store {
            VectorStore::SQLiteStore(conn) => {
                if !SQLite::does_project_exist(conn, &project_name).map_err(|e| e.to_string())? {
                    return Err(format!(
                        "project '{project_name}' does not exist; index the project with blockoli_index_project first"
                    ));
                }
                if let Some(info) =
                    SQLite::get_project_info(conn, &project_name).map_err(|e| e.to_string())?
                {
                    if info.total_code_blocks == 0 {
                        return Err(format!(
                            "project '{project_name}' has no indexed blocks; run blockoli_index_project first"
                        ));
                    }
                }
                let code_vectors =
                    SQLite::get_code_vectors(conn, &project_name).map_err(|e| e.to_string())?;
                let nearest =
                    Embeddings::search(code_vectors, query, 5).map_err(|e| e.to_string())?;
                serde_json::to_value(&nearest).map_err(|e| e.to_string())
            }
        }
    })
    .await
    .map_err(|e| format!("search task failed: {e}"))?
}

#[command]
pub async fn search_codebase() -> Result<serde_json::Value, String> {
    Err("Not implemented yet".to_string())
}
