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
use std::path::PathBuf;
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
