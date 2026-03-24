//! GitHub Repository Stats API client functions.
//!
//! This module provides functions for retrieving repository statistics
//! including contributors, language breakdown, and dependency information.
//!
//! # API Endpoints Used
//!
//! - `GET /repos/{owner}/{repo}/contributors` - List contributors
//! - `GET /repos/{owner}/{repo}` - Get repository metadata
//! - `GET /repos/{owner}/{repo}/languages` - Get language breakdown
//! - `GET /repos/{owner}/{repo}/dependency-graph/sbom` - Get dependency SBOM

use anyhow::{anyhow, Result};
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum number of contributors to return.
pub const MAX_CONTRIBUTOR_RESULTS: usize = 100;

/// Default number of contributors to return.
pub const DEFAULT_CONTRIBUTOR_RESULTS: usize = 20;

/// Maximum number of dependencies to return.
pub const MAX_DEPENDENCY_RESULTS: usize = 500;

/// Default number of dependencies to return.
pub const DEFAULT_DEPENDENCY_RESULTS: usize = 100;

/// Contributor information with commit statistics.
#[derive(Debug, Clone, Serialize)]
pub struct ContributorInfo {
    /// Rank by contribution count (1 = most contributions).
    pub rank: u32,
    /// GitHub username.
    pub login: String,
    /// Number of commits by this contributor.
    pub contributions: u32,
    /// URL to the contributor's GitHub profile.
    pub profile_url: String,
    /// URL to the contributor's avatar image.
    pub avatar_url: String,
}

/// Repository statistics including metadata and language breakdown.
#[derive(Debug, Clone, Serialize)]
pub struct RepoStats {
    /// Number of stars.
    pub stars: u32,
    /// Number of forks.
    pub forks: u32,
    /// Number of watchers.
    pub watchers: u32,
    /// Number of open issues.
    pub open_issues: u32,
    /// Repository size in KB.
    pub size_kb: u64,
    /// Default branch name.
    pub default_branch: String,
    /// ISO 8601 timestamp when repository was created.
    pub created_at: String,
    /// ISO 8601 timestamp when repository was last updated.
    pub updated_at: String,
    /// ISO 8601 timestamp when repository was last pushed to.
    pub pushed_at: String,
    /// License SPDX identifier (e.g., "MIT", "Apache-2.0").
    pub license: Option<String>,
    /// Repository topics/tags.
    pub topics: Vec<String>,
    /// Language breakdown by bytes.
    pub languages: Vec<LanguageInfo>,
    /// Primary language (most bytes).
    pub primary_language: Option<String>,
    /// Repository description.
    pub description: Option<String>,
    /// Whether the repository is archived.
    pub archived: bool,
    /// Whether the repository is a fork.
    pub is_fork: bool,
}

/// Language statistics for a repository.
#[derive(Debug, Clone, Serialize)]
pub struct LanguageInfo {
    /// Language name.
    pub name: String,
    /// Number of bytes of code in this language.
    pub bytes: u64,
    /// Percentage of total code (0-100).
    pub percentage: f32,
}

/// Dependency information from SBOM.
#[derive(Debug, Clone, Serialize)]
pub struct DependencyInfo {
    /// Package name.
    pub name: String,
    /// Package version (if specified).
    pub version: Option<String>,
    /// Package URL in purl format.
    pub package_url: Option<String>,
    /// Dependency scope: "runtime" or "development".
    pub scope: Option<String>,
    /// Relationship type: "direct" or "indirect".
    pub relationship: String,
}

/// Repository dependency graph.
#[derive(Debug, Clone, Serialize)]
pub struct DependencyGraph {
    /// SBOM format version.
    pub sbom_version: String,
    /// ISO 8601 timestamp when SBOM was generated.
    pub created_at: String,
    /// List of dependencies.
    pub dependencies: Vec<DependencyInfo>,
    /// Total number of dependencies.
    pub total_count: usize,
    /// Number of direct dependencies.
    pub direct_count: usize,
    /// Number of indirect (transitive) dependencies.
    pub indirect_count: usize,
}

// ============================================================================
// GitHub API Response Structs (private)
// ============================================================================

#[derive(Deserialize)]
struct ContributorResponse {
    login: String,
    contributions: u32,
    html_url: String,
    avatar_url: String,
}

