//! GitHub Issues API client functions.
//!
//! This module provides functions for interacting with GitHub's Issues API
//! to search issues, read issue details, and list comments.
//!
//! # API Endpoints Used
//!
//! - `GET /repos/{owner}/{repo}/issues` - List repository issues
//! - `GET /repos/{owner}/{repo}/issues/{number}` - Get single issue
//! - `GET /repos/{owner}/{repo}/issues/{number}/comments` - List issue comments
//! - `GET /search/issues` - Search issues with advanced filters

use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};

/// Maximum number of issues to return from search.
pub const MAX_ISSUE_RESULTS: usize = 30;

/// Default number of comments per page.
pub const DEFAULT_COMMENTS_PER_PAGE: u32 = 30;

/// Maximum comments per page (GitHub API limit).
pub const MAX_COMMENTS_PER_PAGE: u32 = 100;

/// Issue information returned from GitHub API.
///
/// Contains all relevant metadata about an issue including
/// state, labels, assignees, and timestamps.
#[derive(Debug, Clone, Serialize)]
pub struct IssueInfo {
    /// Issue number (unique within repository).
    pub number: u64,
    /// Issue title.
    pub title: String,
    /// Issue body/description (may be None if empty).
    pub body: Option<String>,
    /// Issue state: "open" or "closed".
    pub state: String,
    /// Username of issue author.
    pub author: String,
    /// List of assigned usernames.
    pub assignees: Vec<String>,
    /// List of label names.
    pub labels: Vec<String>,
    /// ISO 8601 timestamp when issue was created.
    pub created_at: String,
    /// ISO 8601 timestamp when issue was last updated.
    pub updated_at: String,
    /// ISO 8601 timestamp when issue was closed (if closed).
    pub closed_at: Option<String>,
    /// Number of comments on the issue.
    pub comments_count: u32,
    /// GitHub HTML URL for the issue.
    pub url: String,
}

/// Issue comment information.
///
/// Represents a single comment on an issue with author
/// and timestamp metadata.
#[derive(Debug, Clone, Serialize)]
pub struct IssueComment {
    /// Comment ID (unique globally).
    pub id: u64,
    /// Username of comment author.
    pub author: String,
    /// Comment body text.
    pub body: String,
    /// ISO 8601 timestamp when comment was created.
    pub created_at: String,
    /// ISO 8601 timestamp when comment was last updated.
    pub updated_at: String,
}

/// Filters for searching issues.
///
/// All filters are optional. When multiple filters are provided,
/// they are combined with AND logic.
#[derive(Debug, Clone, Default)]
pub struct IssueSearchFilters {
    /// Filter by state: "open", "closed", or "all".
    pub state: Option<String>,
    /// Filter by labels (issue must have ALL specified labels).
    pub labels: Option<Vec<String>>,
    /// Filter by assignee username.
    pub assignee: Option<String>,
    /// Filter by issue creator username.
    pub creator: Option<String>,
    /// Search text in issue title and body.
    pub query: Option<String>,
}

/// GitHub API response structures for deserialization.
#[derive(Deserialize)]
struct IssueResponse {
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    user: UserResponse,
    assignees: Vec<UserResponse>,
    labels: Vec<LabelResponse>,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    comments: u32,
    html_url: String,
    #[serde(default)]
    pull_request: Option<PullRequestRef>,
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
}

#[derive(Deserialize)]
struct LabelResponse {
    name: String,
}

#[derive(Deserialize)]
struct PullRequestRef {
    #[allow(dead_code)]
    url: Option<String>,
}

