//! GitHub authentication via Personal Access Token (PAT) and token management.
//!
//! This module provides secure token storage and retrieval, with support for:
//! - Environment variables (GITHUB_TOKEN, GH_TOKEN) - highest priority
//! - System keyring (OS-level secure storage)
//! - File-based fallback (~/.config/gitctx/token.json)
//!
//! For interactive authentication, the user is prompted to create a PAT
//! on GitHub and paste it into the terminal.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// Note: urlencoding is available if needed for dynamic URL parts in the future

/// URL for creating a new GitHub Personal Access Token
const GITHUB_TOKEN_URL: &str =
    "https://github.com/settings/tokens/new?description=gitctx&scopes=repo,read:org,read:user";

/// Stored token with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredToken {
    /// The GitHub access token.
    access_token: String,
    /// Unix timestamp when the token was stored.
    created_at: i64,
    /// Optional token type (usually "bearer").
    #[serde(default)]
    token_type: Option<String>,
}

/// Authentication status information.
#[derive(Debug)]
pub struct AuthStatus {
    /// Whether authentication is available.
    pub authenticated: bool,
    /// Where the token is stored.
    pub source: Option<String>,
    /// When the token was stored (if from file).
    pub stored_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Get the gitctx configuration directory.
///
/// Creates the directory if it doesn't exist.
///
/// # Returns
///
/// Path to ~/.config/gitctx/ (or platform equivalent)
pub fn config_dir() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.push("gitctx");
    fs::create_dir_all(&dir).ok();
    dir
}

/// Get the path to the token file.
fn token_file_path() -> PathBuf {
    config_dir().join("token.json")
}

/// Get the stored GitHub token, checking multiple sources.
///
/// Checks in order of priority:
/// 1. GITHUB_TOKEN environment variable
/// 2. GH_TOKEN environment variable (gh CLI compatibility)
/// 3. System keyring
/// 4. Config file (~/.config/gitctx/token.json)
///
/// # Returns
///
/// - `Ok(Some(token))` if a token was found
/// - `Ok(None)` if no token is available
/// - `Err` if there was an error reading the token
///
/// # Examples
///
/// ```no_run
/// use gitctx::auth::github::get_token;
///
/// if let Some(token) = get_token()? {
///     println!("Found token");
/// }
/// ```
pub fn get_token() -> Result<Option<String>> {
    // 1. Environment variable - GITHUB_TOKEN (highest priority)
    if let Ok(env_tok) = std::env::var("GITHUB_TOKEN") {
        if !env_tok.trim().is_empty() {
            return Ok(Some(env_tok));
        }
    }

    // 2. GH_TOKEN (GitHub CLI compatibility)
    if let Ok(env_tok) = std::env::var("GH_TOKEN") {
        if !env_tok.trim().is_empty() {
            return Ok(Some(env_tok));
        }
    }

    // 3. Config file
    let path = token_file_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(stored) = serde_json::from_str::<StoredToken>(&content) {
            if !stored.access_token.trim().is_empty() {
                return Ok(Some(stored.access_token));
            }
        }
    }

    Ok(None)
}

/// Get authentication status including source information.
///
/// # Returns
///
/// AuthStatus with details about the current authentication state.
pub fn get_auth_status() -> Result<AuthStatus> {
    // Check environment variables
    if let Ok(env_tok) = std::env::var("GITHUB_TOKEN") {
        if !env_tok.trim().is_empty() {
            return Ok(AuthStatus {
                authenticated: true,
                source: Some("GITHUB_TOKEN environment variable".to_string()),
                stored_at: None,
            });
        }
    }

    if let Ok(env_tok) = std::env::var("GH_TOKEN") {
        if !env_tok.trim().is_empty() {
            return Ok(AuthStatus {
                authenticated: true,
                source: Some("GH_TOKEN environment variable".to_string()),
                stored_at: None,
            });
        }
    }

    // Check config file
    let path = token_file_path();
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(stored) = serde_json::from_str::<StoredToken>(&content) {
            if !stored.access_token.trim().is_empty() {
                let stored_at = chrono::DateTime::from_timestamp(stored.created_at, 0);
                return Ok(AuthStatus {
                    authenticated: true,
                    source: Some(format!("Config file: {}", path.display())),
                    stored_at,
                });
            }
        }
    }

    Ok(AuthStatus {
        authenticated: false,
        source: None,
        stored_at: None,
    })
}

