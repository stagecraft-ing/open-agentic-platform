//! Desktop PKCE OAuth flow — spec 080 Phase 1.
//!
//! Implements the full GitHub-backed PKCE authentication flow for the OPC
//! desktop app. The stagecraft service provides desktop-specific endpoints:
//!   GET  /auth/desktop/authorize  — initiates GitHub OAuth, accepts PKCE params
//!   POST /auth/desktop/token      — exchanges code + code_verifier for tokens
//!   POST /auth/desktop/org-select — multi-org selection
//!   POST /auth/desktop/refresh    — token refresh with rotation
//!
//! Deep-link scheme: `opc://auth/callback?code=...&state=...`

use super::result::AppResult;
use super::stagecraft_client::StagecraftState;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

// ---------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------

/// Holds the in-flight PKCE flow parameters.
pub struct AuthFlowState(pub Mutex<Option<PkceFlow>>);

pub struct PkceFlow {
    pub state: String,
    pub code_verifier: String,
    pub started_at: std::time::Instant,
}

// ---------------------------------------------------------------------------
// Public types (exposed to the frontend via specta)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub name: String,
    #[serde(default)]
    pub github_login: String,
    #[serde(default)]
    pub idp_provider: String,
    #[serde(default)]
    pub idp_login: String,
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct AuthOrg {
    pub org_id: String,
    pub org_slug: String,
    #[serde(default)]
    pub github_org_login: String,
    #[serde(default)]
    pub org_display_name: String,
    pub platform_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum AuthResult {
    #[serde(rename = "authenticated")]
    Authenticated {
        user: AuthUser,
        org: AuthOrg,
        expires_at: i64,
    },
    #[serde(rename = "org_selection")]
    OrgSelection {
        pending_id: String,
        orgs: Vec<AuthOrg>,
        user: AuthUser,
    },
    #[serde(rename = "error")]
    Error { code: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub user: Option<AuthUser>,
    pub org: Option<AuthOrg>,
    pub expires_at: Option<i64>,
}

// ---------------------------------------------------------------------------
// Keychain helpers
// ---------------------------------------------------------------------------

const KEYCHAIN_SERVICE: &str = "dev.opc.stagecraft";

fn keychain_get(key: &str) -> Option<String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, key)
        .ok()
        .and_then(|e| e.get_password().ok())
}

fn keychain_set(key: &str, value: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, key) {
        let _ = entry.set_password(value);
    }
}

fn keychain_delete(key: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, key) {
        let _ = entry.delete_credential();
    }
}

// ---------------------------------------------------------------------------
// JWT helper
// ---------------------------------------------------------------------------

fn decode_jwt_claims(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    serde_json::from_slice(&payload).ok()
}

