//! GitHub Commits API client functions.
//!
//! This module provides functions for interacting with GitHub's Commits API
//! to list commits, get commit details, compare refs, and get blame information.
//!
//! # API Endpoints Used
//!
//! - `GET /repos/{owner}/{repo}/commits` - List repository commits
//! - `GET /repos/{owner}/{repo}/commits/{ref}` - Get single commit details
//! - `GET /repos/{owner}/{repo}/compare/{base}...{head}` - Compare two refs
//! - GraphQL API for blame information (REST API lacks blame endpoint)

use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};

/// Maximum number of commits to return from list.
pub const MAX_COMMIT_RESULTS: usize = 100;

/// Default number of commits to return.
pub const DEFAULT_COMMIT_RESULTS: usize = 20;

/// Commit information returned from GitHub API.
///
/// Contains metadata about a commit including author, message, and SHA.
#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    /// Full SHA of the commit.
    pub sha: String,
    /// Short SHA (first 7 characters).
    pub short_sha: String,
    /// Commit message (may be multi-line).
    pub message: String,
    /// Author's name from git config.
    pub author_name: String,
    /// Author's email from git config.
    pub author_email: String,
    /// GitHub username of the author (if linked).
    pub author_login: Option<String>,
    /// Committer's name from git config.
    pub committer_name: String,
    /// Committer's email from git config.
    pub committer_email: String,
    /// ISO 8601 timestamp when the commit was authored.
    pub date: String,
    /// GitHub HTML URL for the commit.
    pub url: String,
    /// SHA(s) of parent commit(s).
    pub parents: Vec<String>,
    /// Whether the commit signature was verified.
    pub verified: bool,
}

/// Detailed commit information with file changes.
///
/// Includes the commit metadata plus statistics and file-level changes.
#[derive(Debug, Clone, Serialize)]
pub struct CommitDetails {
    /// Basic commit information.
    pub commit: CommitInfo,
    /// Aggregate statistics for the commit.
    pub stats: CommitStats,
    /// List of files changed in this commit.
    pub files: Vec<CommitFile>,
}

/// Commit statistics showing lines added/deleted.
#[derive(Debug, Clone, Serialize)]
pub struct CommitStats {
    /// Number of lines added.
    pub additions: u32,
    /// Number of lines deleted.
    pub deletions: u32,
    /// Total lines changed (additions + deletions).
    pub total: u32,
}

/// File changed in a commit.
#[derive(Debug, Clone, Serialize)]
pub struct CommitFile {
    /// Path to the file.
    pub filename: String,
    /// Change status: "added", "modified", "removed", "renamed".
    pub status: String,
    /// Lines added to this file.
    pub additions: u32,
    /// Lines deleted from this file.
    pub deletions: u32,
    /// Total changes in this file.
    pub changes: u32,
    /// Patch/diff content (may be None for large files).
    pub patch: Option<String>,
    /// Previous filename if file was renamed.
    pub previous_filename: Option<String>,
}

/// Result of comparing two commits/refs.
#[derive(Debug, Clone, Serialize)]
pub struct CompareResult {
    /// SHA of the base commit.
    pub base_commit: String,
    /// SHA of the head commit.
    pub head_commit: String,
    /// Comparison status: "ahead", "behind", "diverged", "identical".
    pub status: String,
    /// Number of commits head is ahead of base.
    pub ahead_by: u32,
    /// Number of commits head is behind base.
    pub behind_by: u32,
    /// Total number of commits in the comparison.
    pub total_commits: u32,
    /// List of commits between base and head.
    pub commits: Vec<CommitInfo>,
    /// List of files changed.
    pub files: Vec<CommitFile>,
    /// Aggregate statistics for all changes.
    pub stats: CommitStats,
    /// URL to view the comparison on GitHub.
    pub diff_url: String,
}

/// Filters for listing commits.
///
/// All filters are optional and combined with AND logic when multiple are specified.
#[derive(Debug, Clone, Default)]
pub struct CommitFilters {
    /// Branch, tag, or SHA to start listing from.
    pub sha: Option<String>,
    /// Filter commits by author username or email.
    pub author: Option<String>,
    /// Only show commits after this date (ISO 8601).
    pub since: Option<String>,
    /// Only show commits before this date (ISO 8601).
    pub until: Option<String>,
    /// Filter commits that touch this file path.
    pub path: Option<String>,
}

/// Blame information for a file.
#[derive(Debug, Clone, Serialize)]
pub struct BlameInfo {
    /// Path to the file.
    pub path: String,
    /// Branch the blame was computed on.
    pub branch: String,
    /// Blame information for each line.
    pub lines: Vec<BlameLine>,
    /// Total number of lines in the file.
    pub total_lines: u32,
    /// Number of unique commits that touched this file.
    pub unique_commits: u32,
}

