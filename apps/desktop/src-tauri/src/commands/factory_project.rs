// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Tauri commands for OPC's Factory Open path (spec 112 §4).
//!
//! Thin shim over the `factory-project-detect` crate. OPC invokes
//! `detect_factory_project` when a user opens a folder (File → Open,
//! recents menu, or workspace-sync surfaced project) to decide whether
//! the Factory Cockpit should light up.

use factory_project_detect::{FactoryProject, detect};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
