//! Sidecar lifecycle and port discovery.
//!
//! Both sidecars (axiomregent, gitctx-mcp) announce their listening port
//! on stdout as a single line:
//!   `OPC_AXIOMREGENT_PORT=<port>`
//!   `OPC_GITCTX_PORT=<port>`
//!
//! `SidecarState` is managed via Tauri and holds the discovered ports.
//! The frontend queries them via `get_sidecar_ports`.

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

// ============================================================================
// Managed state
// ============================================================================

/// Holds the dynamically-discovered ports for each sidecar.
/// `None` until the sidecar has started and announced its port.
#[derive(Default)]
pub struct SidecarState {
    pub axiomregent_port: Arc<Mutex<Option<u16>>>,
    pub gitctx_port: Arc<Mutex<Option<u16>>>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct SidecarPorts {
    pub axiomregent: Option<u16>,
    pub gitctx: Option<u16>,
}

/// Returns the currently-known ports for all sidecars.
/// A `None` entry means the sidecar hasn't announced its port yet.
#[tauri::command]
#[specta::specta]
pub fn get_sidecar_ports(state: State<'_, SidecarState>) -> SidecarPorts {
    SidecarPorts {
        axiomregent: *state.axiomregent_port.lock().unwrap(),
        gitctx: *state.gitctx_port.lock().unwrap(),
    }
}

// ============================================================================
// Spawn helpers
// ============================================================================

/// Spawn axiomregent and watch stdout for `OPC_AXIOMREGENT_PORT=<port>`.
pub fn spawn_axiomregent(app: &AppHandle) {
    let port_slot = Arc::clone(&app.state::<SidecarState>().axiomregent_port);
    let cmd = match app.shell().sidecar("axiomregent") {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to create axiomregent sidecar command: {e}");
            return;
        }
    };
    tauri::async_runtime::spawn(async move {
        let (mut rx, _child) = match cmd.spawn() {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to spawn axiomregent: {e}");
                return;
            }
        };
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(bytes) => {
                    let line = String::from_utf8_lossy(&bytes);
                    if let Some(port_str) = line.trim().strip_prefix("OPC_AXIOMREGENT_PORT=") {
                        if let Ok(port) = port_str.parse::<u16>() {
                            *port_slot.lock().unwrap() = Some(port);
                            log::info!("axiomregent listening on port {port}");
                            break;
                        }
                    }
                }
                CommandEvent::Error(e) => {
                    log::error!("axiomregent error: {e}");
                    break;
                }
                CommandEvent::Terminated(status) => {
                    log::warn!("axiomregent terminated: {status:?}");
                    break;
                }
                _ => {}
            }
        }
    });
}

/// Spawn gitctx-mcp and watch stdout for `OPC_GITCTX_PORT=<port>`.
pub fn spawn_gitctx(app: &AppHandle) {
    let port_slot = Arc::clone(&app.state::<SidecarState>().gitctx_port);
    let cmd = match app.shell().sidecar("gitctx-mcp") {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to create gitctx-mcp sidecar command: {e}");
            return;
        }
    };
    tauri::async_runtime::spawn(async move {
        let (mut rx, _child) = match cmd.spawn() {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to spawn gitctx-mcp: {e}");
                return;
            }
        };
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(bytes) => {
                    let line = String::from_utf8_lossy(&bytes);
                    if let Some(port_str) = line.trim().strip_prefix("OPC_GITCTX_PORT=") {
                        if let Ok(port) = port_str.parse::<u16>() {
                            *port_slot.lock().unwrap() = Some(port);
                            log::info!("gitctx-mcp listening on port {port}");
                            break;
                        }
                    }
                }
                CommandEvent::Error(e) => {
                    log::error!("gitctx-mcp error: {e}");
                    break;
                }
                CommandEvent::Terminated(status) => {
                    log::warn!("gitctx-mcp terminated: {status:?}");
                    break;
                }
                _ => {}
            }
        }
    });
}
