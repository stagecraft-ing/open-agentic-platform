//! In-memory LRU cache for GitHub API responses.
//!
//! Caches file contents and directory listings to reduce API calls
//! and improve response times for repeated queries.
//!
//! # Architecture
//!
//! The cache uses a simple time-based expiration strategy with LRU eviction
//! when capacity is reached. It is thread-safe and can be shared across
//! multiple async tasks.
//!
//! # Usage
//!
//! ```rust,ignore
//! use gitctx::cache::{ApiCache, file_cache_key};
//!
//! let cache: ApiCache<String> = ApiCache::new();
//!
//! // Store a file's contents
//! let key = file_cache_key("owner", "repo", "main", "src/lib.rs");
//! cache.insert(key.clone(), "file contents".to_string());
//!
//! // Retrieve later
//! if let Some(contents) = cache.get(&key) {
//!     println!("Cache hit: {}", contents);
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Default cache TTL (5 minutes).
///
/// This provides a reasonable balance between freshness and API efficiency
/// for typical interactive sessions.
const DEFAULT_TTL: Duration = Duration::from_secs(300);

/// Maximum cache entries.
///
/// Limits memory usage while providing sufficient capacity for typical
/// repository exploration sessions.
const MAX_ENTRIES: usize = 1000;

/// A cached value with expiration time.
///
/// Stores both the cached data and when it should be considered stale.
#[derive(Clone)]
struct CacheEntry<T> {
    /// The cached value.
    value: T,
    /// When this entry expires and should be evicted.
    expires_at: Instant,
}

/// Thread-safe LRU cache for API responses.
///
/// This cache implementation provides:
/// - Thread-safe access via `RwLock`
/// - Time-based expiration (TTL)
/// - LRU eviction when capacity is reached
/// - Clone-on-read semantics for cached values
///
/// # Type Parameters
///
/// * `T` - The type of values to cache. Must implement `Clone` for retrieval.
///
/// # Thread Safety
///
/// The cache is wrapped in `Arc<RwLock<...>>` allowing safe concurrent access
/// from multiple threads. Read operations acquire a read lock, while write
/// operations acquire an exclusive write lock.
///
/// # Eviction Strategy
///
/// When the cache reaches capacity:
/// 1. First, all expired entries are removed
/// 2. If still at capacity, the entry with the earliest expiration time is removed
/// 3. This continues until space is available for the new entry
pub struct ApiCache<T: Clone> {
    /// The underlying storage protected by a read-write lock.
    entries: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    /// Time-to-live for cache entries.
    ttl: Duration,
    /// Maximum number of entries before eviction.
    max_size: usize,
}

impl<T: Clone> ApiCache<T> {
    /// Create a new cache with default settings.
    ///
    /// Uses a TTL of 5 minutes and maximum of 1000 entries.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let cache: ApiCache<String> = ApiCache::new();
    /// ```
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl: DEFAULT_TTL,
            max_size: MAX_ENTRIES,
        }
    }

    /// Create a cache with custom TTL.
    ///
    /// # Arguments
    ///
    /// * `ttl` - How long entries should remain valid before expiring.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::time::Duration;
    ///
    /// // Cache with 1 minute TTL
    /// let cache: ApiCache<String> = ApiCache::with_ttl(Duration::from_secs(60));
    /// ```
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            ttl,
            max_size: MAX_ENTRIES,
        }
    }

    /// Get a value from cache if it exists and hasn't expired.
    ///
    /// Returns `None` if the key doesn't exist or the entry has expired.
    /// Expired entries are not automatically removed during reads to avoid
    /// blocking; they will be cleaned up during the next write operation.
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key to look up.
    ///
    /// # Returns
    ///
    /// * `Some(T)` - A clone of the cached value if found and not expired.
    /// * `None` - If not found, expired, or lock acquisition failed.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let cache: ApiCache<String> = ApiCache::new();
    /// cache.insert("key".to_string(), "value".to_string());
    ///
    /// assert_eq!(cache.get("key"), Some("value".to_string()));
    /// assert_eq!(cache.get("nonexistent"), None);
    /// ```
    pub fn get(&self, key: &str) -> Option<T> {
        let entries = self.entries.read().ok()?;
        let entry = entries.get(key)?;

        if entry.expires_at > Instant::now() {
            Some(entry.value.clone())
        } else {
            None
        }
    }

    /// Insert a value into the cache.
    ///
    /// If the cache is at capacity, expired entries are evicted first,
    /// followed by entries closest to expiration (LRU-style eviction).
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key for retrieval.
    /// * `value` - The value to cache.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let cache: ApiCache<String> = ApiCache::new();
    /// cache.insert("file:owner:repo:main:README.md".to_string(), "# Hello".to_string());
    /// ```
    ///
    /// # Panics
    ///
    /// Does not panic. If the lock is poisoned, the insert is silently skipped.
    pub fn insert(&self, key: String, value: T) {
        if let Ok(mut entries) = self.entries.write() {
            // Evict expired entries if we're at capacity
            if entries.len() >= self.max_size {
                let now = Instant::now();
                entries.retain(|_, v| v.expires_at > now);
            }

            // If still at capacity, remove oldest entries (earliest expiration)
            while entries.len() >= self.max_size {
                if let Some(oldest_key) = entries
                    .iter()
                    .min_by_key(|(_, v)| v.expires_at)
                    .map(|(k, _)| k.clone())
                {
                    entries.remove(&oldest_key);
                } else {
                    break;
                }
            }

            entries.insert(
                key,
                CacheEntry {
                    value,
                    expires_at: Instant::now() + self.ttl,
                },
            );
        }
    }

    /// Clear all cached entries.
    ///
    /// Removes all entries from the cache regardless of expiration status.
    /// Useful when the underlying data source has been modified and all
    /// cached data should be considered stale.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let cache: ApiCache<String> = ApiCache::new();
    /// cache.insert("key".to_string(), "value".to_string());
    /// cache.clear();
    /// assert!(cache.is_empty());
    /// ```
    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.clear();
        }
    }

    /// Get the number of cached entries.
    ///
    /// Note: This count includes expired entries that haven't been evicted yet.
    /// For an accurate count of valid entries, consider the timing of your reads.
    ///
    /// # Returns
    ///
    /// The number of entries in the cache, or 0 if the lock cannot be acquired.
    pub fn len(&self) -> usize {
        self.entries.read().map(|e| e.len()).unwrap_or(0)
    }

    /// Check if the cache is empty.
    ///
    /// # Returns
    ///
    /// `true` if the cache contains no entries, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Clone> Default for ApiCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a cache key for file content.
