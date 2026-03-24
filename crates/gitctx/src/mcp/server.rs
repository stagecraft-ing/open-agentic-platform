//! MCP server implementation for gitctx.
//!
//! This module contains the `GitCtxMcpServer` struct which implements the MCP
//! `ServerHandler` trait to expose GitHub exploration tools via the Model Context Protocol.

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::*,
    service::RequestContext,
    tool_handler,
};

use crate::context::GitHubContext;

/// MCP server for gitctx GitHub repository exploration.
///
/// This server exposes 23 tools for exploring GitHub repositories through the
/// Model Context Protocol. It maintains shared state via `GitHubContext` across
/// all tool invocations.
///
/// # Creating a Server
///
/// ```no_run
/// use gitctx::mcp::GitCtxMcpServer;
///
/// // Create from environment (checks GITHUB_TOKEN, GH_TOKEN, config file)
/// let server = GitCtxMcpServer::from_env();
///
/// // Or with explicit token
/// let server = GitCtxMcpServer::new(Some("ghp_xxx".to_string()));
/// ```
///
/// # Tool Categories
///
/// - **Code Navigation**: find_repo, list_dir, read_file, read_files, search_code, switch_branch
/// - **Issues**: search_issues, get_issue, list_issue_comments
/// - **Pull Requests**: search_prs, get_pr, list_pr_comments
/// - **Commits**: list_commits, get_commit, compare_commits, blame_file
/// - **Releases**: list_releases, get_release, compare_releases
/// - **Insights**: get_contributors, get_repo_stats, get_dependency_graph
#[derive(Clone)]
pub struct GitCtxMcpServer {
    /// Shared GitHub context across all tool calls.
    pub(crate) context: GitHubContext,
    /// Tool router for dispatching tool calls.
    pub(crate) tool_router: ToolRouter<Self>,
}

impl GitCtxMcpServer {
    /// Check if the server has authentication configured.
    ///
    /// # Returns
    ///
    /// `true` if a GitHub token is available, `false` otherwise.
    pub fn is_authenticated(&self) -> bool {
        self.context.get_token().is_some()
    }

    /// Get a reference to the server's context.
    ///
    /// This is primarily useful for testing and debugging.
    pub fn context(&self) -> &GitHubContext {
        &self.context
    }
}

#[tool_handler]
impl ServerHandler for GitCtxMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "MCP server for consulting up-to-date codebase, dependency, and library context via GitHub. \
                 Use this to explore source code, check latest API changes, review documentation, \
                 and understand implementation details that may have changed since your knowledge cutoff. \
                 Start by calling find_repo with an owner/name (e.g., 'rust-lang/rust') \
                 or a search query to select a repository. Then use other tools to explore \
                 code, issues, pull requests, commits, and releases."
                    .into(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![
                RawResource::new(
                    "gitctx://context/current",
                    "Current Repository Context",
                ).no_annotation(),
            ],
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        if request.uri == "gitctx://context/current" {
            let ctx_json = if let Some(repo) = self.context.get_repo() {
                serde_json::json!({
                    "repository": {
                        "owner": repo.owner,
                        "name": repo.name,
                        "full_name": format!("{}/{}", repo.owner, repo.name),
                        "default_branch": repo.default_branch,
                        "description": repo.description,
                        "is_private": repo.is_private
                    },
                    "current_branch": self.context.get_current_branch(),
                    "current_path": self.context.get_current_path(),
                    "authenticated": self.context.get_token().is_some(),
                    "status": "repository_selected"
                })
            } else {
                serde_json::json!({
                    "repository": null,
                    "current_branch": null,
                    "current_path": "/",
                    "authenticated": self.context.get_token().is_some(),
                    "status": "no_repository_selected",
                    "message": "No repository selected. Use the find_repo tool to select a repository."
                })
            };

            Ok(ReadResourceResult {
                contents: vec![ResourceContents::text(
                    serde_json::to_string_pretty(&ctx_json)
                        .unwrap_or_else(|_| ctx_json.to_string()),
                    "gitctx://context/current",
                )],
            })
        } else {
            Err(McpError::resource_not_found(
                format!("Unknown resource: {}", request.uri),
                None,
            ))
        }
    }
}