// ---------------------------------------------------------------------------
// Token response shapes (deserialized from stagecraft)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponseAuthenticated {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    user: TokenUser,
    org: TokenOrg,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenResponseOrgSelection {
    pending_id: String,
    orgs: Vec<TokenOrg>,
    user: TokenUser,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenUser {
    id: String,
    email: String,
    name: String,
    #[serde(default)]
    github_login: String,
    #[serde(default)]
    idp_provider: String,
    #[serde(default)]
    idp_login: String,
    avatar_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenOrg {
    org_id: String,
    org_slug: String,
    #[serde(default)]
    github_org_login: String,
    #[serde(default)]
    org_display_name: String,
    platform_role: String,
}

impl From<TokenUser> for AuthUser {
    fn from(u: TokenUser) -> Self {
        Self {
            id: u.id,
            email: u.email,
            name: u.name,
            github_login: u.github_login,
            idp_provider: u.idp_provider,
            idp_login: u.idp_login,
            avatar_url: u.avatar_url,
        }
    }
}

impl From<TokenOrg> for AuthOrg {
    fn from(o: TokenOrg) -> Self {
        Self {
            org_id: o.org_id,
            org_slug: o.org_slug,
            github_org_login: o.github_org_login,
            org_display_name: o.org_display_name,
            platform_role: o.platform_role,
        }
    }
}

// Tagged token response (the server always includes a "type" discriminant)
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TokenResponse {
    Authenticated(TokenResponseAuthenticated),
    OrgSelection(TokenResponseOrgSelection),
}

// org-select / refresh response shapes
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrgSelectResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    user: TokenUser,
    org: TokenOrg,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

// ---------------------------------------------------------------------------
// PKCE helpers
// ---------------------------------------------------------------------------

fn random_base64url(byte_count: usize) -> String {
    // Use uuid::Uuid::new_v4() as a source of OS-level cryptographic randomness.
    // Each UUID provides 16 random bytes (128 bits). We generate enough UUIDs
    // to cover the requested byte count.
    let mut bytes = Vec::with_capacity(byte_count);
    while bytes.len() < byte_count {
        let uuid = uuid::Uuid::new_v4();
        bytes.extend_from_slice(uuid.as_bytes());
    }
    bytes.truncate(byte_count);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest)
}

// ---------------------------------------------------------------------------
// Helper: compute expires_at from expires_in seconds
// ---------------------------------------------------------------------------
fn expires_at_from_in(expires_in: i64) -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    now + expires_in
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Start the desktop OAuth login flow.
///
/// Generates a PKCE challenge, stores the flow in managed state, then opens
/// the user's default browser pointing at the stagecraft authorize endpoint.
#[tauri::command]
#[specta::specta]
pub async fn auth_start_login(
    stagecraft: State<'_, StagecraftState>,
    flow: State<'_, AuthFlowState>,
    app: tauri::AppHandle,
) -> AppResult<()> {
    let client = stagecraft
        .0
        .as_ref()
        .ok_or("Stagecraft client not configured — set STAGECRAFT_BASE_URL")?;

    // Generate PKCE materials
    let code_verifier = random_base64url(32);
    let code_challenge = pkce_challenge(&code_verifier);
    let state_param = random_base64url(32);

    // Persist flow for validation in the callback
    {
        let mut guard = flow
            .0
            .lock()
            .map_err(|e| format!("auth flow lock poisoned: {e}"))?;
        *guard = Some(PkceFlow {
            state: state_param.clone(),
            code_verifier,
            started_at: std::time::Instant::now(),
        });
    }

    let url = format!(
        "{}/auth/desktop/authorize?code_challenge={}&code_challenge_method=S256&state={}&redirect_uri=opc://auth/callback",
        client.base_url(),
        code_challenge,
        state_param
    );

    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| format!("failed to open browser: {e}"))?;

    Ok(())
}

/// Handle the deep-link callback from the OAuth provider.
///
/// Parses the `opc://auth/callback?code=...&state=...` URL, validates the
/// PKCE state, exchanges the code for tokens, and stores them.
#[tauri::command]
#[specta::specta]
pub async fn auth_handle_callback(
    url: String,
    stagecraft: State<'_, StagecraftState>,
    flow: State<'_, AuthFlowState>,
) -> AppResult<AuthResult> {
    // Parse the URL for query params.
    // The URL has the form: opc://auth/callback?code=...&state=...
    // We extract the query string manually to avoid pulling in `url` crate.
    let query = url
        .split_once('?')
        .map(|(_, q)| q)
        .unwrap_or("");

    let mut code_param: Option<String> = None;
    let mut state_param: Option<String> = None;
    let mut error_param: Option<String> = None;

    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            let decoded = v.replace('+', " ");
            // Minimal percent-decoding for common chars
            match k {
                "code" => code_param = Some(decoded),
                "state" => state_param = Some(decoded),
                "error" => error_param = Some(decoded),
                _ => {}
            }
        }
    }

    if let Some(err) = error_param {
        return Ok(AuthResult::Error {
            code: err.clone(),
            message: format!("OAuth provider returned error: {err}"),
        });
    }

    let code = code_param.ok_or("missing 'code' in callback URL")?;
    let state_value = state_param.ok_or("missing 'state' in callback URL")?;

    // Validate and consume the PKCE flow
    let code_verifier = {
        let mut guard = flow
            .0
            .lock()
            .map_err(|e| format!("auth flow lock poisoned: {e}"))?;
        let pkce = guard.take().ok_or("no auth flow in progress")?;
        if pkce.state != state_value {
            return Ok(AuthResult::Error {
                code: "state_mismatch".into(),
                message: "OAuth state parameter mismatch — possible CSRF attack".into(),
            });
        }
        pkce.code_verifier
    };

    let client = stagecraft
        .0
        .as_ref()
        .ok_or("Stagecraft client not configured")?;

    // Exchange code + verifier for tokens
    let body = serde_json::json!({
        "code": code,
        "codeVerifier": code_verifier,
        "redirectUri": "opc://auth/callback"
    });

    let resp = reqwest::Client::new()
        .post(format!("{}/auth/desktop/token", client.base_url()))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("token exchange request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Ok(AuthResult::Error {
            code: format!("http_{status}"),
            message: format!("token exchange failed ({status}): {text}"),
        });
    }

    let token_resp: TokenResponse = resp
        .json()
        .await
        .map_err(|e| format!("failed to decode token response: {e}"))?;

    match token_resp {
        TokenResponse::Authenticated(data) => {
            let expires_at = expires_at_from_in(data.expires_in);
            client.set_auth_token(&data.access_token);
            keychain_set("refresh_token", &data.refresh_token);
            client.set_workspace_id(&data.org.org_id);
            Ok(AuthResult::Authenticated {
                user: data.user.into(),
                org: data.org.into(),
                expires_at,
            })
        }
        TokenResponse::OrgSelection(data) => Ok(AuthResult::OrgSelection {
            pending_id: data.pending_id,
            orgs: data.orgs.into_iter().map(Into::into).collect(),
            user: data.user.into(),
        }),
    }
}

