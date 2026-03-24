use tauri::command;

#[command]
pub async fn mcp_list_tools(_server: String) -> Result<serde_json::Value, String> {
    // Placeholder for proxying list_tools to the actual sidecar process
    Ok(serde_json::json!({ "tools": [] }))
}

#[command]
pub async fn mcp_call_tool(_server: String, _tool_name: String, _args: serde_json::Value) -> Result<serde_json::Value, String> {
    // Placeholder for proxying call_tool
    Ok(serde_json::json!({ "error": "Not implemented yet" }))
}

#[command]
pub async fn mcp_read_resource(_server: String, _uri: String) -> Result<serde_json::Value, String> {
    // Placeholder for proxying read_resource
    Ok(serde_json::json!({ "error": "Not implemented yet" }))
}

// Stubs for missing mcp functions
#[command] pub async fn mcp_add() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_list() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_get() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_remove() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_add_json() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_add_from_claude_desktop() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_serve() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_test_connection() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_reset_project_choices() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_get_server_status() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_read_project_config() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_save_project_config() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
