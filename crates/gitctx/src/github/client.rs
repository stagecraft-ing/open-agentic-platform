//! GitHub API client for repository content operations.
//!
//! This module provides functions for interacting with GitHub's REST API
//! to explore repository contents, read files, and search code.
//!
//! # Rate Limiting
//!
//! GitHub's API has rate limits:
//! - Unauthenticated: 60 requests/hour
//! - Authenticated: 5,000 requests/hour
//!
//! Always use authentication for production use.

use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};

/// Maximum file size in bytes (500KB).
/// Files larger than this will be rejected to prevent memory issues.
pub const MAX_FILE_SIZE: usize = 500_000;

/// Maximum number of items to return from directory listings.
pub const MAX_DIR_ITEMS: usize = 200;

/// Maximum number of search results.
pub const MAX_SEARCH_RESULTS: usize = 50;

/// Create an authenticated or unauthenticated octocrab client.
///
/// # Arguments
///
/// * `token` - Optional GitHub personal access token
///
/// # Returns
///
/// Configured Octocrab client instance.
///
/// # Examples
///
/// ```no_run
/// use gitctx::github::client::create_client;
///
/// // Authenticated
/// let client = create_client(Some("ghp_xxx"))?;
///
/// // Unauthenticated (rate limited)
/// let client = create_client(None)?;
/// ```
pub fn create_client(token: Option<&str>) -> Result<Octocrab> {
    let mut builder = Octocrab::builder();

    if let Some(tok) = token {
        builder = builder.personal_token(tok.to_string());
    }

    builder
        .build()
        .map_err(|e| anyhow!("Failed to create GitHub client: {}", e))
}

/// Create a shared Octocrab client wrapped in Arc for thread-safe sharing.
///
/// This function creates a client that can be stored in `GitHubContext`
/// and shared across multiple tools without creating new clients each time.
///
/// # Arguments
///
/// * `token` - Optional GitHub personal access token
///
/// # Returns
///
/// Arc-wrapped Octocrab client instance.
///
/// # Examples
///
/// ```no_run
/// use gitctx::github::client::create_shared_client;
///
/// // Create a shared authenticated client
/// let client = create_shared_client(Some("ghp_xxx"))?;
///
/// // Store in context for sharing across tools
/// // ctx.set_client(client);
/// ```
pub fn create_shared_client(token: Option<&str>) -> Result<std::sync::Arc<Octocrab>> {
    Ok(std::sync::Arc::new(create_client(token)?))
}

/// Repository information.
#[derive(Debug, Clone, Serialize)]
pub struct RepositoryInfo {
    /// Repository owner (user or organization).
    pub owner: String,
    /// Repository name.
    pub name: String,
    /// Default branch (usually "main" or "master").
    pub default_branch: String,
    /// Whether the repository is private.
    pub is_private: bool,
    /// Repository description, if available.
    pub description: Option<String>,
}