///
/// Creates a unique key for caching file contents based on the repository
/// coordinates and file path. The key format is:
/// `file:{owner}:{repo}:{branch}:{path}`
///
/// # Arguments
///
/// * `owner` - Repository owner (user or organization).
/// * `repo` - Repository name.
/// * `branch` - Branch, tag, or commit reference.
/// * `path` - File path within the repository.
///
/// # Returns
///
/// A string suitable for use as a cache key.
///
/// # Examples
///
/// ```rust,ignore
/// let key = file_cache_key("rust-lang", "rust", "master", "src/lib.rs");
/// assert_eq!(key, "file:rust-lang:rust:master:src/lib.rs");
/// ```
pub fn file_cache_key(owner: &str, repo: &str, branch: &str, path: &str) -> String {
    format!("file:{}:{}:{}:{}", owner, repo, branch, path)
}

/// Generate a cache key for directory listing.
///
/// Creates a unique key for caching directory contents based on the repository
/// coordinates and directory path. The key format is:
/// `dir:{owner}:{repo}:{branch}:{path}`
///
/// # Arguments
///
/// * `owner` - Repository owner (user or organization).
/// * `repo` - Repository name.
/// * `branch` - Branch, tag, or commit reference.
/// * `path` - Directory path within the repository.
///
/// # Returns
///
/// A string suitable for use as a cache key.
///
/// # Examples
///
/// ```rust,ignore
/// let key = dir_cache_key("rust-lang", "rust", "master", "src");
/// assert_eq!(key, "dir:rust-lang:rust:master:src");
/// ```
pub fn dir_cache_key(owner: &str, repo: &str, branch: &str, path: &str) -> String {
    format!("dir:{}:{}:{}:{}", owner, repo, branch, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let cache: ApiCache<String> = ApiCache::new();

        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_cache_expiry() {
        let cache: ApiCache<String> = ApiCache::with_ttl(Duration::from_millis(10));

        cache.insert("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get("key1"), Some("value1".to_string()));

        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_cache_clear() {
        let cache: ApiCache<String> = ApiCache::new();

        cache.insert("key1".to_string(), "value1".to_string());
        cache.insert("key2".to_string(), "value2".to_string());
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_cache_key_generation() {
        let file_key = file_cache_key("owner", "repo", "main", "src/lib.rs");
        assert_eq!(file_key, "file:owner:repo:main:src/lib.rs");

        let dir_key = dir_cache_key("owner", "repo", "main", "src");
        assert_eq!(dir_key, "dir:owner:repo:main:src");
    }

    #[test]
    fn test_cache_default() {
        let cache: ApiCache<String> = ApiCache::default();
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_overwrite() {
        let cache: ApiCache<String> = ApiCache::new();

        cache.insert("key".to_string(), "value1".to_string());
        assert_eq!(cache.get("key"), Some("value1".to_string()));

        cache.insert("key".to_string(), "value2".to_string());
        assert_eq!(cache.get("key"), Some("value2".to_string()));
    }
}
