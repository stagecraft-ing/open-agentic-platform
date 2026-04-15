//! Sidecar lifecycle and port discovery.
//!
//! **axiomregent** announces a local probe port on **stderr** as:
//!   `OPC_AXIOMREGENT_PORT=<port>`
//! (Stdout is reserved for MCP framing.)
//!
//! `SidecarState` is managed via Tauri and holds discovered ports where applicable.
//! The frontend queries them via `get_sidecar_ports`.

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

// ============================================================================
// Managed state
// ============================================================================

/// Holds the dynamically-discovered port for axiomregent.
/// `None` until the sidecar has started and announced its port.
#[derive(Default)]
pub struct SidecarState {
    pub axiomregent_port: Arc<Mutex<Option<u16>>>,
}

#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct SidecarPorts {
    pub axiomregent: Option<u16>,
}

/// Returns the currently-known ports for sidecars that use port discovery.
/// A `None` entry means the sidecar hasn't announced its port yet.
#[tauri::command]
#[specta::specta]
pub fn get_sidecar_ports(state: State<'_, SidecarState>) -> SidecarPorts {
    SidecarPorts {
        axiomregent: *state.axiomregent_port.lock().unwrap(),
    }
}

// ============================================================================
// Spawn helpers
// ============================================================================

/// Parse a line for `OPC_AXIOMREGENT_PORT=<u16>` (first line win).
pub fn parse_axiomregent_port_line(line: &str) -> Option<u16> {
    line.trim()
        .strip_prefix("OPC_AXIOMREGENT_PORT=")
        .and_then(|s| s.parse::<u16>().ok())
}

/// Spawn axiomregent and watch stderr for `OPC_AXIOMREGENT_PORT=<port>`.
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
                CommandEvent::Stderr(bytes) | CommandEvent::Stdout(bytes) => {
                    let line = String::from_utf8_lossy(&bytes);
                    if let Some(port) = parse_axiomregent_port_line(&line) {
                        *port_slot.lock().unwrap() = Some(port);
                        log::info!("axiomregent probe port {port}");
                        break;
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

/// Spawn axiomregent as a standalone OS process (no Tauri shell).
///
/// Used by `start_web_mode` where there is no Tauri `AppHandle`. Watches stderr
/// for the `OPC_AXIOMREGENT_PORT=<port>` announcement and writes it into
/// `port_slot`, fixing the race described in spec 090 SC-090-3.
pub fn spawn_axiomregent_standalone(port_slot: Arc<Mutex<Option<u16>>>) {
    let binary = match crate::governed_claude::bundled_axiomregent_binary_path() {
        Ok(p) => p,
        Err(e) => {
            log::warn!("axiomregent binary not available for standalone spawn: {e}");
            return;
        }
    };
    tokio::spawn(async move {
        let mut child = match tokio::process::Command::new(&binary)
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to spawn axiomregent standalone: {e}");
                return;
            }
        };
        if let Some(stderr) = child.stderr.take() {
            let reader = tokio::io::BufReader::new(stderr);
            let mut lines = tokio::io::AsyncBufReadExt::lines(reader);
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(port) = parse_axiomregent_port_line(&line) {
                    *port_slot.lock().unwrap() = Some(port);
                    log::info!("axiomregent standalone probe port {port}");
                    break;
                }
            }
        }
        // Keep the child alive — it runs for the lifetime of the web server.
        let _ = child.wait().await;
    });
}

#[cfg(test)]
mod tests {
    use super::parse_axiomregent_port_line;

    #[test]
    fn parse_port_line_accepts_stderr_style() {
        assert_eq!(
            parse_axiomregent_port_line("OPC_AXIOMREGENT_PORT=9123\n"),
            Some(9123)
        );
        assert_eq!(
            parse_axiomregent_port_line("  OPC_AXIOMREGENT_PORT=1  "),
            Some(1)
        );
        assert_eq!(parse_axiomregent_port_line("noise"), None);
    }
}