/// Save a token to persistent storage.
///
/// # Security Note
///
/// The token is stored in a JSON file with 0600 permissions on Unix.
/// For higher security, consider using the system keyring (keyring crate).
/// The file location is ~/.config/gitctx/token.json
///
/// # Arguments
///
/// * `token` - The GitHub access token to save
fn save_token(token: &str) -> Result<()> {
    // Use file-based storage (keyring is unreliable across processes on macOS)
    let path = token_file_path();
    let data = StoredToken {
        access_token: token.to_string(),
        created_at: chrono::Utc::now().timestamp(),
        token_type: Some("bearer".to_string()),
    };

    let json = serde_json::to_string_pretty(&data)?;
    fs::write(&path, json)?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

/// Remove stored GitHub credentials.
///
/// Removes the token from the config file.
///
/// # Examples
///
/// ```no_run
/// use gitctx::auth::github::logout;
///
/// logout()?;
/// println!("Logged out");
/// ```
pub fn logout() -> Result<()> {
    let path = token_file_path();
    if path.exists() {
        fs::remove_file(&path)?;
    }

    Ok(())
}

/// Ensure a GitHub token is available, prompting for PAT creation if needed.
///
/// This function first checks for an existing token. If none is found, it
/// prompts the user to create a Personal Access Token:
///
/// 1. Opens the user's browser to GitHub's token creation page
/// 2. Prompts the user to paste the generated token
/// 3. Validates the token via GitHub API
/// 4. Saves the token to keyring or config file
///
/// # Returns
///
/// The GitHub access token (either existing or newly obtained).
///
/// # Errors
///
/// Returns an error if:
/// - The token is invalid or empty
/// - Token validation fails
///
/// # Examples
///
/// ```no_run
/// use gitctx::auth::github::ensure_token_interactive;
///
/// let token = ensure_token_interactive().await?;
/// println!("Authenticated!");
/// ```
pub async fn ensure_token_interactive() -> Result<String> {
    // Check for existing token first
    if let Some(tok) = get_token()? {
        return Ok(tok);
    }

    // No token found, prompt user to create a PAT
    eprintln!();
    eprintln!("  No GitHub token found.");
    eprintln!();
    eprintln!("  To authenticate, create a Personal Access Token:");
    eprintln!();
    eprintln!("  1. Open: {}", GITHUB_TOKEN_URL);
    eprintln!("  2. Generate a new token with 'repo', 'read:org', 'read:user' scopes");
    eprintln!("  3. Copy the token and paste it below");
    eprintln!();

    // Try to open browser automatically
    if open::that(GITHUB_TOKEN_URL).is_ok() {
        eprintln!("  (Opening browser...)");
        eprintln!();
    }

    // Read token from stdin
    eprint!("  Enter your GitHub token: ");
    std::io::Write::flush(&mut std::io::stderr())?;

    let mut token = String::new();
    std::io::stdin().read_line(&mut token)?;
    let token = token.trim().to_string();

    if token.is_empty() {
        return Err(anyhow!(
            "No token provided. You can also set the GITHUB_TOKEN environment variable."
        ));
    }

    // Validate the token by making a test API call
    eprintln!();
    eprintln!("  Validating token...");

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "gitctx/0.1.0")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .context("Failed to validate token")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Invalid token. GitHub returned status {}. Please check your token and try again.",
            response.status()
        ));
    }

    // Check token scopes from response headers
    let scopes = response
        .headers()
        .get("x-oauth-scopes")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let required_scopes = ["repo", "read:org", "read:user"];
    let has_required = required_scopes.iter().all(|s| scopes.contains(s));

    if !has_required {
        eprintln!();
        eprintln!("  Warning: Token may be missing required scopes.");
        eprintln!("  Required: repo, read:org, read:user");
        eprintln!(
            "  Found: {}",
            if scopes.is_empty() { "(none)" } else { scopes }
        );
        eprintln!();
    }

    // Parse user info for confirmation
    #[derive(Deserialize)]
    struct UserInfo {
        login: String,
    }

    let user: UserInfo = response.json().await.context("Failed to parse user info")?;

    // Save the token
    save_token(&token)?;

    eprintln!();
    eprintln!("  Authentication successful! Logged in as: {}", user.login);
    eprintln!();

    Ok(token)
}
