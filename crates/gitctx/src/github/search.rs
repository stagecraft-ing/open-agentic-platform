//! GitHub repository search functionality.
//!
//! This module provides functions for searching GitHub repositories
//! to help auto-discover relevant repositories based on user queries.

use anyhow::{anyhow, Result};
use serde::Deserialize;

/// Repository search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Repository owner.
    pub owner: String,
    /// Repository name.
    pub name: String,
    /// Default branch.
    #[allow(dead_code)]
    pub default_branch: String,
    /// Number of stars.
    pub stars: u32,
    /// Repository description.
    pub description: Option<String>,
    /// Primary language.
    pub language: Option<String>,
}

impl SearchResult {
    /// Get the full repository name (owner/repo).
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// Search for repositories on GitHub.
///
/// # Arguments
///
/// * `token` - Optional GitHub token for authenticated requests
/// * `query` - Search query
/// * `limit` - Maximum number of results to return
///
/// # Returns
///
/// Vector of search results sorted by relevance (stars + match quality).
///
/// # Examples
///
/// ```no_run
/// use gitctx::github::search::search_repositories;
///
/// let results = search_repositories(Some("ghp_xxx"), "better-auth authentication", 5).await?;
/// for r in results {
///     println!("{}/{} - {} stars", r.owner, r.name, r.stars);
/// }
/// ```
pub async fn search_repositories(
    token: Option<&str>,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    #[derive(Deserialize)]
    struct SearchResponse {
        items: Vec<RepoItem>,
        #[allow(dead_code)]
        total_count: u32,
    }

    #[derive(Deserialize)]
    struct RepoItem {
        name: String,
        #[allow(dead_code)]
        full_name: String,
        description: Option<String>,
        stargazers_count: u32,
        default_branch: String,
        language: Option<String>,
        owner: OwnerInfo,
    }

    #[derive(Deserialize)]
    struct OwnerInfo {
        login: String,
    }

    // Build HTTP client
    let client = reqwest::Client::builder().build()?;

    let mut request = client
        .get("https://api.github.com/search/repositories")
        .query(&[
            ("q", query),
            ("sort", "stars"),
            ("order", "desc"),
            ("per_page", &limit.min(100).to_string()),
        ])
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "gitctx/0.1.0");

    if let Some(tok) = token {
        request = request.header("Authorization", format!("Bearer {}", tok));
    }

    let response = request.send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!("Repository search failed ({}): {}", status, body));
    }

    let search_response: SearchResponse = response.json().await?;

    Ok(search_response
        .items
        .into_iter()
        .map(|item| SearchResult {
            owner: item.owner.login,
            name: item.name,
            default_branch: item.default_branch,
            stars: item.stargazers_count,
            description: item.description,
            language: item.language,
        })
        .collect())
}

/// Parse a repository reference into owner and name.
///
/// Handles multiple formats:
/// - owner/repo
/// - https://github.com/owner/repo
/// - https://github.com/owner/repo.git
/// - git@github.com:owner/repo.git
///
/// # Arguments
///
/// * `reference` - The repository reference string
///
/// # Returns
///
/// Tuple of (owner, repo) strings.
///
/// # Errors
///
/// Returns an error if the format is not recognized.
pub fn parse_repo_reference(reference: &str) -> Result<(String, String)> {
    let reference = reference.trim();

    // Handle HTTPS URLs
    if reference.starts_with("https://github.com/") || reference.starts_with("http://github.com/") {
        let url = url::Url::parse(reference).map_err(|_| anyhow!("Invalid URL"))?;
        let segments: Vec<_> = url
            .path_segments()
            .ok_or_else(|| anyhow!("No path in URL"))?
            .collect();

        if segments.len() < 2 {
            return Err(anyhow!("URL must have owner/repo path"));
        }

        let owner = segments[0].to_string();
        let name = segments[1].trim_end_matches(".git").to_string();
        return Ok((owner, name));
    }

    // Handle SSH URLs (git@github.com:owner/repo.git)
    if reference.starts_with("git@github.com:") {
        let path = reference.trim_start_matches("git@github.com:");
        let parts: Vec<_> = path.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid SSH URL format"));
        }
        let owner = parts[0].to_string();
        let name = parts[1].trim_end_matches(".git").to_string();
        return Ok((owner, name));
    }

    // Handle owner/repo format
    let parts: Vec<_> = reference.split('/').collect();
    if parts.len() == 2 {
        let owner = parts[0].to_string();
        let name = parts[1].trim_end_matches(".git").to_string();

        // Validate parts aren't empty
        if owner.is_empty() || name.is_empty() {
            return Err(anyhow!("Owner and repo name cannot be empty"));
        }

        return Ok((owner, name));
    }

    Err(anyhow!(
        "Invalid repository reference. Use 'owner/repo' or 'https://github.com/owner/repo'"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_owner_repo() {
        let (owner, repo) = parse_repo_reference("rust-lang/rust").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_parse_https_url() {
        let (owner, repo) =
            parse_repo_reference("https://github.com/better-auth/better-auth").unwrap();
        assert_eq!(owner, "better-auth");
        assert_eq!(repo, "better-auth");
    }

    #[test]
    fn test_parse_https_url_with_git() {
        let (owner, repo) = parse_repo_reference("https://github.com/rust-lang/rust.git").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_parse_ssh_url() {
        let (owner, repo) = parse_repo_reference("git@github.com:rust-lang/rust.git").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_invalid_format() {
        assert!(parse_repo_reference("invalid").is_err());
        assert!(parse_repo_reference("/repo").is_err());
        assert!(parse_repo_reference("owner/").is_err());
    }
}