/// Complete multi-org selection after the initial token exchange.
#[tauri::command]
#[specta::specta]
pub async fn auth_select_org(
    pending_id: String,
    org_id: String,
    stagecraft: State<'_, StagecraftState>,
) -> AppResult<AuthResult> {
    let client = stagecraft
        .0
        .as_ref()
        .ok_or("Stagecraft client not configured")?;

    let body = serde_json::json!({
        "pendingId": pending_id,
        "orgId": org_id
    });

    let resp = reqwest::Client::new()
        .post(format!("{}/auth/desktop/org-select", client.base_url()))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("org-select request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Ok(AuthResult::Error {
            code: format!("http_{status}"),
            message: format!("org selection failed ({status}): {text}"),
        });
    }

    let data: OrgSelectResponse = resp
        .json()
        .await
        .map_err(|e| format!("failed to decode org-select response: {e}"))?;

    let expires_at = expires_at_from_in(data.expires_in);
    client.set_auth_token(&data.access_token);
    keychain_set("refresh_token", &data.refresh_token);
    client.set_workspace_id(&data.org.org_id);

    Ok(AuthResult::Authenticated {
        user: data.user.into(),
        org: data.org.into(),
        expires_at,
    })
}

/// Refresh the access token using the stored refresh token.
///
/// Returns the new `expires_at` Unix timestamp.
#[tauri::command]
#[specta::specta]
pub async fn auth_refresh_token(stagecraft: State<'_, StagecraftState>) -> AppResult<i64> {
    let refresh_token =
        keychain_get("refresh_token").ok_or("no refresh token stored in keychain")?;

    let client = stagecraft
        .0
        .as_ref()
        .ok_or("Stagecraft client not configured")?;

    let body = serde_json::json!({ "refreshToken": refresh_token });

    let resp = reqwest::Client::new()
        .post(format!("{}/auth/desktop/refresh", client.base_url()))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("refresh request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("token refresh failed ({status}): {text}"));
    }

    let data: RefreshResponse = resp
        .json()
        .await
        .map_err(|e| format!("failed to decode refresh response: {e}"))?;

    client.set_auth_token(&data.access_token);
    keychain_set("refresh_token", &data.refresh_token);

    // Extract `exp` from new JWT if available; otherwise derive from expires_in
    let exp = decode_jwt_claims(&data.access_token)
        .and_then(|c| c.get("exp")?.as_i64())
        .unwrap_or_else(|| expires_at_from_in(data.expires_in));

    Ok(exp)
}

