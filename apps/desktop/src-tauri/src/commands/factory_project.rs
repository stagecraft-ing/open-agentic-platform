// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Tauri commands for OPC's Factory Open path (spec 112 §4 + §6.3).
//!
//! - `detect_factory_project` — thin shim over the `factory-project-detect`
//!   crate. OPC invokes it when a user opens a folder (File → Open,
//!   recents menu, or workspace-sync surfaced project) to decide whether
//!   the Factory Cockpit should light up.
//! - `fetch_project_opc_bundle` — Spec 112 §6.3. Fetches the
//!   adapter/contracts/processes/agents bundle from stagecraft for a
//!   project the user has activated via an `opc://` deep link or via the
//!   workspace project list. The bundle is what populates the cockpit
//!   on the OPC side.

use factory_project_detect::{FactoryProject, detect};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::State;

use super::stagecraft_client::{OpcBundleResponse, StagecraftState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<FactoryProject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[tauri::command]
pub fn detect_factory_project(request: DetectRequest) -> DetectResponse {
    let path = PathBuf::from(&request.path);
    match detect(&path) {
        Ok(project) => DetectResponse {
            ok: true,
            project: Some(project),
            error: None,
        },
        Err(e) => DetectResponse {
            ok: false,
            project: None,
            error: Some(e.to_string()),
        },
    }
}

// ---------------------------------------------------------------------------
// Spec 112 §6.3 — OPC handoff bundle fetch
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchBundleRequest {
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchBundleResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle: Option<OpcBundleResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[tauri::command]
pub async fn fetch_project_opc_bundle(
    request: FetchBundleRequest,
    state: State<'_, StagecraftState>,
) -> Result<FetchBundleResponse, String> {
    let client = match state.current() {
        Some(c) => c,
        None => {
            return Ok(FetchBundleResponse {
                ok: false,
                bundle: None,
                error: Some(
                    "Stagecraft client is not configured. Set the stagecraft base URL \
                     in OPC settings before fetching project bundles."
                        .into(),
                ),
            });
        }
    };

    match client.get_project_opc_bundle(&request.project_id).await {
        Ok(bundle) => Ok(FetchBundleResponse {
            ok: true,
            bundle: Some(bundle),
            error: None,
        }),
        Err(e) => Ok(FetchBundleResponse {
            ok: false,
            bundle: None,
            error: Some(e.to_string()),
        }),
    }
}

// ---------------------------------------------------------------------------
// Spec 112 §6.3 — local clone of a handed-off project
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneProjectRequest {
    /// HTTPS or SSH clone URL from the bundle (`bundle.repo.cloneUrl`).
    pub clone_url: String,
    /// Absolute target directory. Convention: `<home>/oap-projects/<slug>`.
    pub target_dir: String,
    /// Optional default branch (`bundle.repo.defaultBranch`); when set,
    /// `git clone --branch <branch>` is used. Falls back to remote
    /// HEAD when omitted.
    #[serde(default)]
    pub default_branch: Option<String>,
    /// Spec 112 §6.4.3 — short-lived clone token from the bundle's
    /// `cloneToken.value`. When set, the clone URL is rewritten to
    /// `https://x-access-token:<token>@…` for the subprocess only;
    /// `git config` is scrubbed after clone so the token does not
    /// land in `.git/config`. None = anonymous clone (public repos).
    #[serde(default)]
    pub github_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneProjectResponse {
    pub ok: bool,
    /// Absolute path to the resolved working tree (the clone target,
    /// or an existing checkout if `already_cloned` is true).
    pub path: String,
    /// True when the target directory already contained a `.git` directory
    /// pointing at the same `clone_url`. Idempotent — caller can proceed
    /// to detection / cockpit activation either way.
    pub already_cloned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Clone a stagecraft-handed-off project to a local directory.
///
/// When `github_token` is supplied (spec 112 §6.4.3), the HTTPS clone URL
/// is rewritten to embed `x-access-token:<token>@…` for the duration of
/// the subprocess. After a successful clone we set `git remote origin`
/// back to the bare URL so the token does not sit in `.git/config`.
/// All subprocess output is scrubbed of token substrings before logging
/// or surfacing to the UI. When `github_token` is None, the clone is
/// anonymous (existing osxkeychain / GCM / SSH-agent fallbacks still
/// apply for repos that have host-level credentials configured).
///
/// Clone failures (network, auth, target collision) surface as
/// `{ ok: false, error }` rather than panics so the inbox UI can render
/// them inline. Successful clones return the resolved absolute path the
/// frontend hands to `detect_factory_project` / `ProjectCockpit`.
#[tauri::command]
pub async fn clone_project_from_bundle(
    request: CloneProjectRequest,
) -> Result<CloneProjectResponse, String> {
    let target = PathBuf::from(&request.target_dir);

    // Idempotency: an existing checkout of the same remote is treated
    // as success. We do not pull/fetch — that's a separate user action.
    if target.exists() {
        match existing_remote_origin(&target) {
            Some(existing) if same_remote(&existing, &request.clone_url) => {
                return Ok(CloneProjectResponse {
                    ok: true,
                    path: target.to_string_lossy().to_string(),
                    already_cloned: true,
                    error: None,
                });
            }
            Some(other) => {
                return Ok(CloneProjectResponse {
                    ok: false,
                    path: target.to_string_lossy().to_string(),
                    already_cloned: false,
                    error: Some(format!(
                        "target directory already contains a different repo (origin={}); \
                         pick a different path or remove the existing checkout",
                        scrub_token(&other, request.github_token.as_deref())
                    )),
                });
            }
            None => {
                // Directory exists but isn't a git repo — refuse to clobber.
                return Ok(CloneProjectResponse {
                    ok: false,
                    path: target.to_string_lossy().to_string(),
                    already_cloned: false,
                    error: Some(
                        "target directory exists and is not a git repo; \
                         pick a different path or remove the existing directory"
                            .into(),
                    ),
                });
            }
        }
    }

    // Ensure parent directory exists so `git clone` doesn't bail.
    if let Some(parent) = target.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create parent dir failed: {e}"))?;
    }

    let bare_url = request.clone_url.clone();
    let auth_url = match request.github_token.as_deref() {
        Some(tok) if !tok.is_empty() => inject_token_into_https_url(&bare_url, tok),
        _ => bare_url.clone(),
    };

    let mut args: Vec<String> =
        vec!["clone".into(), "--depth".into(), "1".into()];
    if let Some(branch) = request.default_branch.as_deref()
        && !branch.is_empty()
    {
        args.push("--branch".into());
        args.push(branch.to_string());
    }
    args.push(auth_url);
    args.push(target.to_string_lossy().to_string());

    let output = Command::new("git")
        .args(&args)
        .output()
        .map_err(|e| format!("failed to spawn git: {e}"))?;

    if !output.status.success() {
        let stderr_raw = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stderr = scrub_token(&stderr_raw, request.github_token.as_deref());
        return Ok(CloneProjectResponse {
            ok: false,
            path: target.to_string_lossy().to_string(),
            already_cloned: false,
            error: Some(format!(
                "git clone exited {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            )),
        });
    }

    // Spec 112 §6.4.3 — make the token-bearing URL invariant subprocess-
    // local. Reset `origin` to the bare URL so `.git/config` carries no
    // credential. Modern git already drops the embedded auth on its own;
    // the explicit reset is a belt-and-braces guarantee.
    if request.github_token.as_deref().is_some_and(|t| !t.is_empty()) {
        let reset = Command::new("git")
            .args([
                "-C",
                &target.to_string_lossy(),
                "remote",
                "set-url",
                "origin",
                &bare_url,
            ])
            .output();
        if let Ok(out) = reset
            && !out.status.success()
        {
            // Non-fatal: the clone succeeded. Surface the warning in the
            // error field so the cockpit can show it without rolling back.
            let stderr_raw = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Ok(CloneProjectResponse {
                ok: true,
                path: target.to_string_lossy().to_string(),
                already_cloned: false,
                error: Some(format!(
                    "clone succeeded but failed to reset remote origin to bare URL: {}",
                    scrub_token(&stderr_raw, request.github_token.as_deref())
                )),
            });
        }
    }

    Ok(CloneProjectResponse {
        ok: true,
        path: target.to_string_lossy().to_string(),
        already_cloned: false,
        error: None,
    })
}

/// Spec 112 §6.4.3 — embed a clone token in an HTTPS URL.
///
/// Only `https://` URLs are rewritten; SSH and other schemes pass
/// through unchanged (the token is meaningless there). If the URL
/// already carries credentials, they are replaced — we don't support
/// the `user:pass@user2:pass2@host` shape.
fn inject_token_into_https_url(url: &str, token: &str) -> String {
    let Some(rest) = url.strip_prefix("https://") else {
        return url.to_string();
    };
    // Drop any pre-existing `userinfo@` so we don't double-stack creds.
    let rest_no_creds = match rest.split_once('@') {
        Some((_creds, host_path)) => host_path,
        None => rest,
    };
    format!("https://x-access-token:{token}@{rest_no_creds}")
}

/// Replace token substrings with a fixed redaction marker. Empty or
/// `None` token = no rewrite (avoids redacting random short strings
/// in the unauthed path).
fn scrub_token(text: &str, token: Option<&str>) -> String {
    match token {
        Some(t) if !t.is_empty() => text.replace(t, "<redacted>"),
        _ => text.to_string(),
    }
}

fn existing_remote_origin(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["-C", &path.to_string_lossy(), "remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if url.is_empty() { None } else { Some(url) }
}

/// Treat `https://github.com/o/r.git`, `https://github.com/o/r`, and
/// `git@github.com:o/r.git` as equivalent for the idempotency check.
fn same_remote(a: &str, b: &str) -> bool {
    normalise_remote(a) == normalise_remote(b)
}

fn normalise_remote(url: &str) -> String {
    let trimmed = url.trim_end_matches('/').trim_end_matches(".git");
    let lower = trimmed.to_lowercase();
    if let Some(rest) = lower.strip_prefix("git@") {
        // git@github.com:owner/repo  →  github.com/owner/repo
        return rest.replacen(':', "/", 1);
    }
    if let Some(rest) = lower.strip_prefix("https://") {
        return rest.to_string();
    }
    if let Some(rest) = lower.strip_prefix("http://") {
        return rest.to_string();
    }
    if let Some(rest) = lower.strip_prefix("ssh://git@") {
        return rest.to_string();
    }
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_remote_aliases_https_ssh_short_forms() {
        assert_eq!(
            normalise_remote("https://github.com/Acme/Foo.git"),
            "github.com/acme/foo"
        );
        assert_eq!(
            normalise_remote("https://github.com/acme/foo"),
            "github.com/acme/foo"
        );
        assert_eq!(
            normalise_remote("git@github.com:acme/foo.git"),
            "github.com/acme/foo"
        );
        assert_eq!(
            normalise_remote("ssh://git@github.com/acme/foo.git"),
            "github.com/acme/foo"
        );
    }

    #[test]
    fn same_remote_treats_aliases_as_equal() {
        assert!(same_remote(
            "https://github.com/acme/foo.git",
            "git@github.com:acme/foo.git"
        ));
        assert!(same_remote(
            "https://github.com/acme/foo",
            "https://github.com/acme/foo.git"
        ));
        assert!(!same_remote(
            "https://github.com/acme/foo.git",
            "https://github.com/acme/bar.git"
        ));
    }

    #[test]
    fn inject_token_rewrites_https_url() {
        assert_eq!(
            inject_token_into_https_url("https://github.com/acme/foo.git", "ghs_TOKEN"),
            "https://x-access-token:ghs_TOKEN@github.com/acme/foo.git"
        );
    }

    #[test]
    fn inject_token_replaces_pre_existing_creds() {
        // Caller supplies a URL that already has a userinfo segment;
        // we replace it rather than stack on top.
        assert_eq!(
            inject_token_into_https_url("https://stale-user:stale-pw@github.com/acme/foo.git", "ghs_NEW"),
            "https://x-access-token:ghs_NEW@github.com/acme/foo.git"
        );
    }

    #[test]
    fn inject_token_passes_through_non_https_urls() {
        // SSH URLs cannot use the userinfo trick; pass through unchanged.
        assert_eq!(
            inject_token_into_https_url("git@github.com:acme/foo.git", "ghs_TOKEN"),
            "git@github.com:acme/foo.git"
        );
        assert_eq!(
            inject_token_into_https_url("ssh://git@github.com/acme/foo.git", "ghs_TOKEN"),
            "ssh://git@github.com/acme/foo.git"
        );
    }

    #[test]
    fn scrub_token_redacts_token_substrings() {
        let raw = "fatal: could not read Username for 'https://x-access-token:ghs_SECRET@github.com'";
        let cleaned = scrub_token(raw, Some("ghs_SECRET"));
        assert!(!cleaned.contains("ghs_SECRET"));
        assert!(cleaned.contains("<redacted>"));
    }

    #[test]
    fn scrub_token_passes_through_when_no_token() {
        let raw = "some unrelated error";
        assert_eq!(scrub_token(raw, None), raw);
        assert_eq!(scrub_token(raw, Some("")), raw);
    }
}
