//! gitctx MCP server binary entry point.
//!
//! This binary exposes gitctx's GitHub exploration tools via the Model Context
//! Protocol (MCP) for use with Claude Code, Cursor, and other MCP-compatible tools.
//!
//! # Usage
//!
//! ```bash
//! # Run the MCP server (typically spawned by an MCP client)
//! gitctx-mcp
//!
//! # With authentication
//! GITHUB_TOKEN=ghp_xxx gitctx-mcp
//! ```
//!
//! # Configuration for Claude Code
//!
//! Add to `~/.claude/mcp.json`:
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "gitctx": {
//!       "command": "gitctx-mcp",
//!       "env": {
//!         "GITHUB_TOKEN": "${GITHUB_TOKEN}"
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! # Authentication
//!
//! Token resolution order:
//! 1. `GITHUB_TOKEN` environment variable
//! 2. `GH_TOKEN` environment variable
//! 3. `~/.config/gitctx/token.json` config file

use gitctx::mcp::GitCtxMcpServer;
use rmcp::{transport::io::stdio, ServiceExt};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging to stderr (required for MCP stdio transport).
    // MCP servers must use stderr for logging since stdout is used for protocol messages.
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gitctx_mcp=info,rmcp=warn".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .init();

    tracing::info!(
        "Starting gitctx MCP server v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Create server with authentication from environment.
    // Checks: GITHUB_TOKEN, GH_TOKEN, ~/.config/gitctx/token.json
    let server = GitCtxMcpServer::from_env();

    if server.is_authenticated() {
        tracing::info!("GitHub authentication configured");
    } else {
        tracing::warn!(
            "No GitHub token found. Set GITHUB_TOKEN environment variable for full access."
        );
    }

    // Serve on stdio transport.
    // This is the standard transport for MCP servers spawned by clients like Claude Code.
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Failed to start MCP server: {:?}", e);
    })?;

    tracing::info!("MCP server started, waiting for client connections...");

    // Wait for the service to complete (client disconnects or error).
    service.waiting().await?;

    tracing::info!("MCP server shutting down");

    Ok(())
}