/// Return the current authentication status by reading the stored JWT.
#[tauri::command]
#[specta::specta]
pub async fn auth_get_status(stagecraft: State<'_, StagecraftState>) -> AppResult<AuthStatus> {
    let token = match keychain_get("session") {
        Some(t) => t,
        None => {
            return Ok(AuthStatus {
                authenticated: false,
                user: None,
                org: None,
                expires_at: None,
            });
        }
    };

    let claims = match decode_jwt_claims(&token) {
        Some(c) => c,
        None => {
            return Ok(AuthStatus {
                authenticated: false,
                user: None,
                org: None,
                expires_at: None,
            });
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let exp = claims.get("exp").and_then(|v| v.as_i64()).unwrap_or(0);

    if exp <= now {
        // Token has expired — check if we can use stagecraft state to determine
        // whether the client still holds one (in-memory may differ from keychain)
        let _ = stagecraft; // accessed to satisfy State borrow rules
        return Ok(AuthStatus {
            authenticated: false,
            user: None,
            org: None,
            expires_at: Some(exp),
        });
    }

    let user = AuthUser {
        id: claims
            .get("oap_user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        email: claims
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        name: claims
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        github_login: claims
            .get("github_login")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        idp_provider: claims
            .get("idp_provider")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        idp_login: claims
            .get("idp_login")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        avatar_url: claims
            .get("avatar_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    };

    let org_slug = claims
        .get("oap_org_slug")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let org = AuthOrg {
        org_id: claims
            .get("oap_org_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        org_slug: org_slug.clone(),
        github_org_login: claims
            .get("github_org_login")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        org_display_name: org_slug,
        platform_role: claims
            .get("platform_role")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    };

    Ok(AuthStatus {
        authenticated: true,
        user: Some(user),
        org: Some(org),
        expires_at: Some(exp),
    })
}

/// Switch to a different org — calls the org-switch API, stores new tokens.
///
/// Returns an AuthResult with the new org context.
#[tauri::command]
#[specta::specta]
pub async fn auth_switch_org(
    org_id: String,
    stagecraft: State<'_, StagecraftState>,
) -> AppResult<AuthResult> {
    let client = stagecraft
        .0
        .as_ref()
        .ok_or("Stagecraft client not configured")?;

    let body = serde_json::json!({ "orgId": org_id });

    let mut req = reqwest::Client::new()
        .post(format!("{}/auth/org-switch", client.base_url()))
        .json(&body);

    // Attach the current auth token
    if let Some(token) = keychain_get("session") {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("org-switch request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Ok(AuthResult::Error {
            code: format!("http_{status}"),
            message: format!("org switch failed ({status}): {text}"),
        });
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct OrgSwitchResponse {
        ok: bool,
        access_token: String,
        expires_in: i64,
    }

    let data: OrgSwitchResponse = resp
        .json()
        .await
        .map_err(|e| format!("failed to decode org-switch response: {e}"))?;

    if !data.ok {
        return Ok(AuthResult::Error {
            code: "switch_failed".into(),
            message: "Org switch was not successful".into(),
        });
    }

    // Store the new token in memory and keychain
    client.set_auth_token(&data.access_token);
    keychain_set("session", &data.access_token);

    // Extract claims from the new JWT to build AuthUser + AuthOrg
    let claims = decode_jwt_claims(&data.access_token);
    let expires_at = claims
        .as_ref()
        .and_then(|c| c.get("exp")?.as_i64())
        .unwrap_or_else(|| expires_at_from_in(data.expires_in));

    let user = claims.as_ref().map(|c| AuthUser {
        id: c.get("oap_user_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        email: c.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        name: c.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        github_login: c.get("github_login").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        idp_provider: c.get("idp_provider").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        idp_login: c.get("idp_login").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        avatar_url: c.get("avatar_url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    });

    let org = claims.as_ref().map(|c| {
        let slug = c.get("oap_org_slug").and_then(|v| v.as_str()).unwrap_or("").to_string();
        AuthOrg {
            org_id: c.get("oap_org_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            org_slug: slug.clone(),
            github_org_login: c.get("github_org_login").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            org_display_name: slug,
            platform_role: c.get("platform_role").and_then(|v| v.as_str()).unwrap_or("member").to_string(),
        }
    });

    match (user, org) {
        (Some(u), Some(o)) => Ok(AuthResult::Authenticated {
            user: u,
            org: o,
            expires_at,
        }),
        _ => Ok(AuthResult::Error {
            code: "decode_failed".into(),
            message: "Could not decode org context from new token".into(),
        }),
    }
}

/// Clear all auth state — keychain entries and in-memory token.
#[tauri::command]
#[specta::specta]
pub async fn auth_logout(stagecraft: State<'_, StagecraftState>) -> AppResult<()> {
    keychain_delete("session");
    keychain_delete("refresh_token");
    if let Some(client) = stagecraft.0.as_ref() {
        client.clear_auth();
    }
    Ok(())
}
