//! GitHub Pull Requests API client functions.
//!
//! This module provides functions for interacting with GitHub's Pull Requests API
//! to search PRs, read PR details, and list comments/reviews.
//!
//! # API Endpoints Used
//!
//! - `GET /repos/{owner}/{repo}/pulls` - List repository PRs
//! - `GET /repos/{owner}/{repo}/pulls/{number}` - Get single PR
//! - `GET /repos/{owner}/{repo}/pulls/{number}/reviews` - List PR reviews
//! - `GET /repos/{owner}/{repo}/pulls/{number}/comments` - List review comments
//! - `GET /repos/{owner}/{repo}/issues/{number}/comments` - List conversation comments
//! - `GET /search/issues` - Search PRs with advanced filters

use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};

/// Maximum number of PRs to return from search.
pub const MAX_PR_RESULTS: usize = 30;

/// Default number of comments per page.
pub const DEFAULT_COMMENTS_PER_PAGE: u32 = 30;

/// Maximum comments per page (GitHub API limit).
pub const MAX_COMMENTS_PER_PAGE: u32 = 100;

/// Pull request information returned from GitHub API.
///
/// Contains comprehensive metadata about a PR including
/// branches, merge status, and file change statistics.
#[derive(Debug, Clone, Serialize)]
pub struct PRInfo {
    /// PR number (unique within repository).
    pub number: u64,
    /// PR title.
    pub title: String,
    /// PR body/description (may be None if empty).
    pub body: Option<String>,
    /// PR state: "open" or "closed".
    pub state: String,
    /// Whether the PR has been merged.
    pub merged: bool,
    /// Username of PR author.
    pub author: String,
    /// Source branch name (head).
    pub head_branch: String,
    /// Target branch name (base).
    pub base_branch: String,
    /// SHA of the head commit.
    pub head_sha: String,
    /// List of label names.
    pub labels: Vec<String>,
    /// List of assigned usernames.
    pub assignees: Vec<String>,
    /// List of requested reviewer usernames.
    pub reviewers: Vec<String>,
    /// ISO 8601 timestamp when PR was created.
    pub created_at: String,
    /// ISO 8601 timestamp when PR was last updated.
    pub updated_at: String,
    /// ISO 8601 timestamp when PR was merged (if merged).
    pub merged_at: Option<String>,
    /// ISO 8601 timestamp when PR was closed (if closed).
    pub closed_at: Option<String>,
    /// Number of conversation comments.
    pub comments_count: u32,
    /// Number of review comments (inline code comments).
    pub review_comments_count: u32,
    /// Number of commits in the PR.
    pub commits_count: u32,
    /// Number of lines added.
    pub additions: u32,
    /// Number of lines deleted.
    pub deletions: u32,
    /// Number of files changed.
    pub changed_files: u32,
    /// GitHub HTML URL for the PR.
    pub url: String,
    /// Whether the PR can be merged (null if unknown).
    pub mergeable: Option<bool>,
    /// Mergeable state description.
    pub mergeable_state: Option<String>,
}

