//! GitHub Releases API client functions.
//!
//! This module provides functions for interacting with GitHub's Releases API
//! to list releases, get release details, and retrieve tags.
//!
//! # API Endpoints Used
//!
//! - `GET /repos/{owner}/{repo}/releases` - List repository releases
//! - `GET /repos/{owner}/{repo}/releases/tags/{tag}` - Get release by tag
//! - `GET /repos/{owner}/{repo}/releases/latest` - Get latest release
//! - `GET /repos/{owner}/{repo}/tags` - List repository tags

use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};

/// Maximum number of releases to return from list.
pub const MAX_RELEASE_RESULTS: usize = 100;

/// Default number of releases to return.
pub const DEFAULT_RELEASE_RESULTS: usize = 10;

/// Release information returned from GitHub API.
///
/// Contains all metadata about a release including tag, name, body,
/// and associated assets.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseInfo {
    /// Unique release ID.
    pub id: u64,
    /// Git tag name for this release (e.g., "v1.0.0").
    pub tag_name: String,
    /// Release title/name (may differ from tag).
    pub name: Option<String>,
    /// Release body/description (changelog in Markdown).
    pub body: Option<String>,
    /// Whether this is a draft release (not yet published).
    pub draft: bool,
    /// Whether this is a prerelease (beta, alpha, rc, etc.).
    pub prerelease: bool,
    /// ISO 8601 timestamp when the release was created.
    pub created_at: String,
    /// ISO 8601 timestamp when the release was published.
    pub published_at: Option<String>,
    /// Username of the release author.
    pub author: String,
    /// GitHub HTML URL for the release.
    pub url: String,
    /// URL to download the source as tarball.
    pub tarball_url: Option<String>,
    /// URL to download the source as zipball.
    pub zipball_url: Option<String>,
    /// Assets attached to this release.
    pub assets: Vec<ReleaseAsset>,
}

/// Release asset (downloadable file attached to a release).
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseAsset {
    /// Unique asset ID.
    pub id: u64,
    /// Asset filename.
    pub name: String,
    /// Optional label for the asset.
    pub label: Option<String>,
    /// MIME content type.
    pub content_type: String,
    /// Size in bytes.
    pub size: u64,
    /// Number of times downloaded.
    pub download_count: u64,
    /// Direct download URL.
    pub download_url: String,
    /// ISO 8601 timestamp when the asset was created.
    pub created_at: String,
    /// ISO 8601 timestamp when the asset was last updated.
    pub updated_at: String,
}

/// Tag information for repositories that use tags without releases.
#[derive(Debug, Clone, Serialize)]
pub struct TagInfo {
    /// Tag name (e.g., "v1.0.0").
    pub name: String,
    /// SHA of the commit this tag points to.
    pub commit_sha: String,
    /// URL to the commit.
    pub commit_url: String,
    /// URL to download source as tarball.
    pub tarball_url: String,
    /// URL to download source as zipball.
    pub zipball_url: String,
}

// ============================================================================
// GitHub API Response Structs (private)
// ============================================================================

#[derive(Deserialize)]
struct ReleaseResponse {
    id: u64,
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    created_at: String,
    published_at: Option<String>,
    author: UserResponse,
    html_url: String,
    tarball_url: Option<String>,
    zipball_url: Option<String>,
    assets: Vec<AssetResponse>,
}

#[derive(Deserialize)]
struct AssetResponse {
    id: u64,
    name: String,
    label: Option<String>,
    content_type: String,
    size: u64,
    download_count: u64,
    browser_download_url: String,
    created_at: String,
    updated_at: String,
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
}

#[derive(Deserialize)]
struct TagResponse {
    name: String,
    commit: TagCommitRef,
    tarball_url: String,
    zipball_url: String,
}

#[derive(Deserialize)]
struct TagCommitRef {
    sha: String,
    url: String,
}

// ============================================================================
// Public API Functions
// ============================================================================

