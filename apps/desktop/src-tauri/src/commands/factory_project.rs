// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Tauri commands for OPC's Factory Open path (spec 112 §4 + §6.3).
//!
//! - `detect_factory_project` — thin shim over the `factory-project-detect`
//!   crate. OPC invokes it when a user opens a folder (File → Open,
//!   recents menu, or workspace-sync surfaced project) to decide whether
//!   the Factory Cockpit should light up.
//! - `fetch_project_oap_bundle` — Spec 112 §6.3. Fetches the
//!   adapter/contracts/processes/agents bundle from stagecraft for a
//!   project the user has activated via an `oap://` deep link or via the
//!   workspace project list. The bundle is what populates the cockpit
//!   on the OPC side.

use factory_project_detect::{FactoryProject, detect};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::State;

use super::stagecraft_client::{OapBundleResponse, StagecraftState};

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
    pub bundle: Option<OapBundleResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[tauri::command]
pub async fn fetch_project_oap_bundle(
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

    match client.get_project_oap_bundle(&request.project_id).await {
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
/// The command relies on the host's `git` binary so existing credential
/// helpers (osxkeychain, libsecret, GCM) and SSH agents continue to work.
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
                        "target directory already contains a different repo (origin={other}); \
                         pick a different path or remove the existing checkout"
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

    let mut args: Vec<String> =
        vec!["clone".into(), "--depth".into(), "1".into()];
    if let Some(branch) = request.default_branch.as_deref()
        && !branch.is_empty()
    {
        args.push("--branch".into());
        args.push(branch.to_string());
    }
    args.push(request.clone_url.clone());
    args.push(target.to_string_lossy().to_string());

    let output = Command::new("git")
        .args(&args)
        .output()
        .map_err(|e| format!("failed to spawn git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
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

    Ok(CloneProjectResponse {
        ok: true,
        path: target.to_string_lossy().to_string(),
        already_cloned: false,
        error: None,
    })
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
}
