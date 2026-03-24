use tauri::command;
use xray::scan_target;
use std::path::PathBuf;

#[command]
pub async fn xray_scan_project(path: String) -> Result<serde_json::Value, String> {
    let target = PathBuf::from(&path);
    let index = scan_target(&target, None).map_err(|e| e.to_string())?;
    serde_json::to_value(&index).map_err(|e| e.to_string())
}

#[command]
pub async fn featuregraph_overview(_features_yaml_path: String) -> Result<serde_json::Value, String> {
    todo!()
}

#[command]
pub async fn featuregraph_impact(_file_paths: Vec<String>, _features_yaml_path: String) -> Result<serde_json::Value, String> {
    todo!()
}
