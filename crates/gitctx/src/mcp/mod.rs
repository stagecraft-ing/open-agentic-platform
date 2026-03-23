//! MCP (Model Context Protocol) server module for gitctx.
//!
//! This module provides an MCP server that exposes gitctx's GitHub exploration
//! tools to MCP-compatible clients like Claude Code, Cursor, and other AI coding tools.
//!
//! # Overview
//!
//! The MCP server exposes 23 tools for exploring GitHub repositories:
//!
//! - **Code Navigation**: find_repo, list_dir, read_file, read_files, search_code, switch_branch
//! - **Issues**: search_issues, get_issue, list_issue_comments
//! - **Pull Requests**: search_prs, get_pr, get_pr_diff, list_pr_comments
//! - **Commits**: list_commits, get_commit, compare_commits, blame_file
//! - **Releases**: list_releases, get_release, compare_releases
//! - **Insights**: get_contributors, get_repo_stats, get_dependency_graph
//!
//! # Usage
//!
//! Run the MCP server binary:
//!
//! ```bash
//! gitctx-mcp
//! ```
//!
//! Or with authentication:
//!
//! ```bash
//! GITHUB_TOKEN=ghp_xxx gitctx-mcp
//! ```
//!
//! # Claude Code Configuration
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

pub mod resources;
pub mod server;
pub mod tools;

pub use server::GitCtxMcpServer;
