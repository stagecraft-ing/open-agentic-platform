//! OS keychain storage for Rauthy JWT session tokens (spec 087 Phase 5).
//!
//! Stores the Rauthy-issued JWT in the platform's native credential store:
//! - macOS: Keychain
//! - Windows: Credential Manager
//! - Linux: Secret Service (GNOME Keyring, KWallet)
//!
//! The service name is "dev.opc.stagecraft" and the username corresponds
//! to the user's OAP user ID, so multiple accounts can coexist.

use super::result::AppResult;

const SERVICE_NAME: &str = "dev.opc.stagecraft";
const DEFAULT_USER: &str = "session";

/// Store a JWT in the OS keychain.
#[tauri::command]
#[specta::specta]
pub fn keychain_store(token: String, user: Option<String>) -> AppResult<()> {
    let username = user.as_deref().unwrap_or(DEFAULT_USER);
    let entry = keyring::Entry::new(SERVICE_NAME, username)
        .map_err(|e| format!("keychain entry error: {e}"))?;
    entry
        .set_password(&token)
        .map_err(|e| format!("keychain store error: {e}"))?;
    Ok(())
}

/// Retrieve a JWT from the OS keychain. Returns None if not found.
#[tauri::command]
#[specta::specta]
pub fn keychain_retrieve(user: Option<String>) -> AppResult<Option<String>> {
    let username = user.as_deref().unwrap_or(DEFAULT_USER);
    let entry = keyring::Entry::new(SERVICE_NAME, username)
        .map_err(|e| format!("keychain entry error: {e}"))?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("keychain retrieve error: {e}")),
    }
}

/// Clear a JWT from the OS keychain.
#[tauri::command]
#[specta::specta]
pub fn keychain_clear(user: Option<String>) -> AppResult<()> {
    let username = user.as_deref().unwrap_or(DEFAULT_USER);
    let entry = keyring::Entry::new(SERVICE_NAME, username)
        .map_err(|e| format!("keychain entry error: {e}"))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // already cleared
        Err(e) => Err(format!("keychain clear error: {e}")),
    }
}