#[derive(Deserialize)]
struct CommentResponse {
    id: u64,
    user: UserResponse,
    body: String,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct SearchIssuesResponse {
    items: Vec<IssueResponse>,
    #[allow(dead_code)]
    total_count: u32,
}

/// Search for issues in a repository.
///
/// Uses GitHub's search API for advanced filtering or the issues list
/// endpoint for simple queries.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `filters` - Search filters to apply
/// * `max_results` - Maximum number of results to return
///
/// # Returns
///
/// Vector of issues matching the filters. Note that pull requests
/// are filtered out since GitHub's issues endpoint includes them.
///
/// # Example
///
/// ```no_run
/// use gitctx::github::issues::{search_issues, IssueSearchFilters};
///
/// let filters = IssueSearchFilters {
///     state: Some("open".to_string()),
///     labels: Some(vec!["bug".to_string()]),
///     ..Default::default()
/// };
/// let issues = search_issues(&client, "owner", "repo", &filters, 10).await?;
/// ```
pub async fn search_issues(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    filters: &IssueSearchFilters,
    max_results: usize,
) -> Result<Vec<IssueInfo>> {
    let max_results = max_results.min(MAX_ISSUE_RESULTS);

    // Build query for GitHub search API
    let mut query_parts = vec![format!("repo:{}/{}", owner, repo), "is:issue".to_string()];

    if let Some(ref state) = filters.state {
        if state != "all" {
            query_parts.push(format!("is:{}", state));
        }
    } else {
        // Default to open issues
        query_parts.push("is:open".to_string());
    }

    if let Some(ref labels) = filters.labels {
        for label in labels {
            // Labels with spaces need to be quoted
            if label.contains(' ') {
                query_parts.push(format!("label:\"{}\"", label));
            } else {
                query_parts.push(format!("label:{}", label));
            }
        }
    }

    if let Some(ref assignee) = filters.assignee {
        query_parts.push(format!("assignee:{}", assignee));
    }

    if let Some(ref creator) = filters.creator {
        query_parts.push(format!("author:{}", creator));
    }

    if let Some(ref text) = filters.query {
        // Add text search term
        query_parts.push(text.clone());
    }

    let full_query = query_parts.join(" ");
    let encoded_query = urlencoding::encode(&full_query);
    let endpoint = format!(
        "/search/issues?q={}&per_page={}&sort=updated&order=desc",
        encoded_query, max_results
    );

    let response: SearchIssuesResponse = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to search issues: {}", e))?;

    // Filter out pull requests (they appear in issue search results)
    let issues = response
        .items
        .into_iter()
        .filter(|issue| issue.pull_request.is_none())
        .map(|issue| IssueInfo {
            number: issue.number,
            title: issue.title,
            body: issue.body,
            state: issue.state,
            author: issue.user.login,
            assignees: issue.assignees.into_iter().map(|u| u.login).collect(),
            labels: issue.labels.into_iter().map(|l| l.name).collect(),
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            closed_at: issue.closed_at,
            comments_count: issue.comments,
            url: issue.html_url,
        })
        .collect();

    Ok(issues)
}

/// Get detailed information about a specific issue.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `issue_number` - The issue number to retrieve
///
/// # Returns
///
/// Full issue information including body content.
///
/// # Errors
///
/// Returns an error if the issue doesn't exist or isn't accessible.
pub async fn get_issue(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    issue_number: u64,
) -> Result<IssueInfo> {
    let endpoint = format!("/repos/{}/{}/issues/{}", owner, repo, issue_number);

    let issue: IssueResponse = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get issue #{}: {}", issue_number, e))?;

    // Check if this is actually a PR
    if issue.pull_request.is_some() {
        return Err(anyhow!(
            "#{} is a pull request, not an issue. Use get_pr instead.",
            issue_number
        ));
    }

    Ok(IssueInfo {
        number: issue.number,
        title: issue.title,
        body: issue.body,
        state: issue.state,
        author: issue.user.login,
        assignees: issue.assignees.into_iter().map(|u| u.login).collect(),
        labels: issue.labels.into_iter().map(|l| l.name).collect(),
        created_at: issue.created_at,
        updated_at: issue.updated_at,
        closed_at: issue.closed_at,
        comments_count: issue.comments,
        url: issue.html_url,
    })
}

/// List comments on an issue with pagination support.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `issue_number` - The issue number to get comments for
/// * `page` - Page number (1-indexed, defaults to 1)
/// * `per_page` - Number of comments per page (defaults to 30, max 100)
///
/// # Returns
///
/// Tuple of (comments, has_more) where has_more indicates if there are
/// additional pages of comments.
///
/// # Example
///
/// ```no_run
/// // Get first page of comments
/// let (comments, has_more) = list_issue_comments(&client, "owner", "repo", 123, None, None).await?;
///
/// // Get subsequent pages if needed
/// if has_more {
///     let (more_comments, _) = list_issue_comments(&client, "owner", "repo", 123, Some(2), None).await?;
/// }
/// ```
pub async fn list_issue_comments(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    issue_number: u64,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<(Vec<IssueComment>, bool)> {
    let page = page.unwrap_or(1);
    let per_page = per_page
        .unwrap_or(DEFAULT_COMMENTS_PER_PAGE)
        .min(MAX_COMMENTS_PER_PAGE);

    let endpoint = format!(
        "/repos/{}/{}/issues/{}/comments?page={}&per_page={}",
        owner, repo, issue_number, page, per_page
    );

    let comments: Vec<CommentResponse> = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to list comments for issue #{}: {}", issue_number, e))?;

    // Determine if there are more pages
    let has_more = comments.len() as u32 == per_page;

    let comments = comments
        .into_iter()
        .map(|c| IssueComment {
            id: c.id,
            author: c.user.login,
            body: c.body,
            created_at: c.created_at,
            updated_at: c.updated_at,
        })
        .collect();

    Ok((comments, has_more))
}