/// PR conversation comment (not code review comment).
///
/// These appear in the main PR conversation thread.
#[derive(Debug, Clone, Serialize)]
pub struct PRComment {
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

/// PR review submission.
///
/// Represents a formal review with a state (approved, changes requested, etc.).
#[derive(Debug, Clone, Serialize)]
pub struct PRReview {
    /// Review ID (unique globally).
    pub id: u64,
    /// Username of reviewer.
    pub author: String,
    /// Review state: "APPROVED", "CHANGES_REQUESTED", "COMMENTED", "PENDING", "DISMISSED".
    pub state: String,
    /// Review body text (may be None).
    pub body: Option<String>,
    /// ISO 8601 timestamp when review was submitted.
    pub submitted_at: Option<String>,
}

/// PR review comment (inline code comment).
///
/// These are comments attached to specific lines of code in the diff.
#[derive(Debug, Clone, Serialize)]
pub struct PRReviewComment {
    /// Comment ID (unique globally).
    pub id: u64,
    /// Username of comment author.
    pub author: String,
    /// Comment body text.
    pub body: String,
    /// File path the comment is attached to.
    pub path: String,
    /// Line number in the diff (may be None for outdated comments).
    pub line: Option<u32>,
    /// Original line in the file (may be None).
    pub original_line: Option<u32>,
    /// Side of the diff: "LEFT" (deletion) or "RIGHT" (addition).
    pub side: Option<String>,
    /// Surrounding diff context.
    pub diff_hunk: String,
    /// ISO 8601 timestamp when comment was created.
    pub created_at: String,
    /// ID of the comment this is replying to (if a reply).
    pub in_reply_to_id: Option<u64>,
}

/// Filters for searching pull requests.
///
/// All filters are optional. When multiple filters are provided,
/// they are combined with AND logic.
#[derive(Debug, Clone, Default)]
pub struct PRSearchFilters {
    /// Filter by state: "open", "closed", "merged", or "all".
    pub state: Option<String>,
    /// Filter by PR author username.
    pub author: Option<String>,
    /// Search text in PR title and body.
    pub query: Option<String>,
}

/// GitHub API response structures for deserialization.
#[derive(Deserialize)]
struct PRResponse {
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    merged: Option<bool>,
    user: UserResponse,
    head: BranchRef,
    base: BranchRef,
    labels: Vec<LabelResponse>,
    assignees: Vec<UserResponse>,
    requested_reviewers: Vec<UserResponse>,
    created_at: String,
    updated_at: String,
    merged_at: Option<String>,
    closed_at: Option<String>,
    comments: Option<u32>,
    review_comments: Option<u32>,
    commits: Option<u32>,
    additions: Option<u32>,
    deletions: Option<u32>,
    changed_files: Option<u32>,
    html_url: String,
    mergeable: Option<bool>,
    mergeable_state: Option<String>,
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
}

#[derive(Deserialize)]
struct BranchRef {
    #[serde(rename = "ref")]
    branch_ref: String,
    sha: String,
}

#[derive(Deserialize)]
struct LabelResponse {
    name: String,
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
struct ReviewResponse {
    id: u64,
    user: UserResponse,
    state: String,
    body: Option<String>,
    submitted_at: Option<String>,
}

#[derive(Deserialize)]
struct ReviewCommentResponse {
    id: u64,
    user: UserResponse,
    body: String,
    path: String,
    line: Option<u32>,
    original_line: Option<u32>,
    side: Option<String>,
    diff_hunk: String,
    created_at: String,
    in_reply_to_id: Option<u64>,
}

#[derive(Deserialize)]
struct SearchIssuesResponse {
    items: Vec<SearchPRItem>,
    #[allow(dead_code)]
    total_count: u32,
}

#[derive(Deserialize)]
struct SearchPRItem {
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    user: UserResponse,
    labels: Vec<LabelResponse>,
    assignees: Vec<UserResponse>,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    comments: u32,
    html_url: String,
    pull_request: Option<PullRequestRef>,
}

#[derive(Deserialize)]
struct PullRequestRef {
    merged_at: Option<String>,
}

/// Search for pull requests in a repository.
///
/// Uses GitHub's search API for advanced filtering.
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
/// Vector of PRs matching the filters.
///
/// # Example
///
/// ```no_run
/// use gitctx::github::pulls::{search_prs, PRSearchFilters};
///
/// let filters = PRSearchFilters {
///     state: Some("open".to_string()),
///     author: Some("octocat".to_string()),
///     ..Default::default()
/// };
/// let prs = search_prs(&client, "owner", "repo", &filters, 10).await?;
/// ```
pub async fn search_prs(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    filters: &PRSearchFilters,
    max_results: usize,
) -> Result<Vec<PRInfo>> {
    let max_results = max_results.min(MAX_PR_RESULTS);

    // Build query for GitHub search API
    let mut query_parts = vec![format!("repo:{}/{}", owner, repo), "is:pr".to_string()];

    if let Some(ref state) = filters.state {
        match state.as_str() {
            "merged" => {
                query_parts.push("is:merged".to_string());
            }
            "all" => {
                // No state filter
            }
            state => {
                query_parts.push(format!("is:{}", state));
            }
        }
    } else {
        // Default to open PRs
        query_parts.push("is:open".to_string());
    }

    if let Some(ref author) = filters.author {
        query_parts.push(format!("author:{}", author));
    }

    if let Some(ref text) = filters.query {
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
        .map_err(|e| anyhow!("Failed to search pull requests: {}", e))?;

    // Filter to only PRs (should already be, but verify)
    let prs = response
        .items
        .into_iter()
        .filter(|item| item.pull_request.is_some())
        .map(|item| {
            let merged = item
                .pull_request
                .as_ref()
                .and_then(|pr| pr.merged_at.as_ref())
                .is_some();
            PRInfo {
                number: item.number,
                title: item.title,
                body: item.body,
                state: if merged {
                    "merged".to_string()
                } else {
                    item.state
                },
                merged,
                author: item.user.login,
                head_branch: String::new(), // Not available in search results
                base_branch: String::new(), // Not available in search results
                head_sha: String::new(),    // Not available in search results
                labels: item.labels.into_iter().map(|l| l.name).collect(),
                assignees: item.assignees.into_iter().map(|u| u.login).collect(),
                reviewers: vec![], // Not available in search results
                created_at: item.created_at,
                updated_at: item.updated_at,
                merged_at: item.pull_request.and_then(|pr| pr.merged_at),
                closed_at: item.closed_at,
                comments_count: item.comments,
                review_comments_count: 0, // Not available in search results
                commits_count: 0,         // Not available in search results
                additions: 0,             // Not available in search results
                deletions: 0,             // Not available in search results
                changed_files: 0,         // Not available in search results
                url: item.html_url,
                mergeable: None,
                mergeable_state: None,
            }
        })
        .collect();

    Ok(prs)
}

/// Get detailed information about a specific pull request.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `pr_number` - The PR number to retrieve
///
/// # Returns
///
/// Full PR information including branches, merge status, and file changes.
///
/// # Errors
///
/// Returns an error if the PR doesn't exist or isn't accessible.
pub async fn get_pr(client: &Octocrab, owner: &str, repo: &str, pr_number: u64) -> Result<PRInfo> {
    let endpoint = format!("/repos/{}/{}/pulls/{}", owner, repo, pr_number);

    let pr: PRResponse = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get PR #{}: {}", pr_number, e))?;

    let merged = pr.merged.unwrap_or(false);

    Ok(PRInfo {
        number: pr.number,
        title: pr.title,
        body: pr.body,
        state: if merged {
            "merged".to_string()
        } else {
            pr.state
        },
        merged,
        author: pr.user.login,
        head_branch: pr.head.branch_ref,
        base_branch: pr.base.branch_ref,
        head_sha: pr.head.sha,
        labels: pr.labels.into_iter().map(|l| l.name).collect(),
        assignees: pr.assignees.into_iter().map(|u| u.login).collect(),
        reviewers: pr
            .requested_reviewers
            .into_iter()
            .map(|u| u.login)
            .collect(),
        created_at: pr.created_at,
        updated_at: pr.updated_at,
        merged_at: pr.merged_at,
        closed_at: pr.closed_at,
        comments_count: pr.comments.unwrap_or(0),
        review_comments_count: pr.review_comments.unwrap_or(0),
        commits_count: pr.commits.unwrap_or(0),
        additions: pr.additions.unwrap_or(0),
        deletions: pr.deletions.unwrap_or(0),
        changed_files: pr.changed_files.unwrap_or(0),
        url: pr.html_url,
        mergeable: pr.mergeable,
        mergeable_state: pr.mergeable_state,
    })
}

/// List conversation comments on a PR (not code review comments).
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `pr_number` - The PR number
/// * `page` - Page number (1-indexed, defaults to 1)
/// * `per_page` - Comments per page (defaults to 30, max 100)
///
/// # Returns
///
/// Tuple of (comments, has_more).
pub async fn list_pr_comments(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    pr_number: u64,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<(Vec<PRComment>, bool)> {
    let page = page.unwrap_or(1);
    let per_page = per_page
        .unwrap_or(DEFAULT_COMMENTS_PER_PAGE)
        .min(MAX_COMMENTS_PER_PAGE);

    // PR conversation comments use the issues API
    let endpoint = format!(
        "/repos/{}/{}/issues/{}/comments?page={}&per_page={}",
        owner, repo, pr_number, page, per_page
    );

    let comments: Vec<CommentResponse> = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to list comments for PR #{}: {}", pr_number, e))?;

    let has_more = comments.len() as u32 == per_page;

    let comments = comments
        .into_iter()
        .map(|c| PRComment {
            id: c.id,
            author: c.user.login,
            body: c.body,
            created_at: c.created_at,
            updated_at: c.updated_at,
        })
        .collect();

    Ok((comments, has_more))
}

/// List reviews on a pull request.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `pr_number` - The PR number
///
/// # Returns
///
/// Vector of all reviews on the PR.
pub async fn list_pr_reviews(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    pr_number: u64,
) -> Result<Vec<PRReview>> {
    let endpoint = format!(
        "/repos/{}/{}/pulls/{}/reviews?per_page=100",
        owner, repo, pr_number
    );

    let reviews: Vec<ReviewResponse> = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to list reviews for PR #{}: {}", pr_number, e))?;

    Ok(reviews
        .into_iter()
        .map(|r| PRReview {
            id: r.id,
            author: r.user.login,
            state: r.state,
            body: r.body,
            submitted_at: r.submitted_at,
        })
        .collect())
}

/// List review comments (inline code comments) on a PR.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `pr_number` - The PR number
/// * `page` - Page number (1-indexed, defaults to 1)
/// * `per_page` - Comments per page (defaults to 30, max 100)
///
/// # Returns
///
/// Tuple of (review_comments, has_more).
pub async fn list_pr_review_comments(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    pr_number: u64,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<(Vec<PRReviewComment>, bool)> {
    let page = page.unwrap_or(1);
    let per_page = per_page
        .unwrap_or(DEFAULT_COMMENTS_PER_PAGE)
        .min(MAX_COMMENTS_PER_PAGE);

    let endpoint = format!(
        "/repos/{}/{}/pulls/{}/comments?page={}&per_page={}",
        owner, repo, pr_number, page, per_page
    );

    let comments: Vec<ReviewCommentResponse> =
        client.get(&endpoint, None::<&()>).await.map_err(|e| {
            anyhow!(
                "Failed to list review comments for PR #{}: {}",
                pr_number,
                e
            )
        })?;

    let has_more = comments.len() as u32 == per_page;

    let comments = comments
        .into_iter()
        .map(|c| PRReviewComment {
            id: c.id,
            author: c.user.login,
            body: c.body,
            path: c.path,
            line: c.line,
            original_line: c.original_line,
            side: c.side,
            diff_hunk: c.diff_hunk,
            created_at: c.created_at,
            in_reply_to_id: c.in_reply_to_id,
        })
        .collect();

    Ok((comments, has_more))
}
