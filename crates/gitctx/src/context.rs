//! Shared context for GitHub filesystem emulation.
//!
//! This module provides a thread-safe context that tracks the current state
//! of the GitHub "filesystem" being explored. All tools share this context
//! to maintain consistency across tool invocations.
//!
//! # State Tracked
//!
//! - Current repository (owner/name)
//! - Current branch
//! - Current working directory path within the repo
//! - GitHub authentication token
//! - Shared Octocrab client for API calls
//!
//! # Thread Safety
//!
//! The context uses `Arc<RwLock>` internally, allowing safe sharing across
//! multiple tools and async boundaries.

use octocrab::Octocrab;
use std::fmt;
use std::sync::{Arc, RwLock};

/// Information about the currently selected repository.
#[derive(Debug, Clone)]
pub struct RepoInfo {
    /// Repository owner (user or organization).
    pub owner: String,
    /// Repository name.
    pub name: String,
    /// The repository's default branch (usually "main" or "master").
    pub default_branch: String,
    /// Repository description, if available.
    pub description: Option<String>,
    /// Whether the repository is private.
    pub is_private: bool,
}

impl RepoInfo {
    /// Get the full repository reference in "owner/name" format.
    #[allow(dead_code)]
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

/// Inner state of the GitHub context.
struct ContextInner {
    /// Currently selected repository.
    repo: Option<RepoInfo>,
    /// Current working directory within the repository (e.g., "/src/lib").
    current_path: String,
    /// Current branch being explored.
    current_branch: String,
    /// GitHub access token for API calls.
    token: Option<String>,
    /// Shared Octocrab client for making GitHub API calls.
    /// Using Arc to allow sharing across async boundaries without cloning.
    client: Option<Arc<Octocrab>>,
}

/// Manual Debug implementation for ContextInner since Octocrab doesn't implement Debug.
impl fmt::Debug for ContextInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContextInner")
            .field("repo", &self.repo)
            .field("current_path", &self.current_path)
            .field("current_branch", &self.current_branch)
            .field("token", &self.token.as_ref().map(|_| "[REDACTED]"))
            .field("client", &self.client.as_ref().map(|_| "<Octocrab>"))
            .finish()
    }
}

/// Thread-safe shared context for GitHub filesystem emulation.
///
/// This context is shared across all tools to maintain state between
/// tool invocations. It tracks the current repository, branch, and
/// working directory.
///
/// # Examples
///
/// ```
/// use gitctx::context::GitHubContext;
///
/// let ctx = GitHubContext::new(Some("ghp_xxx".to_string()));
/// ctx.set_repo("rust-lang", "rust", "master", None, false);
/// assert_eq!(ctx.get_current_branch(), "master");
/// ```
#[derive(Debug, Clone)]
pub struct GitHubContext {
    inner: Arc<RwLock<ContextInner>>,
}

