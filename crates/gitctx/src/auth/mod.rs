//! Authentication module for GitHub API access.
//!
//! This module handles GitHub authentication using multiple methods:
//! 1. Environment variables (GITHUB_TOKEN, GH_TOKEN)
//! 2. System keyring (secure OS-level credential storage)
//! 3. Config file fallback (~/.config/gitctx/token.json)
//!
//! For interactive authentication, the user is prompted to create a
//! Personal Access Token on GitHub.

pub mod github;

pub use github::{ensure_token_interactive, get_token, logout};
