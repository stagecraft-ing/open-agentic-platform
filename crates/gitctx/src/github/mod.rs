//! GitHub API client module for repository exploration.
//!
//! This module provides functions for interacting with the GitHub API to:
//! - List directory contents
//! - Read file contents
//! - Search code
//! - List and switch branches
//! - Search for repositories
//! - Search and read issues
//! - Search and read pull requests
//!
//! All API calls are made through the octocrab client with optional authentication.

use std::time::Duration;
use tokio::time::sleep;

pub mod client;
pub mod commits;
pub mod issues;
pub mod pulls;
pub mod releases;
pub mod search;
pub mod stats;

pub use client::{create_client, create_shared_client, get_repo_info, list_branches};
pub use issues::IssueSearchFilters;
pub use pulls::PRSearchFilters;

/// Rate limit information from GitHub API.
///
/// Contains the rate limit quota, remaining requests, and reset time.
/// This can be used to monitor API usage and avoid hitting rate limits.
///
/// # Example
///
/// ```no_run
/// use gitctx::github::RateLimitInfo;
///
/// let info = RateLimitInfo {
///     limit: 5000,
///     remaining: 4500,
///     reset_at: chrono::Utc::now() + chrono::Duration::hours(1),
/// };
///
/// if is_rate_limited(info.remaining, info.limit) {
///     println!("Warning: approaching rate limit");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Maximum number of requests allowed in the rate limit window.
    pub limit: u32,
    /// Number of requests remaining in the current rate limit window.
    pub remaining: u32,
    /// Time when the rate limit window resets.
    pub reset_at: chrono::DateTime<chrono::Utc>,
}

/// Check if we're close to rate limit (less than 10% remaining).
///
/// This function helps determine when to back off from making API requests
/// to avoid hitting GitHub's rate limits.
///
/// # Arguments
///
/// * `remaining` - Number of requests remaining in the current window
/// * `limit` - Maximum number of requests allowed in the window
///
/// # Returns
///
/// `true` if remaining requests are less than 10% of the limit.
///
/// # Example
///
/// ```
/// use gitctx::github::is_rate_limited;
///
/// assert!(is_rate_limited(400, 5000));  // 8% remaining - rate limited
/// assert!(!is_rate_limited(4000, 5000)); // 80% remaining - not rate limited
/// ```
pub fn is_rate_limited(remaining: u32, limit: u32) -> bool {
    remaining < limit / 10
}

/// Retry a GitHub API call with exponential backoff.
///
/// This function wraps an async operation and retries it on failure with
/// exponentially increasing delays between attempts. The delays start at
/// 100ms and double with each retry (100ms, 200ms, 400ms, ...).
///
/// # Arguments
///
/// * `f` - A closure that produces a future returning `anyhow::Result<T>`
/// * `max_retries` - Maximum number of retry attempts before giving up
///
/// # Returns
///
/// The result of the successful call, or the last error if all retries failed.
///
/// # Example
///
/// ```no_run
/// use gitctx::github::retry_with_backoff;
///
/// let result = retry_with_backoff(|| async {
///     // Make GitHub API call
///     Ok(42)
/// }, 3).await?;
/// ```
///
/// # Retry Schedule
///
/// - Attempt 1: Immediate
/// - Attempt 2: After 100ms delay
/// - Attempt 3: After 200ms delay
/// - Attempt 4: After 400ms delay
/// - And so on...
pub async fn retry_with_backoff<T, F, Fut>(mut f: F, max_retries: u32) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if retries < max_retries => {
                let delay = Duration::from_millis(100 * 2u64.pow(retries));
                eprintln!(
                    "[warn] GitHub API call failed, retrying in {:?}: {}",
                    delay, e
                );
                sleep(delay).await;
                retries += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