/// Blame information for a single line.
#[derive(Debug, Clone, Serialize)]
pub struct BlameLine {
    /// Line number (1-indexed).
    pub line_number: u32,
    /// Content of the line.
    pub content: String,
    /// SHA of the commit that last modified this line.
    pub commit_sha: String,
    /// Short SHA (first 7 characters).
    pub short_sha: String,
    /// Author of the commit.
    pub author: String,
    /// Date of the commit (ISO 8601).
    pub date: String,
    /// First line of the commit message.
    pub message: String,
}

// ============================================================================
// GitHub API Response Structs (private)
// ============================================================================

#[derive(Deserialize)]
struct CommitResponse {
    sha: String,
    commit: CommitData,
    author: Option<UserResponse>,
    #[allow(dead_code)]
    committer: Option<UserResponse>,
    parents: Vec<ParentRef>,
    html_url: String,
    stats: Option<StatsResponse>,
    files: Option<Vec<FileResponse>>,
}

#[derive(Deserialize)]
struct CommitData {
    message: String,
    author: GitUser,
    committer: GitUser,
    verification: Option<VerificationResponse>,
}

#[derive(Deserialize)]
struct GitUser {
    name: String,
    email: String,
    date: String,
}

#[derive(Deserialize)]
struct VerificationResponse {
    verified: bool,
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
}

#[derive(Deserialize)]
struct ParentRef {
    sha: String,
}

#[derive(Deserialize)]
struct StatsResponse {
    additions: u32,
    deletions: u32,
    total: u32,
}

#[derive(Deserialize)]
struct FileResponse {
    filename: String,
    status: String,
    additions: u32,
    deletions: u32,
    changes: u32,
    patch: Option<String>,
    previous_filename: Option<String>,
}

#[derive(Deserialize)]
struct CompareResponse {
    base_commit: CommitResponse,
    merge_base_commit: CommitResponse,
    status: String,
    ahead_by: u32,
    behind_by: u32,
    total_commits: u32,
    commits: Vec<CommitResponse>,
    files: Option<Vec<FileResponse>>,
    html_url: String,
}

#[derive(Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize)]
struct GraphQLData {
    repository: Option<RepositoryData>,
}

#[derive(Deserialize)]
struct RepositoryData {
    #[serde(rename = "ref")]
    git_ref: Option<RefData>,
    object: Option<BlobData>,
}

#[derive(Deserialize)]
struct RefData {
    target: Option<CommitTargetData>,
}

#[derive(Deserialize)]
struct CommitTargetData {
    blame: Option<BlameData>,
}

#[derive(Deserialize)]
struct BlobData {
    text: Option<String>,
}

#[derive(Deserialize)]
struct BlameData {
    ranges: Vec<BlameRange>,
}

#[derive(Deserialize)]
struct BlameRange {
    #[serde(rename = "startingLine")]
    starting_line: u32,
    #[serde(rename = "endingLine")]
    ending_line: u32,
    commit: BlameCommit,
}

#[derive(Deserialize)]
struct BlameCommit {
    oid: String,
    message: String,
    author: BlameAuthor,
}

#[derive(Deserialize)]
struct BlameAuthor {
    name: String,
    date: String,
}

#[derive(Deserialize)]
struct GraphQLError {
    message: String,
}

// ============================================================================
// Public API Functions
// ============================================================================

/// List commits in a repository with optional filters.
///
/// Retrieves commits from the repository, optionally filtered by author,
/// date range, or file path.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `filters` - Optional filters to apply
/// * `max_results` - Maximum number of commits to return (default: 20, max: 100)
///
/// # Returns
///
/// Vector of commit information sorted by date (newest first).
///
/// # Example
///
/// ```no_run
/// use gitctx::github::commits::{list_commits, CommitFilters};
///
/// let filters = CommitFilters {
///     author: Some("octocat".to_string()),
///     since: Some("2024-01-01T00:00:00Z".to_string()),
///     ..Default::default()
/// };
/// let commits = list_commits(&client, "owner", "repo", &filters, 20).await?;
/// ```
pub async fn list_commits(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    filters: &CommitFilters,
    max_results: usize,
) -> Result<Vec<CommitInfo>> {
    let max_results = max_results.min(MAX_COMMIT_RESULTS);

    // Build query parameters
    let mut query_params = vec![format!("per_page={}", max_results)];

    if let Some(ref sha) = filters.sha {
        query_params.push(format!("sha={}", sha));
    }
    if let Some(ref author) = filters.author {
        query_params.push(format!("author={}", urlencoding::encode(author)));
    }
    if let Some(ref since) = filters.since {
        query_params.push(format!("since={}", urlencoding::encode(since)));
    }
    if let Some(ref until) = filters.until {
        query_params.push(format!("until={}", urlencoding::encode(until)));
    }
    if let Some(ref path) = filters.path {
        query_params.push(format!("path={}", urlencoding::encode(path)));
    }

    let query_string = query_params.join("&");
    let endpoint = format!("/repos/{}/{}/commits?{}", owner, repo, query_string);

    let commits: Vec<CommitResponse> = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to list commits: {}", e))?;

    Ok(commits.into_iter().map(commit_response_to_info).collect())
}