/// Get repository information including the default branch.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
///
/// # Returns
///
/// Repository information including the default branch.
///
/// # Errors
///
/// Returns an error if the repository doesn't exist or isn't accessible.
pub async fn get_repo_info(client: &Octocrab, owner: &str, repo: &str) -> Result<RepositoryInfo> {
    #[derive(Deserialize)]
    struct RepoResponse {
        default_branch: String,
        private: bool,
        description: Option<String>,
    }

    let resp: RepoResponse = client
        .get(format!("/repos/{}/{}", owner, repo), None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get repository info: {}", e))?;

    Ok(RepositoryInfo {
        owner: owner.to_string(),
        name: repo.to_string(),
        default_branch: resp.default_branch,
        is_private: resp.private,
        description: resp.description,
    })
}

/// A file or directory entry in a repository.
#[derive(Debug, Clone, Serialize)]
pub struct DirectoryEntry {
    /// File or directory name.
    pub name: String,
    /// Full path from repository root.
    pub path: String,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// File size in bytes (None for directories).
    pub size: Option<u64>,
}

/// List directory contents at a path in a repository.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `branch` - Branch name
/// * `path` - Directory path (use "/" or "" for root)
///
/// # Returns
///
/// Vector of directory entries (files and subdirectories).
///
/// # Errors
///
/// Returns an error if the path doesn't exist or isn't accessible.
pub async fn list_directory(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
) -> Result<Vec<DirectoryEntry>> {
    #[derive(Deserialize)]
    struct ContentItem {
        name: String,
        path: String,
        #[serde(rename = "type")]
        item_type: String,
        size: Option<u64>,
    }

    // Normalize path
    let path_param = path.trim_start_matches('/').trim_end_matches('/');

    let endpoint = if path_param.is_empty() {
        format!("/repos/{}/{}/contents?ref={}", owner, repo, branch)
    } else {
        format!(
            "/repos/{}/{}/contents/{}?ref={}",
            owner, repo, path_param, branch
        )
    };

    let items: Vec<ContentItem> = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to list directory '{}': {}", path, e))?;

    let entries: Vec<DirectoryEntry> = items
        .into_iter()
        .take(MAX_DIR_ITEMS)
        .map(|item| DirectoryEntry {
            name: item.name,
            path: item.path,
            is_dir: item.item_type == "dir",
            size: item.size,
        })
        .collect();

    Ok(entries)
}

/// File contents and metadata.
#[derive(Debug, Clone, Serialize)]
pub struct FileContents {
    /// File path.
    pub path: String,
    /// File name.
    pub name: String,
    /// File size in bytes.
    pub size: u64,
    /// File content (None if too large or binary).
    pub content: Option<String>,
    /// Whether the content was truncated.
    pub truncated: bool,
    /// Error message if reading failed.
    pub error: Option<String>,
}

/// Read file contents from a repository.
///
/// Files larger than `max_size` will be rejected with an error.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `branch` - Branch name
/// * `path` - File path
/// * `max_size` - Maximum file size to read
///
/// # Returns
///
/// File contents and metadata.
///
/// # Errors
///
/// Returns an error if the file doesn't exist or can't be read.
pub async fn read_file(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    max_size: usize,
) -> Result<FileContents> {
    #[derive(Deserialize)]
    struct FileResponse {
        content: Option<String>,
        #[allow(dead_code)]
        encoding: Option<String>,
        size: u64,
        name: String,
        #[serde(rename = "type")]
        item_type: Option<String>,
    }

    let path_param = path.trim_start_matches('/');
    let endpoint = format!(
        "/repos/{}/{}/contents/{}?ref={}",
        owner, repo, path_param, branch
    );

    let resp: FileResponse = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to read file '{}': {}", path, e))?;

    // Check if it's actually a file
    if resp.item_type.as_deref() == Some("dir") {
        return Ok(FileContents {
            path: path.to_string(),
            name: resp.name,
            size: resp.size,
            content: None,
            truncated: false,
            error: Some("Path is a directory, not a file".to_string()),
        });
    }

    // Check size limit
    if resp.size > max_size as u64 {
        return Ok(FileContents {
            path: path.to_string(),
            name: resp.name,
            size: resp.size,
            content: None,
            truncated: true,
            error: Some(format!(
                "File too large ({} bytes, max {} bytes). Try reading a smaller file or use search_code to find specific content.",
                resp.size, max_size
            )),
        });
    }

    // Decode base64 content
    let content = if let Some(encoded) = resp.content {
        // GitHub's base64 content includes newlines for formatting
        let cleaned = encoded.replace('\n', "");
        match STANDARD.decode(&cleaned) {
            Ok(bytes) => {
                // Check if it's valid UTF-8 (text file)
                match String::from_utf8(bytes) {
                    Ok(text) => Some(text),
                    Err(_) => {
                        return Ok(FileContents {
                            path: path.to_string(),
                            name: resp.name,
                            size: resp.size,
                            content: None,
                            truncated: false,
                            error: Some("Binary file - cannot display content".to_string()),
                        });
                    }
                }
            }
            Err(e) => {
                return Ok(FileContents {
                    path: path.to_string(),
                    name: resp.name,
                    size: resp.size,
                    content: None,
                    truncated: false,
                    error: Some(format!("Failed to decode content: {}", e)),
                });
            }
        }
    } else {
        return Ok(FileContents {
            path: path.to_string(),
            name: resp.name,
            size: resp.size,
            content: None,
            truncated: false,
            error: Some("No content available".to_string()),
        });
    };

    Ok(FileContents {
        path: path.to_string(),
        name: resp.name,
        size: resp.size,
        content,
        truncated: false,
        error: None,
    })
}

/// Code search result.
#[derive(Debug, Clone, Serialize)]
pub struct CodeSearchResult {
    /// File name.
    pub name: String,
    /// File path.
    pub path: String,
    /// GitHub URL to the file.
    pub url: String,
    /// Matching text fragments.
    pub matches: Vec<String>,
}

/// Search for code in a repository.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `query` - Search query
/// * `max_results` - Maximum number of results
///
/// # Returns
///
/// Vector of code search results.
///
/// # Note
///
/// GitHub's code search API requires authentication and has specific requirements:
/// - Queries must have at least 3 characters
/// - Results are limited based on your authentication level
pub async fn search_code(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    query: &str,
    max_results: usize,
) -> Result<Vec<CodeSearchResult>> {
    #[derive(Deserialize)]
    struct SearchResponse {
        items: Vec<SearchItem>,
        #[allow(dead_code)]
        total_count: u32,
    }

    #[derive(Deserialize)]
    struct SearchItem {
        name: String,
        path: String,
        html_url: String,
        #[serde(default)]
        text_matches: Vec<TextMatch>,
    }

    #[derive(Deserialize)]
    struct TextMatch {
        fragment: String,
    }

    // Build search query with repo scope
    let full_query = format!("{} repo:{}/{}", query, owner, repo);
    let encoded_query = urlencoding::encode(&full_query);
    let per_page = max_results.min(MAX_SEARCH_RESULTS);

    let endpoint = format!("/search/code?q={}&per_page={}", encoded_query, per_page);

    // Make request with text-match media type for match fragments
    let resp: SearchResponse = client
        .get(&endpoint, None::<&()>)
        .await
        .context("Code search failed - GitHub requires authentication for code search")?;

    Ok(resp
        .items
        .into_iter()
        .map(|item| CodeSearchResult {
            name: item.name,
            path: item.path,
            url: item.html_url,
            matches: item.text_matches.into_iter().map(|m| m.fragment).collect(),
        })
        .collect())
}

/// A tree entry from GitHub's Git Trees API.
#[derive(Debug, Clone, Serialize)]
pub struct TreeEntry {
    /// Full path from repository root.
    pub path: String,
    /// Git file mode (e.g., "100644" for file, "040000" for directory).
    pub mode: String,
    /// Entry type: "blob" for files, "tree" for directories.
    pub entry_type: String,
    /// File size in bytes (None for directories).
    pub size: Option<u64>,
}

/// Result of a tree query with metadata.
#[derive(Debug, Clone, Serialize)]
pub struct TreeResult {
    /// Tree entries matching the query.
    pub entries: Vec<TreeEntry>,
    /// Total entries before truncation.
    pub total_count: usize,
    /// Whether the result was truncated.
    pub truncated: bool,
    /// Number of directories.
    pub dir_count: usize,
    /// Number of files.
    pub file_count: usize,
}

/// Branch information.
#[derive(Debug, Clone, Serialize)]
pub struct BranchInfo {
    /// Branch name.
    pub name: String,
    /// Whether the branch is protected.
    pub protected: bool,
}

/// List branches in a repository.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
///
/// # Returns
///
/// Vector of branch information.
pub async fn list_branches(client: &Octocrab, owner: &str, repo: &str) -> Result<Vec<BranchInfo>> {
    #[derive(Deserialize)]
    struct BranchResponse {
        name: String,
        protected: bool,
    }

    let endpoint = format!("/repos/{}/{}/branches?per_page=100", owner, repo);

    let branches: Vec<BranchResponse> = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to list branches: {}", e))?;

    Ok(branches
        .into_iter()
        .map(|b| BranchInfo {
            name: b.name,
            protected: b.protected,
        })
        .collect())
}

/// Get the repository tree using GitHub's Git Trees API.
///
/// This is much more efficient than recursive `list_directory` calls
/// as it fetches the entire tree in a single API request.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `branch` - Branch name to get tree from
/// * `path` - Directory path to filter (use "/" or "" for root)
/// * `depth` - Maximum depth to traverse (0 = unlimited)
/// * `max_entries` - Maximum entries to return
///
/// # Returns
///
/// Tree result with entries and metadata.
///
/// # Examples
///
/// ```no_run
/// use gitctx::github::client::{create_client, get_tree};
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = create_client(Some("ghp_xxx"))?;
///
/// // Get full tree
/// let result = get_tree(&client, "rust-lang", "rust", "master", "/", 0, 500).await?;
///
/// // Get only src/ directory, depth 2
/// let result = get_tree(&client, "rust-lang", "rust", "master", "src", 2, 100).await?;
/// # Ok(())
/// # }
/// ```
pub async fn get_tree(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    branch: &str,
    path: &str,
    depth: usize,
    max_entries: usize,
) -> Result<TreeResult> {
    #[derive(Deserialize)]
    struct RefResponse {
        object: RefObject,
    }

    #[derive(Deserialize)]
    struct RefObject {
        sha: String,
    }

    #[derive(Deserialize)]
    struct TreeResponse {
        tree: Vec<TreeItem>,
        truncated: bool,
    }

    #[derive(Deserialize)]
    struct TreeItem {
        path: String,
        mode: String,
        #[serde(rename = "type")]
        item_type: String,
        size: Option<u64>,
    }

    // Get the commit SHA for the branch
    let ref_endpoint = format!("/repos/{}/{}/git/ref/heads/{}", owner, repo, branch);
    let ref_resp: RefResponse = client
        .get(&ref_endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get branch ref '{}': {}", branch, e))?;

    // Get the tree with recursive flag
    let tree_endpoint = format!(
        "/repos/{}/{}/git/trees/{}?recursive=1",
        owner, repo, ref_resp.object.sha
    );
    let tree_resp: TreeResponse = client
        .get(&tree_endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get tree: {}", e))?;

    // Normalize path filter
    let path_filter = path.trim_start_matches('/').trim_end_matches('/');
    let path_prefix = if path_filter.is_empty() {
        String::new()
    } else {
        format!("{}/", path_filter)
    };

    // Filter and transform entries
    let mut entries: Vec<TreeEntry> = tree_resp
        .tree
        .into_iter()
        .filter(|item| {
            // If path filter is set, only include items under that path
            if !path_filter.is_empty()
                && !item.path.starts_with(&path_prefix)
                && item.path != path_filter
            {
                return false;
            }

            // Apply depth filter if set (depth > 0)
            if depth > 0 {
                let relative_path = if path_filter.is_empty() {
                    &item.path
                } else if item.path.starts_with(&path_prefix) {
                    &item.path[path_prefix.len()..]
                } else {
                    return item.path == path_filter;
                };

                let item_depth = relative_path.matches('/').count() + 1;
                if item_depth > depth {
                    return false;
                }
            }

            true
        })
        .map(|item| TreeEntry {
            path: item.path,
            mode: item.mode,
            entry_type: item.item_type,
            size: item.size,
        })
        .collect();

    // Sort entries for consistent tree output (directories first, then alphabetically)
    entries.sort_by(|a, b| {
        let a_is_dir = a.entry_type == "tree";
        let b_is_dir = b.entry_type == "tree";
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.path.cmp(&b.path),
        }
    });

    // Re-sort by path for proper tree structure
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let total_count = entries.len();
    let dir_count = entries.iter().filter(|e| e.entry_type == "tree").count();
    let file_count = entries.iter().filter(|e| e.entry_type == "blob").count();

    // Truncate if needed
    let truncated = entries.len() > max_entries || tree_resp.truncated;
    entries.truncate(max_entries);

    Ok(TreeResult {
        entries,
        total_count,
        truncated,
        dir_count,
        file_count,
    })
}
