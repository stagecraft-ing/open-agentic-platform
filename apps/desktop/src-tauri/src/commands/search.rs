use tauri::command;
use stackwalk::{config::Config, indexer::index_directory};

#[command]
pub async fn stackwalk_index(path: String, config_toml: String) -> Result<serde_json::Value, String> {
    let config = Config::from_toml(&config_toml).map_err(|e| e.to_string())?;
    let (_blocks, _call_stack, call_graph) = index_directory(&config, &path);
    serde_json::to_value(&call_graph).map_err(|e| e.to_string())
}

#[command]
pub async fn blockoli_index_project(_project_name: String, _path: String) -> Result<String, String> {
    todo!()
}

#[command]
pub async fn blockoli_search(_project_name: String, _query: String) -> Result<serde_json::Value, String> {
    todo!()
}

#[command]
pub async fn search_codebase() -> Result<serde_json::Value, String> {
    Err("Not implemented yet".to_string())
}
