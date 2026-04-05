// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! MCP proxy commands for the desktop app (Feature 032 / T006).
//!
//! MCP tool calls are handled by the axiomregent sidecar, which is spawned
//! by the desktop app and communicates via MCP stdio framing.

use serde_json::json;
use tauri::command;

#[command]
pub async fn mcp_list_tools(_server: String) -> Result<serde_json::Value, String> {
    Err("MCP tool listing not implemented — use axiomregent sidecar directly".to_string())
}

#[command]
pub async fn mcp_call_tool(
    _server: String,
    _tool_name: String,
    _args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Err("MCP tool calls not implemented — use axiomregent sidecar directly".to_string())
}

#[command]
pub async fn mcp_read_resource(_server: String, _uri: String) -> Result<serde_json::Value, String> {
    Err("MCP resource reads not implemented — use axiomregent sidecar directly".to_string())
}

#[command] pub async fn mcp_add() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_list() -> Result<serde_json::Value, String> { Ok(json!([])) }
#[command] pub async fn mcp_get() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_remove() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_add_json() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_add_from_claude_desktop() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_serve() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_test_connection() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
#[command] pub async fn mcp_reset_project_choices() -> Result<serde_json::Value, String> { Ok(json!("ok")) }
#[command] pub async fn mcp_get_server_status() -> Result<serde_json::Value, String> { Ok(json!({})) }
#[command] pub async fn mcp_read_project_config() -> Result<serde_json::Value, String> { Ok(json!({"mcpServers": {}})) }
#[command] pub async fn mcp_save_project_config() -> Result<serde_json::Value, String> { Err("Not implemented yet".to_string()) }
