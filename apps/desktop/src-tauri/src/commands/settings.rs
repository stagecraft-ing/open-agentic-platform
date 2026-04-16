//! App-level user-configurable settings.
//!
//! Currently manages the Stagecraft base URL so users can switch servers at
//! runtime without setting env vars. Persisted in the `app_settings` k/v table.

use log::{info, warn};
use rusqlite::params;
use tauri::{AppHandle, Manager, State};

use super::agents::AgentDb;
use super::stagecraft_client::{StagecraftClient, StagecraftState};

/// Production default when nothing is configured.
pub const DEFAULT_STAGECRAFT_BASE_URL: &str = "https://stagecraft.ing";

/// Resolve the effective Stagecraft base URL on startup:
///   1. `app_settings.stagecraft_base_url` (DB, set via UI)
///   2. `STAGECRAFT_BASE_URL` env var (dev / CI override)
///   3. `DEFAULT_STAGECRAFT_BASE_URL`
///
/// Returns an empty string only if the DB override is explicitly set to
/// empty — callers treat "" as "client disabled".
pub fn resolve_stagecraft_base_url(app: &AppHandle) -> String {
    if let Some(url) = read_stagecraft_url_from_db(app) {
        return url;
    }
    let env_url = std::env::var("STAGECRAFT_BASE_URL").unwrap_or_default();
    if !env_url.is_empty() {
        return env_url;
    }
    DEFAULT_STAGECRAFT_BASE_URL.to_string()
}

/// Read the user-set URL directly from the SQLite file. Used at startup
/// before `AgentDb` is installed as managed state.
fn read_stagecraft_url_from_db(app: &AppHandle) -> Option<String> {
    let app_data_dir = app.path().app_data_dir().ok()?;
    let db_path = app_data_dir.join("agents.db");
    if !db_path.exists() {
        return None;
    }
    let conn = rusqlite::Connection::open(&db_path).ok()?;
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = 'stagecraft_base_url'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

/// Return the base URL currently in effect (reflects the live client).
#[tauri::command]
#[specta::specta]
pub async fn get_stagecraft_base_url(stagecraft: State<'_, StagecraftState>) -> Result<String, String> {
    Ok(stagecraft
        .current()
        .map(|c| c.base_url().to_string())
        .unwrap_or_default())
}

/// Persist a new base URL, rebuild the client, and clear any stale auth
/// (tokens from a different server must not leak across).
///
/// Pass an empty string to disable the integration entirely.
#[tauri::command]
#[specta::specta]
pub async fn set_stagecraft_base_url(
    base_url: String,
    db: State<'_, AgentDb>,
    stagecraft: State<'_, StagecraftState>,
) -> Result<(), String> {
    let trimmed = base_url.trim().trim_end_matches('/').to_string();

    // Validate non-empty values parse as http(s) URLs.
    if !trimmed.is_empty()
        && !(trimmed.starts_with("http://") || trimmed.starts_with("https://"))
    {
        return Err("URL must start with http:// or https://".into());
    }

    // Persist to DB.
    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('stagecraft_base_url', ?1)",
            params![trimmed],
        )
        .map_err(|e| format!("failed to save stagecraft_base_url: {e}"))?;
    }

    // Clear keychain auth — the old token belongs to the old server.
    if let Some(old) = stagecraft.current() {
        old.clear_auth();
    }
    for key in ["session", "refresh_token"] {
        if let Ok(entry) = keyring::Entry::new("dev.opc.stagecraft", key) {
            let _ = entry.delete_credential();
        }
    }

    // Rebuild the client.
    let user_id = std::env::var("OPC_USER_ID").unwrap_or_else(|_| "opc-desktop".into());
    let new_client = StagecraftClient::new(&trimmed, &user_id);
    if new_client.is_some() {
        info!("Stagecraft base URL updated → {trimmed}");
    } else {
        warn!("Stagecraft base URL cleared — integration disabled");
    }
    stagecraft.replace(new_client);

    Ok(())
}