/// Get detailed information about a specific commit.
///
/// Retrieves full commit details including file changes and statistics.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `commit_ref` - Commit SHA, branch name, or tag name
///
/// # Returns
///
/// Full commit details including files changed and stats.
///
/// # Errors
///
/// Returns an error if the commit doesn't exist or isn't accessible.
pub async fn get_commit(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    commit_ref: &str,
) -> Result<CommitDetails> {
    let endpoint = format!(
        "/repos/{}/{}/commits/{}",
        owner,
        repo,
        urlencoding::encode(commit_ref)
    );

    let mut commit: CommitResponse = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get commit {}: {}", commit_ref, e))?;

    let stats = commit.stats.as_ref().map_or(
        CommitStats {
            additions: 0,
            deletions: 0,
            total: 0,
        },
        |s| CommitStats {
            additions: s.additions,
            deletions: s.deletions,
            total: s.total,
        },
    );

    // Take ownership of files before converting commit
    let files_data = commit.files.take();
    let commit_info = commit_response_to_info(commit);

    let files = files_data
        .unwrap_or_default()
        .into_iter()
        .map(|f| CommitFile {
            filename: f.filename,
            status: f.status,
            additions: f.additions,
            deletions: f.deletions,
            changes: f.changes,
            patch: f.patch,
            previous_filename: f.previous_filename,
        })
        .collect();

    Ok(CommitDetails {
        commit: commit_info,
        stats,
        files,
    })
}

/// Compare two commits, branches, or tags.
///
/// Shows the commits and file changes between two refs.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `base` - Base ref (commit SHA, branch, or tag)
/// * `head` - Head ref to compare against base
///
/// # Returns
///
/// Comparison result including commits, files changed, and statistics.
///
/// # Example
///
/// ```no_run
/// use gitctx::github::commits::compare_commits;
///
/// // Compare two branches
/// let result = compare_commits(&client, "owner", "repo", "main", "feature-branch").await?;
///
/// // Compare two tags
/// let result = compare_commits(&client, "owner", "repo", "v1.0.0", "v2.0.0").await?;
/// ```
pub async fn compare_commits(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    base: &str,
    head: &str,
) -> Result<CompareResult> {
    let endpoint = format!(
        "/repos/{}/{}/compare/{}...{}",
        owner,
        repo,
        urlencoding::encode(base),
        urlencoding::encode(head)
    );

    let response: CompareResponse = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to compare {} and {}: {}", base, head, e))?;

    let commits: Vec<CommitInfo> = response
        .commits
        .into_iter()
        .map(commit_response_to_info)
        .collect();

    let files: Vec<CommitFile> = response
        .files
        .unwrap_or_default()
        .into_iter()
        .map(|f| CommitFile {
            filename: f.filename,
            status: f.status,
            additions: f.additions,
            deletions: f.deletions,
            changes: f.changes,
            patch: f.patch,
            previous_filename: f.previous_filename,
        })
        .collect();

    // Calculate stats from files
    let (additions, deletions) = files
        .iter()
        .fold((0u32, 0u32), |(a, d), f| (a + f.additions, d + f.deletions));

    Ok(CompareResult {
        base_commit: response.merge_base_commit.sha,
        head_commit: response.base_commit.sha,
        status: response.status,
        ahead_by: response.ahead_by,
        behind_by: response.behind_by,
        total_commits: response.total_commits,
        commits,
        files,
        stats: CommitStats {
            additions,
            deletions,
            total: additions + deletions,
        },
        diff_url: response.html_url,
    })
}

