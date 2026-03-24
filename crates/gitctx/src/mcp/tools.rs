//! MCP tool definitions for gitctx.
//!
//! This module defines all tools exposed via the MCP protocol. Each tool
//! is implemented as an async method on `GitCtxMcpServer` using the `#[tool]`
//! attribute macro.

use rmcp::{
    ErrorData as McpError,
    handler::server::wrapper::Parameters,
    model::*,
    schemars, tool, tool_router,
};
use serde::Deserialize;

use crate::auth;
use crate::cache::{dir_cache_key, file_cache_key, ApiCache};
use crate::context::GitHubContext;
use crate::github::{client, commits, issues, pulls, releases, search, stats, IssueSearchFilters};
use crate::mcp::server::GitCtxMcpServer;
use crate::xml_format;

use once_cell::sync::Lazy;

// ============================================================================
// Caches
// ============================================================================

/// Global cache for directory listings.
static DIR_CACHE: Lazy<ApiCache<String>> = Lazy::new(ApiCache::new);

/// Global cache for file contents.
static FILE_CACHE: Lazy<ApiCache<String>> = Lazy::new(ApiCache::new);

// ============================================================================
// Default value helpers
// ============================================================================

fn default_root_path() -> String {
    "/".to_string()
}

fn default_search_max() -> usize {
    20
}

fn default_issue_max() -> usize {
    10
}

fn default_comments_max() -> usize {
    50
}

fn default_pr_max() -> usize {
    10
}

fn default_commits_max() -> usize {
    20
}

fn default_releases_max() -> usize {
    10
}

fn default_contributors_max() -> usize {
    30
}

fn default_deps_max() -> usize {
    100
}

fn default_tree_depth() -> usize {
    0 // unlimited
}

fn default_tree_max() -> usize {
    500
}