impl GitHubContext {
    /// Create a new GitHub context with an optional authentication token.
    ///
    /// # Arguments
    ///
    /// * `token` - Optional GitHub personal access token or OAuth token
    ///
    /// # Examples
    ///
    /// ```
    /// use gitctx::context::GitHubContext;
    ///
    /// // With token
    /// let ctx = GitHubContext::new(Some("ghp_xxx".to_string()));
    ///
    /// // Without token (public repos only)
    /// let ctx = GitHubContext::new(None);
    /// ```
    pub fn new(token: Option<String>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ContextInner {
                repo: None,
                current_path: "/".to_string(),
                current_branch: "main".to_string(),
                token,
                client: None,
            })),
        }
    }

    /// Set the current repository.
    ///
    /// This also resets the current path to "/" and updates the branch
    /// to the repository's default branch.
    ///
    /// # Arguments
    ///
    /// * `owner` - Repository owner (user or organization)
    /// * `name` - Repository name
    /// * `default_branch` - The repository's default branch
    /// * `description` - Optional repository description
    /// * `is_private` - Whether the repository is private
    pub fn set_repo(
        &self,
        owner: &str,
        name: &str,
        default_branch: &str,
        description: Option<String>,
        is_private: bool,
    ) {
        let mut inner = self.inner.write().unwrap();
        inner.repo = Some(RepoInfo {
            owner: owner.to_string(),
            name: name.to_string(),
            default_branch: default_branch.to_string(),
            description,
            is_private,
        });
        inner.current_branch = default_branch.to_string();
        inner.current_path = "/".to_string();
    }

    /// Get the currently selected repository, if any.
    pub fn get_repo(&self) -> Option<RepoInfo> {
        self.inner.read().unwrap().repo.clone()
    }

    /// Check if a repository has been selected.
    #[allow(dead_code)]
    pub fn has_repo(&self) -> bool {
        self.inner.read().unwrap().repo.is_some()
    }

    /// Get the current working directory path.
    pub fn get_current_path(&self) -> String {
        self.inner.read().unwrap().current_path.clone()
    }

    /// Set the current working directory path.
    ///
    /// # Arguments
    ///
    /// * `path` - New path (should start with "/")
    #[allow(dead_code)]
    pub fn set_current_path(&self, path: &str) {
        let mut inner = self.inner.write().unwrap();
        // Normalize path to always start with /
        if path.starts_with('/') {
            inner.current_path = path.to_string();
        } else {
            inner.current_path = format!("/{}", path);
        }
    }

    /// Get the current branch.
    pub fn get_current_branch(&self) -> String {
        self.inner.read().unwrap().current_branch.clone()
    }

    /// Set the current branch.
    ///
    /// This also resets the current path to "/".
    ///
    /// # Arguments
    ///
    /// * `branch` - Branch name to switch to
    pub fn set_current_branch(&self, branch: &str) {
        let mut inner = self.inner.write().unwrap();
        inner.current_branch = branch.to_string();
        // Reset path when switching branches
        inner.current_path = "/".to_string();
    }

    /// Get the GitHub token, if available.
    pub fn get_token(&self) -> Option<String> {
        self.inner.read().unwrap().token.clone()
    }

    /// Check if authentication is available.
    #[allow(dead_code)]
    pub fn is_authenticated(&self) -> bool {
        self.inner.read().unwrap().token.is_some()
    }

    /// Set the shared Octocrab client for API calls.
    ///
    /// This allows all tools to share the same client instance,
    /// reducing overhead from creating multiple clients.
    ///
    /// # Arguments
    ///
    /// * `client` - Arc-wrapped Octocrab client to share
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use gitctx::context::GitHubContext;
    /// use gitctx::github::client::create_shared_client;
    ///
    /// let ctx = GitHubContext::new(Some("ghp_xxx".to_string()));
    /// let client = create_shared_client(Some("ghp_xxx")).unwrap();
    /// ctx.set_client(client);
    /// ```
    pub fn set_client(&self, client: Arc<Octocrab>) {
        let mut inner = self.inner.write().unwrap();
        inner.client = Some(client);
    }

    /// Get the shared Octocrab client, if available.
    ///
    /// # Returns
    ///
    /// The shared client wrapped in Arc, or None if not set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitctx::context::GitHubContext;
    ///
    /// let ctx = GitHubContext::new(None);
    /// if let Some(client) = ctx.get_client() {
    ///     // Use the shared client
    /// }
    /// ```
    pub fn get_client(&self) -> Option<Arc<Octocrab>> {
        self.inner.read().unwrap().client.clone()
    }

    /// Get a summary of the current context for display.
    ///
    /// Returns a formatted string describing the current state.
    #[allow(dead_code)]
    pub fn summary(&self) -> String {
        let inner = self.inner.read().unwrap();
        match &inner.repo {
            Some(repo) => {
                format!(
                    "Repository: {}/{} | Branch: {} | Path: {}",
                    repo.owner, repo.name, inner.current_branch, inner.current_path
                )
            }
            None => "No repository selected".to_string(),
        }
    }
}

impl Default for GitHubContext {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context() {
        let ctx = GitHubContext::new(Some("token".to_string()));
        assert!(!ctx.has_repo());
        assert_eq!(ctx.get_current_path(), "/");
        assert_eq!(ctx.get_current_branch(), "main");
        assert!(ctx.is_authenticated());
    }

    #[test]
    fn test_set_repo() {
        let ctx = GitHubContext::new(None);
        ctx.set_repo(
            "rust-lang",
            "rust",
            "master",
            Some("The Rust Programming Language".to_string()),
            false,
        );

        let repo = ctx.get_repo().unwrap();
        assert_eq!(repo.owner, "rust-lang");
        assert_eq!(repo.name, "rust");
        assert_eq!(repo.default_branch, "master");
        assert_eq!(ctx.get_current_branch(), "master");
    }

    #[test]
    fn test_branch_switch_resets_path() {
        let ctx = GitHubContext::new(None);
        ctx.set_repo("owner", "repo", "main", None, false);
        ctx.set_current_path("/src/lib");
        assert_eq!(ctx.get_current_path(), "/src/lib");

        ctx.set_current_branch("develop");
        assert_eq!(ctx.get_current_path(), "/");
    }
}
