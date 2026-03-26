//! MCP proxy commands for the desktop app (Feature 032 / T006).
//!
//! **gitctx:** Rust-owned, **per-request** MCP over **stdio** to the bundled `gitctx-mcp`
//! binary (`src-tauri/binaries/gitctx-mcp-*`). There is no long-lived gitctx process or
//! port-based readiness for gitctx — enrichment readiness is whatever this bridge returns.
//!
//! **`get_sidecar_ports`** is unrelated to gitctx; it is for sidecars that announce a TCP
//! port (e.g. axiomregent).

use serde_json::{Value, json};
use std::path::PathBuf;
use tauri::command;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

const MCP_TIMEOUT: Duration = Duration::from_secs(10);

fn ensure_gitctx_server(server: &str) -> Result<(), String> {
    if server == "gitctx" {
        Ok(())
    } else {
        Err(format!(
            "Unsupported MCP server '{server}'. T006 bridge currently supports only 'gitctx'."
        ))
    }
}

fn bundled_gitctx_mcp_binary_path() -> Result<PathBuf, String> {
    let suffix = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "aarch64-apple-darwin"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        "x86_64-apple-darwin"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "aarch64-unknown-linux-gnu"
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        "x86_64-pc-windows-msvc"
    } else {
        return Err("Unsupported host target for bundled gitctx-mcp binary resolution".to_string());
    };

    let mut filename = format!("gitctx-mcp-{suffix}");
    if cfg!(target_os = "windows") {
        filename.push_str(".exe");
    }

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(filename);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!(
            "bundled gitctx-mcp binary not found at {}",
            path.display()
        ))
    }
}

async fn read_mcp_message<R>(reader: &mut BufReader<R>) -> Result<Value, String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).await.map_err(|e| e.to_string())?;
        if read == 0 {
            return Err("EOF while reading MCP headers".to_string());
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.to_ascii_lowercase().strip_prefix("content-length:") {
            let parsed = rest
                .trim()
                .parse::<usize>()
                .map_err(|_| format!("Invalid Content-Length header value: {}", rest.trim()))?;
            content_length = Some(parsed);
        }
    }

    let len = content_length.ok_or_else(|| "Missing Content-Length header".to_string())?;
    let mut payload = vec![0u8; len];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_slice::<Value>(&payload).map_err(|e| format!("Invalid MCP JSON payload: {e}"))
}

async fn write_mcp_message<W>(writer: &mut W, payload: &Value) -> Result<(), String>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let bytes = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let header = format!("Content-Length: {}\r\n\r\n", bytes.len());
    writer
        .write_all(header.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    writer.write_all(&bytes).await.map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())
}

async fn read_response_for_id<R>(reader: &mut BufReader<R>, id: i64) -> Result<Value, String>
where
    R: tokio::io::AsyncRead + Unpin,
{
    loop {
        let msg = read_mcp_message(reader).await?;
        if msg.get("id").and_then(Value::as_i64) == Some(id) {
            return Ok(msg);
        }
    }
}

async fn execute_gitctx_rpc(method: &str, params: Value) -> Result<Value, String> {
    let binary_path = bundled_gitctx_mcp_binary_path()?;
    let mut child = Command::new(binary_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn gitctx-mcp: {e}"))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Failed to open gitctx-mcp stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to open gitctx-mcp stdout".to_string())?;
    let mut reader = BufReader::new(stdout);

    let init_req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "opc-desktop",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    });
    write_mcp_message(&mut stdin, &init_req).await?;
    let init_resp = timeout(MCP_TIMEOUT, read_response_for_id(&mut reader, 1))
        .await
        .map_err(|_| "Timed out waiting for MCP initialize response".to_string())??;
    if let Some(err) = init_resp.get("error") {
        return Err(format!("MCP initialize failed: {err}"));
    }

    let req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": method,
        "params": params
    });
    write_mcp_message(&mut stdin, &req).await?;
    let resp = timeout(MCP_TIMEOUT, read_response_for_id(&mut reader, 2))
        .await
        .map_err(|_| format!("Timed out waiting for MCP response to method '{method}'"))??;

    // Best effort cleanup.
    let _ = child.start_kill();
    let _ = child.wait().await;

    if let Some(err) = resp.get("error") {
        Err(format!("MCP {method} error: {err}"))
    } else {
        Ok(resp
            .get("result")
            .cloned()
            .unwrap_or_else(|| json!({ "status": "ok" })))
    }
}

#[command]
pub async fn mcp_list_tools(server: String) -> Result<serde_json::Value, String> {
    ensure_gitctx_server(&server)?;
    execute_gitctx_rpc("tools/list", json!({})).await
}

#[command]
pub async fn mcp_call_tool(
    server: String,
    tool_name: String,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    ensure_gitctx_server(&server)?;
    execute_gitctx_rpc(
        "tools/call",
        json!({
            "name": tool_name,
            "arguments": args
        }),
    )
    .await
}

#[command]
pub async fn mcp_read_resource(server: String, uri: String) -> Result<serde_json::Value, String> {
    ensure_gitctx_server(&server)?;
    execute_gitctx_rpc(
        "resources/read",
        json!({
            "uri": uri
        }),
    )
    .await
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
