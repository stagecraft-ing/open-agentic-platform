use tauri::command;
use titor::{TitorBuilder, CompressionStrategy};
use std::path::PathBuf;

#[command]
pub async fn titor_init(root_path: String, storage_path: Option<String>) -> Result<String, String> {
    let root = PathBuf::from(&root_path);
    let storage = PathBuf::from(storage_path.unwrap_or_else(|| format!("{}/.titor", root_path)));
    TitorBuilder::new()
        .compression_strategy(CompressionStrategy::Adaptive { min_size: 4096, skip_extensions: vec![] })
        .build(root, storage)
        .map(|_| "initialized".to_string())
        .map_err(|e| e.to_string())
}

#[command]
pub async fn titor_checkpoint(_root_path: String, _message: Option<String>) -> Result<serde_json::Value, String> {
    todo!("implement titor state management via Tauri AppState")
}

#[command]
pub async fn titor_list(_root_path: String) -> Result<serde_json::Value, String> { todo!() }

#[command]
pub async fn titor_restore(_root_path: String, _checkpoint_id: String) -> Result<(), String> { todo!() }

#[command]
pub async fn titor_diff(_root_path: String, _id1: String, _id2: String) -> Result<serde_json::Value, String> { todo!() }

#[command]
pub async fn titor_verify(_root_path: String, _checkpoint_id: String) -> Result<serde_json::Value, String> { todo!() }