/// Get blame information for a file.
///
/// Uses GitHub's GraphQL API to retrieve line-by-line blame information
/// showing which commit last modified each line.
///
/// # Arguments
///
/// * `token` - GitHub personal access token (required for GraphQL)
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `branch` - Branch to get blame from
/// * `path` - Path to the file
/// * `start_line` - Optional starting line (1-indexed)
/// * `end_line` - Optional ending line
///
/// # Returns
///
/// Blame information for each line in the file or specified range.
///
/// # Errors
///
/// Returns an error if the file doesn't exist, authentication fails,
/// or the GraphQL query fails.
///
/// # Note
///
/// This function requires authentication as it uses the GraphQL API.
/// The REST API does not have a blame endpoint.
pub async fn get_blame(
    token: Option<&str>,
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    start_line: Option<u32>,
    end_line: Option<u32>,
) -> Result<BlameInfo> {
    let token = token.ok_or_else(|| {
        anyhow!("Authentication required for blame. GitHub's GraphQL API requires a token.")
    })?;

    let client = reqwest::Client::new();

    // Build GraphQL query for blame
    // The blame field is on Commit, and file content is fetched from repository.object
    let query = format!(
        r#"query {{
            repository(owner: "{}", name: "{}") {{
                ref(qualifiedName: "refs/heads/{}") {{
                    target {{
                        ... on Commit {{
                            blame(path: "{}") {{
                                ranges {{
                                    startingLine
                                    endingLine
                                    commit {{
                                        oid
                                        message
                                        author {{
                                            name
                                            date
                                        }}
                                    }}
                                }}
                            }}
                        }}
                    }}
                }}
                object(expression: "{}:{}") {{
                    ... on Blob {{
                        text
                    }}
                }}
            }}
        }}"#,
        owner, repo, branch, path, branch, path
    );

    let body = serde_json::json!({ "query": query });

    let response = client
        .post("https://api.github.com/graphql")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(USER_AGENT, "gitctx")
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow!("Failed to send GraphQL request: {}", e))?;

    let graphql_response: GraphQLResponse = response
        .json()
        .await
        .map_err(|e| anyhow!("Failed to parse GraphQL response: {}", e))?;

    // Check for GraphQL errors
    if let Some(errors) = graphql_response.errors {
        let error_messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
        return Err(anyhow!("GraphQL errors: {}", error_messages.join(", ")));
    }

    let repository = graphql_response
        .data
        .and_then(|d| d.repository)
        .ok_or_else(|| anyhow!("Repository {}/{} not found", owner, repo))?;

    let commit_data = repository
        .git_ref
        .and_then(|r| r.target)
        .ok_or_else(|| anyhow!("Branch '{}' not found", branch))?;

    let file_content = repository
        .object
        .and_then(|f| f.text)
        .unwrap_or_default();
    let file_lines: Vec<&str> = file_content.lines().collect();

    let blame_data = commit_data
        .blame
        .ok_or_else(|| anyhow!("Blame information not available for {}", path))?;

    // Build blame lines from ranges
    let mut blame_lines: Vec<BlameLine> = Vec::new();
    let mut unique_commits: std::collections::HashSet<String> = std::collections::HashSet::new();

    for range in blame_data.ranges {
        unique_commits.insert(range.commit.oid.clone());

        let short_sha = range.commit.oid.chars().take(7).collect::<String>();
        let message_first_line = range
            .commit
            .message
            .lines()
            .next()
            .unwrap_or("")
            .to_string();

        for line_num in range.starting_line..=range.ending_line {
            let content = file_lines
                .get((line_num - 1) as usize)
                .unwrap_or(&"")
                .to_string();

            blame_lines.push(BlameLine {
                line_number: line_num,
                content,
                commit_sha: range.commit.oid.clone(),
                short_sha: short_sha.clone(),
                author: range.commit.author.name.clone(),
                date: range.commit.author.date.clone(),
                message: message_first_line.clone(),
            });
        }
    }

    // Sort by line number
    blame_lines.sort_by_key(|l| l.line_number);

    // Filter by line range if specified
    let blame_lines: Vec<BlameLine> = if start_line.is_some() || end_line.is_some() {
        let start = start_line.unwrap_or(1);
        let end = end_line.unwrap_or(u32::MAX);
        blame_lines
            .into_iter()
            .filter(|l| l.line_number >= start && l.line_number <= end)
            .collect()
    } else {
        blame_lines
    };

    Ok(BlameInfo {
        path: path.to_string(),
        branch: branch.to_string(),
        lines: blame_lines.clone(),
        total_lines: blame_lines.len() as u32,
        unique_commits: unique_commits.len() as u32,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a CommitResponse to CommitInfo.
fn commit_response_to_info(commit: CommitResponse) -> CommitInfo {
    CommitInfo {
        sha: commit.sha.clone(),
        short_sha: commit.sha.chars().take(7).collect(),
        message: commit.commit.message,
        author_name: commit.commit.author.name,
        author_email: commit.commit.author.email,
        author_login: commit.author.map(|a| a.login),
        committer_name: commit.commit.committer.name,
        committer_email: commit.commit.committer.email,
        date: commit.commit.author.date,
        url: commit.html_url,
        parents: commit.parents.into_iter().map(|p| p.sha).collect(),
        verified: commit
            .commit
            .verification
            .is_some_and(|v| v.verified),
    }
}