/// List releases in a repository.
///
/// Retrieves releases sorted by creation date (newest first).
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `include_drafts` - Whether to include draft releases (requires auth)
/// * `max_results` - Maximum number of releases to return
///
/// # Returns
///
/// Vector of release information sorted by date (newest first).
///
/// # Example
///
/// ```no_run
/// use gitctx::github::releases::list_releases;
///
/// let releases = list_releases(&client, "rust-lang", "rust", false, 10).await?;
/// for release in releases {
///     println!("{}: {}", release.tag_name, release.name.unwrap_or_default());
/// }
/// ```
pub async fn list_releases(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    include_drafts: bool,
    max_results: usize,
) -> Result<Vec<ReleaseInfo>> {
    let max_results = max_results.min(MAX_RELEASE_RESULTS);
    let endpoint = format!(
        "/repos/{}/{}/releases?per_page={}",
        owner, repo, max_results
    );

    let releases: Vec<ReleaseResponse> = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to list releases: {}", e))?;

    let releases: Vec<ReleaseInfo> = releases
        .into_iter()
        .filter(|r| include_drafts || !r.draft)
        .map(release_response_to_info)
        .collect();

    Ok(releases)
}

/// Get a specific release by tag name.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `tag` - Tag name (e.g., "v1.0.0")
///
/// # Returns
///
/// Full release information including body and assets.
///
/// # Errors
///
/// Returns an error if the release doesn't exist.
pub async fn get_release(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    tag: &str,
) -> Result<ReleaseInfo> {
    let endpoint = format!(
        "/repos/{}/{}/releases/tags/{}",
        owner,
        repo,
        urlencoding::encode(tag)
    );

    let release: ReleaseResponse = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get release {}: {}", tag, e))?;

    Ok(release_response_to_info(release))
}

/// Get the latest release for a repository.
///
/// Returns the most recent non-draft, non-prerelease release.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
///
/// # Returns
///
/// The latest release information.
///
/// # Errors
///
/// Returns an error if no releases exist.
pub async fn get_latest_release(client: &Octocrab, owner: &str, repo: &str) -> Result<ReleaseInfo> {
    let endpoint = format!("/repos/{}/{}/releases/latest", owner, repo);

    let release: ReleaseResponse = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get latest release: {}", e))?;

    Ok(release_response_to_info(release))
}

/// List tags in a repository.
///
/// Useful for repositories that use tags without formal releases.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `max_results` - Maximum number of tags to return
///
/// # Returns
///
/// Vector of tag information.
pub async fn list_tags(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    max_results: usize,
) -> Result<Vec<TagInfo>> {
    let max_results = max_results.min(MAX_RELEASE_RESULTS);
    let endpoint = format!("/repos/{}/{}/tags?per_page={}", owner, repo, max_results);

    let tags: Vec<TagResponse> = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to list tags: {}", e))?;

    Ok(tags
        .into_iter()
        .map(|t| TagInfo {
            name: t.name,
            commit_sha: t.commit.sha,
            commit_url: t.commit.url,
            tarball_url: t.tarball_url,
            zipball_url: t.zipball_url,
        })
        .collect())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a ReleaseResponse to ReleaseInfo.
fn release_response_to_info(release: ReleaseResponse) -> ReleaseInfo {
    ReleaseInfo {
        id: release.id,
        tag_name: release.tag_name,
        name: release.name,
        body: release.body,
        draft: release.draft,
        prerelease: release.prerelease,
        created_at: release.created_at,
        published_at: release.published_at,
        author: release.author.login,
        url: release.html_url,
        tarball_url: release.tarball_url,
        zipball_url: release.zipball_url,
        assets: release
            .assets
            .into_iter()
            .map(|a| ReleaseAsset {
                id: a.id,
                name: a.name,
                label: a.label,
                content_type: a.content_type,
                size: a.size,
                download_count: a.download_count,
                download_url: a.browser_download_url,
                created_at: a.created_at,
                updated_at: a.updated_at,
            })
            .collect(),
    }
}