#[derive(Deserialize)]
struct RepoResponse {
    stargazers_count: u32,
    forks_count: u32,
    watchers_count: u32,
    open_issues_count: u32,
    size: u64,
    default_branch: String,
    created_at: String,
    updated_at: String,
    pushed_at: String,
    license: Option<LicenseResponse>,
    topics: Vec<String>,
    description: Option<String>,
    archived: bool,
    fork: bool,
}

#[derive(Deserialize)]
struct LicenseResponse {
    spdx_id: Option<String>,
}

#[derive(Deserialize)]
struct SbomResponse {
    sbom: SbomData,
}

#[derive(Deserialize)]
struct SbomData {
    #[serde(rename = "spdxVersion")]
    spdx_version: String,
    #[serde(rename = "creationInfo")]
    creation_info: CreationInfo,
    packages: Vec<SbomPackage>,
}

#[derive(Deserialize)]
struct CreationInfo {
    created: String,
}

#[derive(Deserialize)]
struct SbomPackage {
    name: String,
    #[serde(rename = "versionInfo")]
    version_info: Option<String>,
    #[serde(rename = "externalRefs")]
    external_refs: Option<Vec<ExternalRef>>,
    #[serde(rename = "relationshipType")]
    relationship_type: Option<String>,
}

#[derive(Deserialize)]
struct ExternalRef {
    #[serde(rename = "referenceType")]
    reference_type: String,
    #[serde(rename = "referenceLocator")]
    reference_locator: String,
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Get top contributors to a repository.
///
/// Returns contributors sorted by number of commits (descending).
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `max_results` - Maximum number of contributors to return
///
/// # Returns
///
/// Vector of contributor information with commit counts.
///
/// # Example
///
/// ```no_run
/// use gitctx::github::stats::get_contributors;
///
/// let contributors = get_contributors(&client, "rust-lang", "rust", 10).await?;
/// for c in contributors {
///     println!("#{} {} - {} commits", c.rank, c.login, c.contributions);
/// }
/// ```
pub async fn get_contributors(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    max_results: usize,
) -> Result<Vec<ContributorInfo>> {
    let max_results = max_results.min(MAX_CONTRIBUTOR_RESULTS);
    let endpoint = format!(
        "/repos/{}/{}/contributors?per_page={}",
        owner, repo, max_results
    );

    let contributors: Vec<ContributorResponse> = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get contributors: {}", e))?;

    Ok(contributors
        .into_iter()
        .enumerate()
        .map(|(i, c)| ContributorInfo {
            rank: (i + 1) as u32,
            login: c.login,
            contributions: c.contributions,
            profile_url: c.html_url,
            avatar_url: c.avatar_url,
        })
        .collect())
}

/// Get repository statistics including language breakdown.
///
/// Combines data from the repository endpoint and languages endpoint.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
///
/// # Returns
///
/// Repository statistics including stars, forks, and language breakdown.
pub async fn get_repo_stats(client: &Octocrab, owner: &str, repo: &str) -> Result<RepoStats> {
    // Get basic repository info
    let repo_endpoint = format!("/repos/{}/{}", owner, repo);
    let repo_info: RepoResponse = client
        .get(&repo_endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get repository info: {}", e))?;

    // Get language breakdown
    let lang_endpoint = format!("/repos/{}/{}/languages", owner, repo);
    let languages: HashMap<String, u64> = client
        .get(&lang_endpoint, None::<&()>)
        .await
        .map_err(|e| anyhow!("Failed to get language info: {}", e))?;

    // Calculate language percentages
    let total_bytes: u64 = languages.values().sum();
    let mut language_info: Vec<LanguageInfo> = languages
        .into_iter()
        .map(|(name, bytes)| {
            let percentage = if total_bytes > 0 {
                (bytes as f32 / total_bytes as f32) * 100.0
            } else {
                0.0
            };
            LanguageInfo {
                name,
                bytes,
                percentage,
            }
        })
        .collect();

    // Sort by bytes descending
    language_info.sort_by(|a, b| b.bytes.cmp(&a.bytes));

    let primary_language = language_info.first().map(|l| l.name.clone());

    Ok(RepoStats {
        stars: repo_info.stargazers_count,
        forks: repo_info.forks_count,
        watchers: repo_info.watchers_count,
        open_issues: repo_info.open_issues_count,
        size_kb: repo_info.size,
        default_branch: repo_info.default_branch,
        created_at: repo_info.created_at,
        updated_at: repo_info.updated_at,
        pushed_at: repo_info.pushed_at,
        license: repo_info.license.and_then(|l| l.spdx_id),
        topics: repo_info.topics,
        languages: language_info,
        primary_language,
        description: repo_info.description,
        archived: repo_info.archived,
        is_fork: repo_info.fork,
    })
}

/// Get repository dependency graph (SBOM).
///
/// Retrieves the Software Bill of Materials (SBOM) for a repository,
/// which lists all dependencies.
///
/// # Arguments
///
/// * `client` - Authenticated Octocrab client
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `scope` - Optional scope filter: "runtime", "development", or None for all
/// * `max_results` - Maximum number of dependencies to return
///
/// # Returns
///
/// Dependency graph with all dependencies and their metadata.
///
/// # Errors
///
/// Returns an error if:
/// - The repository doesn't have Dependency Graph enabled
/// - The user doesn't have read access
/// - Authentication is not provided
///
/// # Note
///
/// This endpoint requires authentication and the Dependency Graph
/// feature to be enabled on the repository.
pub async fn get_dependency_graph(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    scope: Option<&str>,
    max_results: usize,
) -> Result<DependencyGraph> {
    let max_results = max_results.min(MAX_DEPENDENCY_RESULTS);
    let endpoint = format!("/repos/{}/{}/dependency-graph/sbom", owner, repo);

    let response: SbomResponse = client
        .get(&endpoint, None::<&()>)
        .await
        .map_err(|e| {
            if e.to_string().contains("404") {
                anyhow!(
                    "Dependency graph not available for {}/{}. \
                     Ensure the Dependency Graph feature is enabled in repository settings.",
                    owner,
                    repo
                )
            } else if e.to_string().contains("403") {
                anyhow!(
                    "Access denied to dependency graph for {}/{}. \
                     Authentication required.",
                    owner,
                    repo
                )
            } else {
                anyhow!("Failed to get dependency graph: {}", e)
            }
        })?;

    let sbom = response.sbom;

    // Convert SBOM packages to DependencyInfo
    let mut dependencies: Vec<DependencyInfo> = sbom
        .packages
        .into_iter()
        .filter_map(|pkg| {
            // Skip the root package (usually the repository itself)
            if pkg.name.contains('/') && pkg.name.starts_with(&format!("{}/{}", owner, repo)) {
                return None;
            }

            let package_url = pkg.external_refs.and_then(|refs| {
                refs.into_iter()
                    .find(|r| r.reference_type == "purl")
                    .map(|r| r.reference_locator)
            });

            // Determine scope from package URL or relationship
            let dep_scope = package_url.as_ref().map(|purl| {
                if purl.contains("scope=dev") || purl.contains("scope=development") {
                    "development".to_string()
                } else {
                    "runtime".to_string()
                }
            });

            // Determine relationship (direct vs indirect)
            let relationship = pkg
                .relationship_type
                .map(|r| {
                    if r.contains("DIRECT") {
                        "direct".to_string()
                    } else {
                        "indirect".to_string()
                    }
                })
                .unwrap_or_else(|| "direct".to_string());

            Some(DependencyInfo {
                name: pkg.name,
                version: pkg.version_info,
                package_url,
                scope: dep_scope,
                relationship,
            })
        })
        .collect();

    // Filter by scope if specified
    if let Some(scope_filter) = scope {
        dependencies.retain(|d| {
            d.scope
                .as_ref()
                .map(|s| s == scope_filter)
                .unwrap_or(scope_filter == "runtime")
        });
    }

    // Truncate to max_results
    dependencies.truncate(max_results);

    // Count direct vs indirect
    let direct_count = dependencies.iter().filter(|d| d.relationship == "direct").count();
    let indirect_count = dependencies.len() - direct_count;

    Ok(DependencyGraph {
        sbom_version: sbom.spdx_version,
        created_at: sbom.creation_info.created,
        total_count: dependencies.len(),
        direct_count,
        indirect_count,
        dependencies,
    })
}