// ============================================================================
// Parameter Structs
// ============================================================================

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindRepoParams {
    /// Direct repository as 'owner/name' (e.g., 'rust-lang/rust').
    #[serde(default)]
    pub repo: Option<String>,
    /// Search query to find repositories (only if repo name is unknown).
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListDirParams {
    /// Directory path relative to repo root. Use "/" for root.
    #[serde(default = "default_root_path")]
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadFileParams {
    /// File path relative to repo root.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadFilesParams {
    /// List of file paths to read in parallel.
    pub paths: Vec<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchCodeParams {
    /// Code pattern to search for (literal code, not natural language).
    pub query: String,
    /// Maximum results to return.
    #[serde(default = "default_search_max")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SwitchBranchParams {
    /// Branch name to switch to, or empty to list available branches.
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchIssuesParams {
    /// Filter by state: "open", "closed", or "all".
    #[serde(default)]
    pub state: Option<String>,
    /// Filter by labels.
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    /// Filter by assignee username.
    #[serde(default)]
    pub assignee: Option<String>,
    /// Filter by issue creator username.
    #[serde(default)]
    pub creator: Option<String>,
    /// Search text in issue title and body.
    #[serde(default)]
    pub query: Option<String>,
    /// Maximum results to return.
    #[serde(default = "default_issue_max")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetIssueParams {
    /// Issue number to retrieve.
    pub issue_number: u64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListIssueCommentsParams {
    /// Issue number.
    pub issue_number: u64,
    /// Maximum comments to retrieve.
    #[serde(default = "default_comments_max")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchPRsParams {
    /// Filter by state: "open", "closed", "merged", or "all".
    #[serde(default)]
    pub state: Option<String>,
    /// Filter by author username.
    #[serde(default)]
    pub author: Option<String>,
    /// Search text in PR title and body.
    #[serde(default)]
    pub query: Option<String>,
    /// Maximum results to return.
    #[serde(default = "default_pr_max")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPRParams {
    /// Pull request number to retrieve.
    pub pr_number: u64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListPRCommentsParams {
    /// Pull request number.
    pub pr_number: u64,
    /// Maximum comments/reviews to retrieve.
    #[serde(default = "default_comments_max")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListCommitsParams {
    /// Branch, tag, or SHA to list commits from.
    #[serde(default)]
    pub sha: Option<String>,
    /// Filter by author username or email.
    #[serde(default)]
    pub author: Option<String>,
    /// Show commits after this date (ISO 8601).
    #[serde(default)]
    pub since: Option<String>,
    /// Show commits before this date (ISO 8601).
    #[serde(default)]
    pub until: Option<String>,
    /// Filter commits that touch this file path.
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum results to return.
    #[serde(default = "default_commits_max")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetCommitParams {
    /// Full SHA or short SHA of the commit.
    pub sha: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CompareCommitsParams {
    /// Base ref (branch, tag, or SHA).
    pub base: String,
    /// Head ref (branch, tag, or SHA).
    pub head: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BlameFileParams {
    /// File path to get blame information for.
    pub path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListReleasesParams {
    /// Maximum releases to retrieve.
    #[serde(default = "default_releases_max")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetReleaseParams {
    /// Release tag name (e.g., 'v1.0.0') or 'latest'.
    pub tag: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CompareReleasesParams {
    /// Earlier release tag.
    pub from_tag: String,
    /// Later release tag.
    pub to_tag: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetContributorsParams {
    /// Maximum contributors to retrieve.
    #[serde(default = "default_contributors_max")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetRepoStatsParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetDependencyGraphParams {
    /// Maximum dependencies to retrieve.
    #[serde(default = "default_deps_max")]
    pub max_results: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTreeParams {
    /// Directory path to start from. Use "/" or "" for root.
    #[serde(default = "default_root_path")]
    pub path: String,
    /// Maximum depth to traverse (1 = immediate children only, 0 = unlimited).
    #[serde(default = "default_tree_depth")]
    pub depth: usize,
    /// Maximum number of entries to return.
    #[serde(default = "default_tree_max")]
    pub max_entries: usize,
}

// ============================================================================
// Tool Router Implementation
// ============================================================================

#[tool_router]
impl GitCtxMcpServer {
    /// Create a new MCP server instance with an optional GitHub token.
    ///
    /// # Arguments
    ///
    /// * `token` - Optional GitHub personal access token for API authentication.
    ///             Without a token, only public repository access is available
    ///             and rate limits are significantly lower (60 req/hr vs 5000 req/hr).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitctx::mcp::GitCtxMcpServer;
    ///
    /// // With authentication
    /// let server = GitCtxMcpServer::new(Some("ghp_xxx".to_string()));
    ///
    /// // Without authentication (public repos only)
    /// let server = GitCtxMcpServer::new(None);
    /// ```
    pub fn new(token: Option<String>) -> Self {
        let context = GitHubContext::new(token.clone());

        // Initialize shared client if token available
        if let Ok(gh_client) = client::create_client(token.as_deref()) {
            context.set_client(std::sync::Arc::new(gh_client));
        }

        Self {
            context,
            tool_router: Self::tool_router(),
        }
    }

    /// Create a server with authentication from environment variables or config file.
    ///
    /// Token resolution order:
    /// 1. `GITHUB_TOKEN` environment variable
    /// 2. `GH_TOKEN` environment variable (GitHub CLI compatibility)
    /// 3. `~/.config/gitctx/token.json` config file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitctx::mcp::GitCtxMcpServer;
    ///
    /// // Uses GITHUB_TOKEN from environment if available
    /// let server = GitCtxMcpServer::from_env();
    /// ```
    pub fn from_env() -> Self {
        let token = auth::get_token().ok().flatten();
        Self::new(token)
    }

    /// Find and select a GitHub repository. MUST be called first before using other tools.
    #[tool(description = "Find and select a GitHub repository. MUST be called first. Use 'repo' parameter directly if you know the repo name (e.g., 'rust-lang/rust'). Only use 'query' for searching when the repo name is unknown.")]
    pub async fn find_repo(&self, Parameters(params): Parameters<FindRepoParams>) -> Result<CallToolResult, McpError> {
        let token = self.context.get_token();

        // If explicit repo provided
        if let Some(repo_ref) = params.repo.as_ref().filter(|r| !r.trim().is_empty()) {
            let (owner, name) = search::parse_repo_reference(repo_ref)
                .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

            let gh_client = self.context.get_client()
                .map(|c| (*c).clone())
                .or_else(|| client::create_client(token.as_deref()).ok())
                .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

            let info = client::get_repo_info(&gh_client, &owner, &name)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

            self.context.set_repo(
                &info.owner,
                &info.name,
                &info.default_branch,
                info.description.clone(),
                info.is_private,
            );

            let result = serde_json::json!({
                "success": true,
                "selected_repo": {
                    "owner": info.owner,
                    "name": info.name,
                    "default_branch": info.default_branch,
                    "description": info.description
                },
                "message": "Repository selected successfully."
            });

            return Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]));
        }

        // Search for repositories
        let query = params.query
            .as_ref()
            .filter(|q| !q.trim().is_empty())
            .ok_or_else(|| McpError::invalid_params("Provide either 'repo' (owner/name) or 'query' to search.", None))?;

        let results = search::search_repositories(token.as_deref(), query, 5)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if results.is_empty() {
            return Err(McpError::invalid_params(format!("No repositories found matching '{}'.", query), None));
        }

        // Auto-select top result
        let top = &results[0];
        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let info = client::get_repo_info(&gh_client, &top.owner, &top.name)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        self.context.set_repo(
            &info.owner,
            &info.name,
            &info.default_branch,
            info.description.clone(),
            info.is_private,
        );

        let result = serde_json::json!({
            "success": true,
            "selected_repo": {
                "owner": info.owner,
                "name": info.name,
                "default_branch": info.default_branch,
                "description": info.description
            },
            "candidates": results.iter().map(|r| serde_json::json!({
                "full_name": r.full_name(),
                "description": r.description,
                "stars": r.stars,
                "language": r.language
            })).collect::<Vec<_>>(),
            "message": format!("Auto-selected: {}/{}. Use 'repo' parameter to select a different one.", info.owner, info.name)
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// List files and directories at a path in the repository.
    #[tool(description = "List files and directories at a path in the repository. Use this to explore the repository structure.")]
    pub async fn list_dir(&self, Parameters(params): Parameters<ListDirParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let branch = self.context.get_current_branch();
        let token = self.context.get_token();

        // Check cache
        let cache_key = dir_cache_key(&repo.owner, &repo.name, &branch, &params.path);
        if let Some(cached) = DIR_CACHE.get(&cache_key) {
            return Ok(CallToolResult::success(vec![Content::text(cached)]));
        }

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let entries = client::list_directory(&gh_client, &repo.owner, &repo.name, &branch, &params.path)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "path": params.path,
            "entry_count": entries.len(),
            "entries": entries.iter().map(|e| serde_json::json!({
                "name": e.name,
                "path": e.path,
                "type": if e.is_dir { "dir" } else { "file" },
                "size": e.size
            })).collect::<Vec<_>>()
        });

        let xml_result = xml_format::to_xml(&result);
        DIR_CACHE.insert(cache_key, xml_result.clone());
        Ok(CallToolResult::success(vec![Content::text(xml_result)]))
    }

    /// Read the contents of a file from the repository.
    #[tool(description = "Read the contents of a file from the repository. Files larger than 500KB will be rejected.")]
    pub async fn read_file(&self, Parameters(params): Parameters<ReadFileParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let branch = self.context.get_current_branch();
        let token = self.context.get_token();

        // Check cache
        let cache_key = file_cache_key(&repo.owner, &repo.name, &branch, &params.path);
        if let Some(cached) = FILE_CACHE.get(&cache_key) {
            let result = serde_json::json!({
                "success": true,
                "path": params.path,
                "content": cached,
                "size": cached.len()
            });
            return Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]));
        }

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let file = client::read_file(&gh_client, &repo.owner, &repo.name, &branch, &params.path, client::MAX_FILE_SIZE)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if let Some(ref content) = file.content {
            FILE_CACHE.insert(cache_key, content.clone());
        }

        if let Some(error) = file.error {
            return Err(McpError::internal_error(error, None));
        }

        let result = serde_json::json!({
            "success": true,
            "path": file.path,
            "content": file.content,
            "size": file.size,
            "truncated": file.truncated
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Read multiple files in parallel.
    #[tool(description = "Read multiple files in parallel. More efficient than multiple read_file calls.")]
    pub async fn read_files(&self, Parameters(params): Parameters<ReadFilesParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let branch = self.context.get_current_branch();
        let token = self.context.get_token();

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let mut results = Vec::new();

        for path in &params.paths {
            let cache_key = file_cache_key(&repo.owner, &repo.name, &branch, path);
            if let Some(cached) = FILE_CACHE.get(&cache_key) {
                results.push(serde_json::json!({
                    "path": path,
                    "success": true,
                    "content": cached,
                    "size": cached.len()
                }));
                continue;
            }

            match client::read_file(&gh_client, &repo.owner, &repo.name, &branch, path, client::MAX_FILE_SIZE).await {
                Ok(file) => {
                    if let Some(ref content) = file.content {
                        FILE_CACHE.insert(cache_key, content.clone());
                    }
                    results.push(serde_json::json!({
                        "path": file.path,
                        "success": file.error.is_none(),
                        "content": file.content,
                        "size": file.size,
                        "error": file.error
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "path": path,
                        "success": false,
                        "error": e.to_string()
                    }));
                }
            }
        }

        let result = serde_json::json!({
            "success": true,
            "files": results,
            "total_requested": params.paths.len()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Search for code patterns in the repository.
    #[tool(description = "Search for CODE PATTERNS in the repository (NOT natural language). Use function names, class names, or literals.")]
    pub async fn search_code(&self, Parameters(params): Parameters<SearchCodeParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();
        let max_results = params.max_results.min(client::MAX_SEARCH_RESULTS);

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let results = client::search_code(&gh_client, &repo.owner, &repo.name, &params.query, max_results)
            .await
            .map_err(|e| {
                if e.to_string().contains("authentication") {
                    McpError::invalid_params("Code search requires authentication. Set GITHUB_TOKEN.", None)
                } else {
                    McpError::internal_error(e.to_string(), None)
                }
            })?;

        let result = serde_json::json!({
            "success": true,
            "query": params.query,
            "result_count": results.len(),
            "results": results.iter().map(|r| serde_json::json!({
                "file": r.name,
                "path": r.path,
                "matches": r.matches
            })).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Switch to a different branch in the repository.
    #[tool(description = "Switch to a different branch. Call with no branch parameter to list available branches.")]
    pub async fn switch_branch(&self, Parameters(params): Parameters<SwitchBranchParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let branches = client::list_branches(&gh_client, &repo.owner, &repo.name)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if params.branch.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            let result = serde_json::json!({
                "success": true,
                "current_branch": self.context.get_current_branch(),
                "available_branches": branches.iter().map(|b| &b.name).collect::<Vec<_>>()
            });
            return Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]));
        }

        let target_branch = params.branch.unwrap();

        if !branches.iter().any(|b| b.name == target_branch) {
            return Err(McpError::invalid_params(
                format!("Branch '{}' not found. Available: {}", target_branch, branches.iter().map(|b| b.name.as_str()).collect::<Vec<_>>().join(", ")),
                None,
            ));
        }

        let old_branch = self.context.get_current_branch();
        self.context.set_current_branch(&target_branch);

        let result = serde_json::json!({
            "success": true,
            "previous_branch": old_branch,
            "current_branch": target_branch,
            "message": format!("Switched from '{}' to '{}'", old_branch, target_branch)
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Search for issues in the repository with filters.
    #[tool(description = "Search for issues in the repository. Filter by state (open/closed), labels, assignees, creator, or search text.")]
    pub async fn search_issues(&self, Parameters(params): Parameters<SearchIssuesParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();
        let max_results = params.max_results.min(issues::MAX_ISSUE_RESULTS);

        let filters = IssueSearchFilters {
            state: params.state.clone(),
            labels: params.labels.clone().filter(|v| !v.is_empty()),
            assignee: params.assignee.clone().filter(|s| !s.is_empty()),
            creator: params.creator.clone().filter(|s| !s.is_empty()),
            query: params.query.clone().filter(|s| !s.is_empty()),
        };

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let issues_list = issues::search_issues(&gh_client, &repo.owner, &repo.name, &filters, max_results)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "total_count": issues_list.len(),
            "issues": issues_list.iter().map(|i| serde_json::json!({
                "number": i.number,
                "title": i.title,
                "state": i.state,
                "author": i.author,
                "labels": i.labels,
                "assignees": i.assignees,
                "comments_count": i.comments_count,
                "created_at": i.created_at,
                "updated_at": i.updated_at
            })).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Get detailed information about a specific issue.
    #[tool(description = "Get detailed information about a specific issue including title, body, labels, and metadata.")]
    pub async fn get_issue(&self, Parameters(params): Parameters<GetIssueParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let issue = issues::get_issue(&gh_client, &repo.owner, &repo.name, params.issue_number)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    McpError::invalid_params(format!("Issue #{} not found in {}/{}", params.issue_number, repo.owner, repo.name), None)
                } else {
                    McpError::internal_error(e.to_string(), None)
                }
            })?;

        let result = serde_json::json!({
            "success": true,
            "issue": {
                "number": issue.number,
                "title": issue.title,
                "body": issue.body,
                "state": issue.state,
                "author": issue.author,
                "labels": issue.labels,
                "assignees": issue.assignees,
                "comments_count": issue.comments_count,
                "created_at": issue.created_at,
                "updated_at": issue.updated_at,
                "closed_at": issue.closed_at,
                "url": issue.url
            }
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// List comments on an issue.
    #[tool(description = "List comments on an issue, sorted chronologically.")]
    pub async fn list_issue_comments(&self, Parameters(params): Parameters<ListIssueCommentsParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();
        let per_page = (params.max_results as u32).min(100);

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let (comments, _has_more) = issues::list_issue_comments(&gh_client, &repo.owner, &repo.name, params.issue_number, None, Some(per_page))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "issue_number": params.issue_number,
            "comment_count": comments.len(),
            "comments": comments.iter().map(|c| serde_json::json!({
                "id": c.id,
                "author": c.author,
                "body": c.body,
                "created_at": c.created_at,
                "updated_at": c.updated_at
            })).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Search for pull requests in the repository.
    #[tool(description = "Search for pull requests. Filter by state (open/closed/merged), author, or search text.")]
    pub async fn search_prs(&self, Parameters(params): Parameters<SearchPRsParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();
        let max_results = params.max_results.min(pulls::MAX_PR_RESULTS);

        let filters = pulls::PRSearchFilters {
            state: params.state.clone(),
            author: params.author.clone().filter(|s| !s.is_empty()),
            query: params.query.clone().filter(|s| !s.is_empty()),
        };

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let prs = pulls::search_prs(&gh_client, &repo.owner, &repo.name, &filters, max_results)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "total_count": prs.len(),
            "pull_requests": prs.iter().map(|pr| serde_json::json!({
                "number": pr.number,
                "title": pr.title,
                "state": pr.state,
                "merged": pr.merged,
                "author": pr.author,
                "head_branch": pr.head_branch,
                "base_branch": pr.base_branch,
                "created_at": pr.created_at,
                "updated_at": pr.updated_at
            })).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Get detailed information about a specific pull request.
    #[tool(description = "Get detailed information about a specific pull request including branches, merge status, and file changes.")]
    pub async fn get_pr(&self, Parameters(params): Parameters<GetPRParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let pr = pulls::get_pr(&gh_client, &repo.owner, &repo.name, params.pr_number)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    McpError::invalid_params(format!("PR #{} not found in {}/{}", params.pr_number, repo.owner, repo.name), None)
                } else {
                    McpError::internal_error(e.to_string(), None)
                }
            })?;

        let result = serde_json::json!({
            "success": true,
            "pr": {
                "number": pr.number,
                "title": pr.title,
                "body": pr.body,
                "state": pr.state,
                "merged": pr.merged,
                "author": pr.author,
                "head_branch": pr.head_branch,
                "base_branch": pr.base_branch,
                "head_sha": pr.head_sha,
                "labels": pr.labels,
                "assignees": pr.assignees,
                "reviewers": pr.reviewers,
                "created_at": pr.created_at,
                "updated_at": pr.updated_at,
                "merged_at": pr.merged_at,
                "closed_at": pr.closed_at,
                "comments_count": pr.comments_count,
                "review_comments_count": pr.review_comments_count,
                "commits_count": pr.commits_count,
                "additions": pr.additions,
                "deletions": pr.deletions,
                "changed_files": pr.changed_files,
                "mergeable": pr.mergeable,
                "mergeable_state": pr.mergeable_state,
                "url": pr.url
            }
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// List comments and reviews on a pull request.
    #[tool(description = "List comments and reviews on a pull request.")]
    pub async fn list_pr_comments(&self, Parameters(params): Parameters<ListPRCommentsParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();
        let per_page = (params.max_results as u32).min(100);

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let (comments, _has_more) = pulls::list_pr_comments(&gh_client, &repo.owner, &repo.name, params.pr_number, None, Some(per_page))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "pr_number": params.pr_number,
            "comment_count": comments.len(),
            "comments": comments.iter().map(|c| serde_json::json!({
                "id": c.id,
                "author": c.author,
                "body": c.body,
                "created_at": c.created_at,
                "updated_at": c.updated_at
            })).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// List commits with optional filters.
    #[tool(description = "List commits in the repository with optional filters for author, date range, and file path.")]
    pub async fn list_commits(&self, Parameters(params): Parameters<ListCommitsParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();
        let max_results = params.max_results.min(100);

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let filters = commits::CommitFilters {
            sha: params.sha.clone().or_else(|| Some(self.context.get_current_branch())),
            author: params.author.clone(),
            since: params.since.clone(),
            until: params.until.clone(),
            path: params.path.clone(),
        };

        let commits_list = commits::list_commits(&gh_client, &repo.owner, &repo.name, &filters, max_results)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "commit_count": commits_list.len(),
            "commits": commits_list.iter().map(|c| serde_json::json!({
                "sha": c.sha,
                "short_sha": c.short_sha,
                "message": c.message,
                "author_name": c.author_name,
                "author_email": c.author_email,
                "author_login": c.author_login,
                "date": c.date,
                "url": c.url
            })).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Get detailed information about a specific commit.
    #[tool(description = "Get detailed information about a specific commit including message, author, and file changes.")]
    pub async fn get_commit(&self, Parameters(params): Parameters<GetCommitParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let commit = commits::get_commit(&gh_client, &repo.owner, &repo.name, &params.sha)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") {
                    McpError::invalid_params(format!("Commit '{}' not found", params.sha), None)
                } else {
                    McpError::internal_error(e.to_string(), None)
                }
            })?;

        let result = serde_json::json!({
            "success": true,
            "commit": {
                "sha": commit.commit.sha,
                "short_sha": commit.commit.short_sha,
                "message": commit.commit.message,
                "author_name": commit.commit.author_name,
                "author_email": commit.commit.author_email,
                "author_login": commit.commit.author_login,
                "date": commit.commit.date,
                "parents": commit.commit.parents,
                "verified": commit.commit.verified,
                "stats": {
                    "additions": commit.stats.additions,
                    "deletions": commit.stats.deletions,
                    "total": commit.stats.total
                },
                "files": commit.files.iter().map(|f| serde_json::json!({
                    "filename": f.filename,
                    "status": f.status,
                    "additions": f.additions,
                    "deletions": f.deletions,
                    "patch": f.patch
                })).collect::<Vec<_>>(),
                "url": commit.commit.url
            }
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Compare two commits, branches, or tags.
    #[tool(description = "Compare two commits, branches, or tags to see the differences.")]
    pub async fn compare_commits(&self, Parameters(params): Parameters<CompareCommitsParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let comparison = commits::compare_commits(&gh_client, &repo.owner, &repo.name, &params.base, &params.head)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "comparison": {
                "base": params.base,
                "head": params.head,
                "status": comparison.status,
                "ahead_by": comparison.ahead_by,
                "behind_by": comparison.behind_by,
                "total_commits": comparison.total_commits,
                "commits": comparison.commits.iter().map(|c| serde_json::json!({
                    "sha": c.sha,
                    "short_sha": c.short_sha,
                    "message": c.message,
                    "author_name": c.author_name,
                    "date": c.date
                })).collect::<Vec<_>>(),
                "files": comparison.files.iter().map(|f| serde_json::json!({
                    "filename": f.filename,
                    "status": f.status,
                    "additions": f.additions,
                    "deletions": f.deletions
                })).collect::<Vec<_>>()
            }
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Get line-by-line blame information for a file.
    #[tool(description = "Get line-by-line blame information for a file, showing who last modified each line.")]
    pub async fn blame_file(&self, Parameters(params): Parameters<BlameFileParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();
        let branch = self.context.get_current_branch();

        // get_blame uses GraphQL and takes token directly
        let blame = commits::get_blame(token.as_deref(), &repo.owner, &repo.name, &branch, &params.path, None, None)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "path": blame.path,
            "branch": blame.branch,
            "total_lines": blame.total_lines,
            "unique_commits": blame.unique_commits,
            "lines": blame.lines.iter().take(200).map(|l| serde_json::json!({
                "line_number": l.line_number,
                "content": l.content,
                "commit_sha": l.short_sha,
                "author": l.author,
                "date": l.date,
                "message": l.message
            })).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// List releases in the repository.
    #[tool(description = "List releases in the repository, ordered by creation date.")]
    pub async fn list_releases(&self, Parameters(params): Parameters<ListReleasesParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();
        let max_results = params.max_results.min(100);

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        // include_drafts=false - only show published releases
        let releases_list = releases::list_releases(&gh_client, &repo.owner, &repo.name, false, max_results)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "release_count": releases_list.len(),
            "releases": releases_list.iter().map(|r| serde_json::json!({
                "tag_name": r.tag_name,
                "name": r.name,
                "draft": r.draft,
                "prerelease": r.prerelease,
                "created_at": r.created_at,
                "published_at": r.published_at,
                "author": r.author
            })).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Get detailed information about a specific release.
    #[tool(description = "Get detailed information about a specific release including release notes and assets.")]
    pub async fn get_release(&self, Parameters(params): Parameters<GetReleaseParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let release = if params.tag == "latest" {
            releases::get_latest_release(&gh_client, &repo.owner, &repo.name).await
        } else {
            releases::get_release(&gh_client, &repo.owner, &repo.name, &params.tag).await
        }.map_err(|e| {
            if e.to_string().contains("404") {
                McpError::invalid_params(format!("Release '{}' not found", params.tag), None)
            } else {
                McpError::internal_error(e.to_string(), None)
            }
        })?;

        let result = serde_json::json!({
            "success": true,
            "release": {
                "tag_name": release.tag_name,
                "name": release.name,
                "body": release.body,
                "draft": release.draft,
                "prerelease": release.prerelease,
                "created_at": release.created_at,
                "published_at": release.published_at,
                "author": release.author,
                "tarball_url": release.tarball_url,
                "zipball_url": release.zipball_url,
                "assets": release.assets.iter().map(|a| serde_json::json!({
                    "name": a.name,
                    "size": a.size,
                    "download_count": a.download_count,
                    "download_url": a.download_url
                })).collect::<Vec<_>>(),
                "url": release.url
            }
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Compare changes between two releases.
    #[tool(description = "Compare changes between two releases to see what changed.")]
    pub async fn compare_releases(&self, Parameters(params): Parameters<CompareReleasesParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        // Compare releases by comparing their tags using commit comparison
        let comparison = commits::compare_commits(&gh_client, &repo.owner, &repo.name, &params.from_tag, &params.to_tag)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "comparison": {
                "from_tag": params.from_tag,
                "to_tag": params.to_tag,
                "status": comparison.status,
                "ahead_by": comparison.ahead_by,
                "behind_by": comparison.behind_by,
                "total_commits": comparison.total_commits,
                "commits": comparison.commits.iter().map(|c| serde_json::json!({
                    "sha": c.sha,
                    "short_sha": c.short_sha,
                    "message": c.message,
                    "author_name": c.author_name,
                    "date": c.date
                })).collect::<Vec<_>>(),
                "files_changed": comparison.files.len()
            }
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Get top contributors to the repository.
    #[tool(description = "Get top contributors to the repository sorted by number of commits.")]
    pub async fn get_contributors(&self, Parameters(params): Parameters<GetContributorsParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();
        let max_results = params.max_results.min(100);

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let contributors = stats::get_contributors(&gh_client, &repo.owner, &repo.name, max_results)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "contributor_count": contributors.len(),
            "contributors": contributors.iter().map(|c| serde_json::json!({
                "login": c.login,
                "contributions": c.contributions,
                "avatar_url": c.avatar_url,
                "profile_url": c.profile_url
            })).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Get repository statistics.
    #[tool(description = "Get repository statistics including stars, forks, watchers, languages, and topics.")]
    pub async fn get_repo_stats(&self, Parameters(_params): Parameters<GetRepoStatsParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let repo_stats = stats::get_repo_stats(&gh_client, &repo.owner, &repo.name)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "stats": {
                "description": repo_stats.description,
                "stars": repo_stats.stars,
                "forks": repo_stats.forks,
                "watchers": repo_stats.watchers,
                "open_issues": repo_stats.open_issues,
                "default_branch": repo_stats.default_branch,
                "primary_language": repo_stats.primary_language,
                "languages": repo_stats.languages.iter().map(|l| serde_json::json!({
                    "name": l.name,
                    "bytes": l.bytes,
                    "percentage": l.percentage
                })).collect::<Vec<_>>(),
                "topics": repo_stats.topics,
                "license": repo_stats.license,
                "archived": repo_stats.archived,
                "is_fork": repo_stats.is_fork,
                "created_at": repo_stats.created_at,
                "updated_at": repo_stats.updated_at,
                "pushed_at": repo_stats.pushed_at,
                "size_kb": repo_stats.size_kb
            }
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Get dependency information from package manifest files.
    #[tool(description = "Get dependency information from package manifest files (package.json, Cargo.toml, etc.).")]
    pub async fn get_dependency_graph(&self, Parameters(params): Parameters<GetDependencyGraphParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let token = self.context.get_token();
        let max_results = params.max_results.min(stats::MAX_DEPENDENCY_RESULTS);

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let deps = stats::get_dependency_graph(&gh_client, &repo.owner, &repo.name, None, max_results)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let result = serde_json::json!({
            "success": true,
            "sbom_version": deps.sbom_version,
            "created_at": deps.created_at,
            "total_count": deps.total_count,
            "direct_count": deps.direct_count,
            "indirect_count": deps.indirect_count,
            "dependencies": deps.dependencies.iter().map(|d| serde_json::json!({
                "name": d.name,
                "version": d.version,
                "package_url": d.package_url,
                "scope": d.scope,
                "relationship": d.relationship
            })).collect::<Vec<_>>()
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }

    /// Get a tree view of the repository structure.
    #[tool(description = "Get a tree view of the repository structure. More efficient than multiple list_dir calls. Returns a tree-style output similar to the Unix tree command.")]
    pub async fn get_tree(&self, Parameters(params): Parameters<GetTreeParams>) -> Result<CallToolResult, McpError> {
        let repo = self.context.get_repo()
            .ok_or_else(|| McpError::invalid_params("No repository selected. Call find_repo first.", None))?;

        let branch = self.context.get_current_branch();
        let token = self.context.get_token();

        let gh_client = self.context.get_client()
            .map(|c| (*c).clone())
            .or_else(|| client::create_client(token.as_deref()).ok())
            .ok_or_else(|| McpError::internal_error("Failed to create GitHub client", None))?;

        let tree_result = client::get_tree(
            &gh_client,
            &repo.owner,
            &repo.name,
            &branch,
            &params.path,
            params.depth,
            params.max_entries,
        )
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // Format as tree string
        let tree_string = format_tree_output(&tree_result.entries, &repo.name, &params.path);

        let result = serde_json::json!({
            "success": true,
            "path": params.path,
            "depth": params.depth,
            "total_entries": tree_result.total_count,
            "truncated": tree_result.truncated,
            "tree": tree_string,
            "summary": {
                "directories": tree_result.dir_count,
                "files": tree_result.file_count
            }
        });

        Ok(CallToolResult::success(vec![Content::text(xml_format::to_xml(&result))]))
    }
}

// ============================================================================
// Tree Formatting Helper
// ============================================================================

/// Format tree entries as a tree-style string output.
///
/// Produces output similar to the Unix `tree` command with box-drawing characters.
fn format_tree_output(entries: &[client::TreeEntry], repo_name: &str, base_path: &str) -> String {
    use std::collections::HashMap;

    if entries.is_empty() {
        return String::from("(empty)");
    }

    // Build a tree structure from flat paths
    #[derive(Default)]
    struct TreeNode {
        children: HashMap<String, TreeNode>,
        is_dir: bool,
    }

    let mut root = TreeNode {
        is_dir: true,
        ..Default::default()
    };

    // Normalize base path
    let base = base_path.trim_start_matches('/').trim_end_matches('/');

    for entry in entries {
        // Get the path relative to base
        let relative_path = if base.is_empty() {
            entry.path.as_str()
        } else if entry.path.starts_with(&format!("{}/", base)) {
            &entry.path[base.len() + 1..]
        } else if entry.path == base {
            continue; // Skip the base directory itself
        } else {
            continue;
        };

        let parts: Vec<&str> = relative_path.split('/').collect();
        let mut current = &mut root;

        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            current = current.children.entry(part.to_string()).or_default();
            if is_last {
                current.is_dir = entry.entry_type == "tree";
            } else {
                current.is_dir = true;
            }
        }
    }

    // Format the tree with box-drawing characters
    fn format_node(
        node: &TreeNode,
        name: &str,
        prefix: &str,
        is_last: bool,
        is_root: bool,
        output: &mut String,
    ) {
        if is_root {
            output.push_str(name);
            if node.is_dir {
                output.push('/');
            }
            output.push('\n');
        } else {
            output.push_str(prefix);
            output.push_str(if is_last { "└── " } else { "├── " });
            output.push_str(name);
            if node.is_dir {
                output.push('/');
            }
            output.push('\n');
        }

        let new_prefix = if is_root {
            String::new()
        } else {
            format!("{}{}", prefix, if is_last { "    " } else { "│   " })
        };

        // Sort children: directories first, then files, alphabetically within each group
        let mut children: Vec<_> = node.children.iter().collect();
        children.sort_by(|(a_name, a_node), (b_name, b_node)| {
            match (a_node.is_dir, b_node.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a_name.cmp(b_name),
            }
        });

        let child_count = children.len();
        for (i, (child_name, child_node)) in children.into_iter().enumerate() {
            let child_is_last = i == child_count - 1;
            format_node(child_node, child_name, &new_prefix, child_is_last, false, output);
        }
    }

    let mut output = String::new();
    let root_name = if base.is_empty() {
        repo_name.to_string()
    } else {
        base.rsplit('/').next().unwrap_or(base).to_string()
    };

    format_node(&root, &root_name, "", true, true, &mut output);

    output
}
