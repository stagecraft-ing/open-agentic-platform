// DEPRECATED (Phase 6): blockoli and stackwalk crates have been absorbed/removed.
// These commands now return deprecation errors. Use axiomregent's search.index /
// search.semantic tools instead, accessed via the axiomregent sidecar.

use serde_json::json;
use tauri::command;

/// Stub state for blockoli — kept so that manage() calls and the invoke_handler still compile.
/// The underlying blockoli crate has been removed in Phase 6 cleanup.
#[derive(Clone, Default)]
pub struct BlockoliState;

fn deprecated_err(name: &str) -> String {
    format!(
        "{name} is deprecated (Phase 6): blockoli/stackwalk crates removed. \
         Use axiomregent search.index / search.semantic tools instead."
    )
}

#[command]
pub async fn stackwalk_index(
    _path: String,
    _config_toml: String,
) -> Result<serde_json::Value, String> {
    Err(deprecated_err("stackwalk_index"))
}

#[command]
pub async fn blockoli_index_project(
    _state: tauri::State<'_, BlockoliState>,
    _project_name: String,
    _path: String,
) -> Result<serde_json::Value, String> {
    Err(deprecated_err("blockoli_index_project"))
}

#[command]
pub async fn blockoli_search(
    _state: tauri::State<'_, BlockoliState>,
    _project_name: String,
    _query: String,
) -> Result<serde_json::Value, String> {
    Err(deprecated_err("blockoli_search"))
}

#[command]
pub async fn search_codebase() -> Result<serde_json::Value, String> {
    Ok(json!({
        "status": "deprecated",
        "message": "Use axiomregent search.semantic tool instead."
    }))
}
