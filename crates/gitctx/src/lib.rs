//! gitctx - MCP server for GitHub repository exploration.
//!
//! This library provides the core functionality for exploring GitHub repositories
//! through the Model Context Protocol (MCP). It includes:
//!
//! - **Context management**: Thread-safe state tracking for repository exploration
//! - **GitHub API client**: Wrappers around octocrab for repository operations
//! - **Authentication**: GitHub token management via environment variables and config files
//! - **Caching**: In-memory API caching for directory listings and file contents
//! - **MCP server**: Model Context Protocol server for integration with AI coding tools
//!
//! # MCP Server
//!
//! The library provides an MCP server that can be used with Claude Code, Cursor,
//! and other MCP-compatible tools. Run with the `gitctx-mcp` binary.
//!
//! # Example
//!
//! ```no_run
//! use gitctx::context::GitHubContext;
//! use gitctx::github;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create a context with authentication
//! let ctx = GitHubContext::new(Some("ghp_xxx".to_string()));
//!
//! // Create a GitHub client
//! let client = github::create_client(Some("ghp_xxx"))?;
//!
//! // Fetch repository info
//! let info = github::get_repo_info(&client, "rust-lang", "rust").await?;
//! println!("Default branch: {}", info.default_branch);
//! # Ok(())
//! # }
//! ```

pub mod auth;
pub mod cache;
pub mod context;
pub mod github;
pub mod mcp;
pub mod xml_format;
