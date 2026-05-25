//! Sidecar lifecycle and port discovery.
//!
//! **axiomregent** announces a local probe port on **stderr** as:
//!   `OPC_AXIOMREGENT_PORT=<port>`
//! (Stdout is reserved for MCP framing.)
//!
//! `SidecarState` is managed via Tauri and holds discovered ports where applicable.
//! The frontend queries them via `get_sidecar_ports`.
//!
//! Spec 183 FR-T1 (sidecar liveness gate) adds a TCP-connect-based liveness
//! check on top of the announcement parser: `check_axiomregent_alive` opens a
//! short-lived TCP connection to the announced probe port and reports whether
//! the diagnostic listener in `axiomregent::main` is still accepting. The
//! probe port is a pure liveness signal (no HTTP, no JSON-RPC payload), so
//! the binary "connect succeeded" outcome is the entire green-light contract.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use tokio::net::TcpStream;

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

/// Spec 183 FR-T1(b) — TCP-connect liveness probe against the announced
/// axiomregent port. Returns `true` when the diagnostic listener accepted
/// the connection within the configured timeout; `false` on connection
/// refused, timeout, or any other transport error. The probe port carries
/// no protocol — connection establishment is the entire green-light test.
///
/// FR-T1 explicitly excludes absolute latency budgets from the binding
/// surface; `LIVENESS_TIMEOUT` is an implementer choice that keeps the boot
/// UI responsive on a healthy local loopback while leaving room for slow
/// machines. Adjust freely — the contract is "bounded timeout," not "X ms."
const LIVENESS_TIMEOUT: Duration = Duration::from_millis(500);

#[tauri::command]
#[specta::specta]
pub async fn check_axiomregent_alive(state: State<'_, SidecarState>) -> Result<bool, String> {
    let port = match *state.axiomregent_port.lock().unwrap() {
        Some(p) => p,
        None => return Ok(false),
    };
    Ok(probe_port_alive(port).await)
}

async fn probe_port_alive(port: u16) -> bool {
    let addr = format!("127.0.0.1:{port}");
    match tokio::time::timeout(LIVENESS_TIMEOUT, TcpStream::connect(&addr)).await {
        Ok(Ok(_stream)) => true,
        // Connection refused, timeout, or any other transport failure all
        // collapse to "not alive" — the boot gate's only question is
        // whether the listener is accepting.
        _ => false,
    }
}

/// Spec 183 — unified boot-gate status query. Combines FR-T1 (sidecar
/// liveness) and FR-T2 (org session materialised + sync.hello received)
/// into a single observable the boot-state UI subscribes to. Computed
/// fresh on every call; the frontend polls (not subscribes) during the
/// boot phase per FR-T4's explicit-retry posture.
#[derive(Debug, Clone, serde::Serialize, specta::Type)]
pub struct BootGateStatus {
    /// FR-T1: probe port announced AND TCP-connect succeeds.
    pub sidecar_alive: bool,
    /// FR-T2: StagecraftState has a non-empty org_id AND the duplex
    /// consumer has received `sync.hello` from stagecraft.
    pub org_session_ready: bool,
    /// Convenience: announced probe port (None if unannounced).
    pub axiomregent_port: Option<u16>,
    /// Convenience: org_id when present, for surface display only.
    pub org_id: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn boot_gate_status(app: AppHandle) -> Result<BootGateStatus, String> {
    let port = *app
        .state::<SidecarState>()
        .axiomregent_port
        .lock()
        .unwrap();
    let sidecar_alive = match port {
        Some(p) => probe_port_alive(p).await,
        None => false,
    };

    // Org-session readiness combines org_id presence on StagecraftState
    // with sync.hello receipt on the duplex consumer. Either missing → not
    // ready. The duplex consumer is only spawned when both base URL and JWT
    // are loaded (see lib.rs); before that, sync_hello_received() is false
    // and the gate stays closed — which is exactly the expected pre-login
    // boot state.
    let (org_id, has_org) = {
        match app.try_state::<crate::commands::stagecraft_client::StagecraftState>() {
            Some(sc_state) => match sc_state.current() {
                Some(client) => {
                    let id = client.org_id();
                    let has = !id.is_empty();
                    (Some(id).filter(|s| !s.is_empty()), has)
                }
                None => (None, false),
            },
            None => (None, false),
        }
    };
    let sync_hello = app
        .try_state::<crate::commands::sync_client::SyncClientState>()
        .map(|s| s.sync_hello_received())
        .unwrap_or(false);
    let org_session_ready = has_org && sync_hello;

    Ok(BootGateStatus {
        sidecar_alive,
        org_session_ready,
        axiomregent_port: port,
        org_id,
    })
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
