//! HTTP client for Stagecraft Platform API (Specs 077, 087).
//!
//! Provides dual-write capability: the local Factory engine executes pipelines
//! while this client mirrors lifecycle events to the Stagecraft control plane
//! for centralized audit, token tracking, and governance.
//!
//! Spec 087 Phase 5: auth_token is a Rauthy JWT (stored in OS keychain).
//! All authenticated requests use `Authorization: Bearer <jwt>`.
//!
//! All methods are best-effort — callers log warnings on failure but never
//! block local pipeline execution.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::time::Duration;

/// Tauri-managed wrapper for the optional Stagecraft HTTP client.
///
/// Wrapped in a `RwLock` so the base URL can be swapped at runtime via the
/// settings UI (see `commands::settings::set_stagecraft_base_url`). When the
/// URL is unset, the inner `Option` is `None` and factory commands run
/// local-only.
pub struct StagecraftState(pub RwLock<Option<StagecraftClient>>);

impl StagecraftState {
    /// Return a clone of the current client, if any.
    pub fn current(&self) -> Option<StagecraftClient> {
        self.0.read().ok().and_then(|g| g.clone())
    }

    /// Replace the current client (used when the base URL changes).
    pub fn replace(&self, client: Option<StagecraftClient>) {
        if let Ok(mut g) = self.0.write() {
            *g = client;
        }
    }
}

// ---------------------------------------------------------------------------
// Stage ID mapping — local engine uses longer names than Stagecraft
// ---------------------------------------------------------------------------

/// Map local pipeline stage IDs to Stagecraft's canonical stage IDs.
pub fn to_stagecraft_stage_id(local_id: &str) -> &str {
    match local_id {
        "s4-api-specification" => "s4-api-spec",
        "s5-ui-specification" => "s5-ui-spec",
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Thin HTTP wrapper around the Stagecraft Platform API.
///
/// Held as Tauri managed state so all commands share one connection pool.
/// Org-aware: carries the active org ID and a Rauthy JWT for authenticated
/// platform endpoints (spec 087 Phase 5, renamed by spec 119). The JWT is stored in the OS
/// keychain via the `keychain` module.
pub struct StagecraftClient {
    client: Client,
    base_url: String,
    /// Default actor identity sent on mutating requests.
    actor_user_id: RwLock<String>,
    /// Active org ID (set at runtime after auth).
    org_id: RwLock<String>,
    /// Rauthy JWT for authenticated endpoints (loaded from OS keychain).
    auth_token: RwLock<Option<String>>,
}

impl Clone for StagecraftClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            actor_user_id: RwLock::new(self.actor_user_id.read().unwrap().clone()),
            org_id: RwLock::new(self.org_id.read().unwrap().clone()),
            auth_token: RwLock::new(self.auth_token.read().unwrap().clone()),
        }
    }
}

impl StagecraftClient {
    /// Build a new client.  Returns `None` when `base_url` is empty (integration disabled).
    pub fn new(base_url: &str, actor_user_id: &str) -> Option<Self> {
        if base_url.is_empty() {
            return None;
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .ok()?;
        Some(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            actor_user_id: RwLock::new(actor_user_id.to_string()),
            org_id: RwLock::new(String::new()),
            auth_token: RwLock::new(None),
        })
    }

    /// Set the active org ID (called after auth).
    pub fn set_org_id(&self, id: &str) {
        *self.org_id.write().unwrap() = id.to_string();
    }

    /// Get the active org ID.
    pub fn org_id(&self) -> String {
        self.org_id.read().unwrap().clone()
    }

    /// Set the auth token (Rauthy JWT) and persist to OS keychain.
    pub fn set_auth_token(&self, token: &str) {
        *self.auth_token.write().unwrap() = Some(token.to_string());
        // Best-effort persist to keychain
        if let Ok(entry) = keyring::Entry::new("dev.opc.stagecraft", "session") {
            let _ = entry.set_password(token);
        }
    }

    /// Load the auth token from the OS keychain (called on startup).
    pub fn load_token_from_keychain(&self) -> bool {
        if let Ok(entry) = keyring::Entry::new("dev.opc.stagecraft", "session")
            && let Ok(token) = entry.get_password()
        {
            *self.auth_token.write().unwrap() = Some(token);
            return true;
        }
        false
    }

    /// Update the actor user identity at runtime (e.g. after desktop OAuth).
    pub fn set_actor_user_id(&self, id: &str) {
        *self.actor_user_id.write().unwrap() = id.to_string();
    }

    /// Return the base URL (without trailing slash).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Return the current auth token, if one is loaded. Primarily used by
    /// the duplex sync consumer (spec 110 Phase 2) which needs to attach
    /// it as a `Authorization: Bearer …` header on the WebSocket handshake.
    pub fn auth_token(&self) -> Option<String> {
        self.auth_token.read().ok().and_then(|g| g.clone())
    }

    /// Clear all auth state (tokens, workspace, actor identity).
    pub fn clear_auth(&self) {
        *self.auth_token.write().unwrap() = None;
        *self.org_id.write().unwrap() = String::new();
        if let Ok(entry) = keyring::Entry::new("dev.opc.stagecraft", "session") {
            let _ = entry.delete_credential();
        }
        if let Ok(entry) = keyring::Entry::new("dev.opc.stagecraft", "refresh_token") {
            let _ = entry.delete_credential();
        }
    }

    /// Build a request with Bearer auth header.
    fn authed_get(&self, url: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.get(url);
        if let Some(token) = self.auth_token.read().unwrap().as_ref() {
            req = req.bearer_auth(token);
        }
        req
    }

    fn authed_post(&self, url: &str) -> reqwest::RequestBuilder {
        let mut req = self.client.post(url);
        if let Some(token) = self.auth_token.read().unwrap().as_ref() {
            req = req.bearer_auth(token);
        }
        req
    }

    /// Spec 112 §6.3 — silent JWT refresh.
    ///
    /// Reads the persisted Rauthy refresh token from the OS keychain, exchanges
    /// it at `/auth/desktop/refresh`, and rotates the in-memory + keychain
    /// access/refresh pair. Returns Ok only when the new bearer is live;
    /// callers retry the failing request once on success and surface the
    /// original 401 on failure.
    async fn refresh_jwt(&self) -> Result<(), StagecraftError> {
        let refresh_token = keyring::Entry::new("dev.opc.stagecraft", "refresh_token")
            .ok()
            .and_then(|e| e.get_password().ok())
            .ok_or_else(|| {
                StagecraftError::Api(401, "no refresh_token in keychain".into())
            })?;

        let url = format!("{}/auth/desktop/refresh", self.base_url);
        let body = serde_json::json!({ "refreshToken": refresh_token });
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        let data: RefreshTokenResponse =
            resp.json().await.map_err(StagecraftError::Decode)?;
        self.set_auth_token(&data.access_token);
        if let Ok(entry) = keyring::Entry::new("dev.opc.stagecraft", "refresh_token") {
            let _ = entry.set_password(&data.refresh_token);
        }
        Ok(())
    }

    // -- Workspace CRUD (spec 087) -------------------------------------------

    /// List workspaces for the authenticated user's org.
    pub async fn list_workspaces(&self) -> Result<ListWorkspacesResponse, StagecraftError> {
        let url = format!("{}/api/workspaces", self.base_url);
        let resp = self
            .authed_get(&url)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    /// Get a single workspace by ID (legacy endpoint, kept for API compat).
    pub async fn get_workspace(
        &self,
        org_id: &str,
    ) -> Result<GetWorkspaceResponse, StagecraftError> {
        let url = format!("{}/api/workspaces/{}", self.base_url, org_id);
        let resp = self
            .authed_get(&url)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    /// Get or create the default workspace for the current org.
    pub async fn get_default_workspace(&self) -> Result<GetWorkspaceResponse, StagecraftError> {
        let url = format!("{}/api/workspaces/by-org/default", self.base_url);
        let resp = self
            .authed_get(&url)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- FR-001: Init Pipeline ------------------------------------------------

    pub async fn init_pipeline(
        &self,
        project_id: &str,
        adapter: &str,
        business_docs: &[BusinessDocRef],
    ) -> Result<InitResponse, StagecraftError> {
        let url = format!("{}/api/projects/{}/factory/init", self.base_url, project_id);
        let body = InitRequest {
            adapter: adapter.into(),
            business_docs: if business_docs.is_empty() {
                None
            } else {
                Some(business_docs.to_vec())
            },
            policy_overrides: None,
            actor_user_id: self.actor_user_id.read().unwrap().clone(),
            org_id: self.org_id(),
            source: "opc-direct",
        };
        let resp = self
            .authed_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- FR-004: Confirm Stage ------------------------------------------------

    pub async fn confirm_stage(
        &self,
        project_id: &str,
        stage_id: &str,
        notes: Option<&str>,
    ) -> Result<ConfirmResponse, StagecraftError> {
        let sc_stage = to_stagecraft_stage_id(stage_id);
        let url = format!(
            "{}/api/projects/{}/factory/stage/{}/confirm",
            self.base_url, project_id, sc_stage
        );
        let body = ConfirmRequest {
            notes: notes.map(String::from),
            actor_user_id: self.actor_user_id.read().unwrap().clone(),
            org_id: self.org_id(),
        };
        let resp = self
            .authed_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- FR-005: Reject Stage -------------------------------------------------

    pub async fn reject_stage(
        &self,
        project_id: &str,
        stage_id: &str,
        feedback: &str,
    ) -> Result<RejectResponse, StagecraftError> {
        let sc_stage = to_stagecraft_stage_id(stage_id);
        let url = format!(
            "{}/api/projects/{}/factory/stage/{}/reject",
            self.base_url, project_id, sc_stage
        );
        let body = RejectRequest {
            feedback: feedback.into(),
            actor_user_id: self.actor_user_id.read().unwrap().clone(),
            org_id: self.org_id(),
        };
        let resp = self
            .authed_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- FR-009: Pipeline Status Update -----------------------------------------

    pub async fn update_pipeline_status(
        &self,
        project_id: &str,
        pipeline_id: &str,
        status: &str,
        current_stage: Option<&str>,
        error: Option<&str>,
        phase: Option<&str>,
    ) -> Result<StatusUpdateResponse, StagecraftError> {
        let url = format!(
            "{}/api/projects/{}/factory/status-update",
            self.base_url, project_id
        );
        let body = StatusUpdateRequest {
            pipeline_id: pipeline_id.into(),
            status: status.into(),
            current_stage: current_stage.map(String::from),
            error: error.map(String::from),
            phase: phase.map(String::from),
            actor_user_id: self.actor_user_id.read().unwrap().clone(),
            org_id: self.org_id(),
        };
        let resp = self
            .authed_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- FR-010: Scaffold Progress ---------------------------------------------

    pub async fn report_scaffold_progress(
        &self,
        project_id: &str,
        pipeline_id: &str,
        features: &[ScaffoldFeatureReport],
    ) -> Result<ScaffoldProgressResponse, StagecraftError> {
        let url = format!(
            "{}/api/projects/{}/factory/scaffold-progress",
            self.base_url, project_id
        );
        let body = ScaffoldProgressRequest {
            pipeline_id: pipeline_id.into(),
            features: features.to_vec(),
            actor_user_id: self.actor_user_id.read().unwrap().clone(),
            org_id: self.org_id(),
        };
        let resp = self
            .authed_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- FR-012: Cancel Pipeline -------------------------------------------------

    pub async fn cancel_pipeline(
        &self,
        project_id: &str,
        reason: &str,
    ) -> Result<CancelResponse, StagecraftError> {
        let url = format!(
            "{}/api/projects/{}/factory/cancel",
            self.base_url, project_id
        );
        let body = CancelRequest {
            reason: reason.into(),
            actor_user_id: self.actor_user_id.read().unwrap().clone(),
            org_id: self.org_id(),
        };
        let resp = self
            .authed_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- FR-011: Batch Event Ingestion ------------------------------------------

    pub async fn ingest_events(
        &self,
        project_id: &str,
        pipeline_id: &str,
        events: &[OrchestratorEventReport],
    ) -> Result<EventIngestionResponse, StagecraftError> {
        let url = format!(
            "{}/api/projects/{}/factory/events",
            self.base_url, project_id
        );
        let body = EventIngestionRequest {
            pipeline_id: pipeline_id.into(),
            events: events.to_vec(),
            org_id: self.org_id(),
        };
        let resp = self
            .authed_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- 082 FR-023: Record Artifacts ------------------------------------------

    pub async fn record_artifacts(
        &self,
        project_id: &str,
        pipeline_id: &str,
        stage_id: &str,
        artifacts: &[ArtifactRecord],
    ) -> Result<RecordArtifactsResponse, StagecraftError> {
        let url = format!(
            "{}/api/projects/{}/factory/artifacts",
            self.base_url, project_id
        );
        let body = RecordArtifactsRequest {
            pipeline_id: pipeline_id.into(),
            stage_id: stage_id.into(),
            artifacts: artifacts.to_vec(),
            org_id: self.org_id(),
        };
        let resp = self
            .authed_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- 082 FR-025: Lookup Artifacts ------------------------------------------

    pub async fn lookup_artifact(
        &self,
        project_id: &str,
        content_hash: &str,
        stage_id: &str,
    ) -> Result<LookupArtifactResponse, StagecraftError> {
        let url = format!(
            "{}/api/projects/{}/factory/artifacts/lookup?content_hash={}&stage_id={}",
            self.base_url,
            project_id,
            content_hash,
            stage_id,
        );
        let resp = self
            .authed_get(&url)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- Spec 111 Phase 6: one-click local→remote agent publishing -----------

    /// Create a draft in the org's agent catalog.
    ///
    /// The server scopes the draft to the orgId embedded in the Rauthy
    /// JWT (see auth middleware in stagecraft's catalog.ts), so the
    /// desktop only needs a valid Bearer token plus the payload. Returns the
    /// new catalog row identifiers so the caller can link the user to the
    /// web-UI publish page.
    pub async fn create_agent_draft(
        &self,
        name: &str,
        frontmatter: serde_json::Value,
        body_markdown: &str,
    ) -> Result<CreateAgentDraftResponse, StagecraftError> {
        let url = format!("{}/api/agents", self.base_url);
        let body = CreateAgentDraftRequest {
            name: name.to_string(),
            frontmatter,
            body_markdown: body_markdown.to_string(),
        };
        let resp = self
            .authed_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    // -- FR-008: Token Spend --------------------------------------------------

    pub async fn report_token_spend(
        &self,
        project_id: &str,
        run_id: &str,
        stage_id: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
        model: &str,
    ) -> Result<(), StagecraftError> {
        let sc_stage = to_stagecraft_stage_id(stage_id);
        let url = format!(
            "{}/api/projects/{}/factory/token-spend",
            self.base_url, project_id
        );
        let body = TokenSpendRequest {
            run_id: run_id.into(),
            stage_id: sc_stage.into(),
            prompt_tokens,
            completion_tokens,
            model: model.into(),
            org_id: self.org_id(),
        };
        let resp = self
            .authed_post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(
                resp.status().as_u16(),
                resp.text().await.unwrap_or_default(),
            ));
        }
        Ok(())
    }

    /// Spec 112 §6.3 — fetch the Open-in-OPC bundle for a project.
    ///
    /// Mirrors `GET /api/projects/:projectId/opc-bundle`. The bundle carries
    /// everything OPC needs after activating an `opc://` deep link: the
    /// project, its repo, its adapter, the org's contracts and processes,
    /// and the workspace's published agent catalog. Workspace scoping is
    /// enforced server-side via the Bearer token.
    ///
    /// Auto-retries once on 401 by silently refreshing the Rauthy session
    /// (refresh-token in the OS keychain) so an expired access token does
    /// not surface to the user as a permanent inbox-banner failure.
    pub async fn get_project_opc_bundle(
        &self,
        project_id: &str,
    ) -> Result<OpcBundleResponse, StagecraftError> {
        let url = format!(
            "{}/api/projects/{}/opc-bundle",
            self.base_url, project_id
        );
        self.authed_json_get_with_refresh(&url).await
    }

    /// Spec 112 §6.4.2 — refresh just the clone token.
    ///
    /// Cheaper than re-fetching the full bundle when an installation
    /// token is within the 5-minute refresh window or when a 401
    /// surfaces from a GitHub call. The response shape is a
    /// `{ cloneToken: OpcBundleCloneToken | null }` envelope.
    ///
    /// Same 401 → silent-refresh-and-retry as `get_project_opc_bundle`.
    pub async fn refresh_project_clone_token(
        &self,
        project_id: &str,
    ) -> Result<CloneTokenResponse, StagecraftError> {
        let url = format!(
            "{}/api/projects/{}/clone-token",
            self.base_url, project_id
        );
        self.authed_json_get_with_refresh(&url).await
    }

    /// Internal: GET → JSON with one transparent retry on 401.
    ///
    /// On 401 the client attempts a Rauthy refresh-token exchange against
    /// `/auth/desktop/refresh`. If the refresh succeeds, the original
    /// request is replayed once with the new bearer; otherwise the original
    /// 401 surfaces to the caller. Both retries log just the status to
    /// avoid leaking response bodies to the structured logger.
    async fn authed_json_get_with_refresh<T>(
        &self,
        url: &str,
    ) -> Result<T, StagecraftError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let resp = self
            .authed_get(url)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if resp.status().as_u16() != 401 {
            if !resp.status().is_success() {
                return Err(StagecraftError::Api(
                    resp.status().as_u16(),
                    resp.text().await.unwrap_or_default(),
                ));
            }
            return resp.json().await.map_err(StagecraftError::Decode);
        }

        // 401: try to refresh once, then retry. Carry forward the
        // original body so a refresh-failure still reports the meaningful
        // error to the inbox banner.
        let original_body = resp.text().await.unwrap_or_default();
        if let Err(refresh_err) = self.refresh_jwt().await {
            return Err(StagecraftError::Api(
                401,
                format!(
                    "{} (refresh failed: {})",
                    if original_body.is_empty() {
                        "unauthenticated".to_string()
                    } else {
                        original_body
                    },
                    refresh_err
                ),
            ));
        }
        let retry = self
            .authed_get(url)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !retry.status().is_success() {
            return Err(StagecraftError::Api(
                retry.status().as_u16(),
                retry.text().await.unwrap_or_default(),
            ));
        }
        retry.json().await.map_err(StagecraftError::Decode)
    }
}

// ---------------------------------------------------------------------------
// Workspace types (spec 087)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub id: String,
    #[serde(rename = "orgId")]
    pub org_id: String,
    pub name: String,
    pub slug: String,
    #[serde(rename = "objectStoreBucket")]
    pub object_store_bucket: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListWorkspacesResponse {
    pub workspaces: Vec<WorkspaceInfo>,
}

#[derive(Debug, Deserialize)]
pub struct GetWorkspaceResponse {
    pub workspace: WorkspaceInfo,
}

// ---------------------------------------------------------------------------
// Request / Response types (mirror Stagecraft's TypeScript shapes)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct InitRequest {
    adapter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    business_docs: Option<Vec<BusinessDocRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy_overrides: Option<PolicyOverrides>,
    #[serde(rename = "actorUserId")]
    actor_user_id: String,
    #[serde(rename = "orgId")]
    org_id: String,
    // Spec 110 §8 Phase 6: server default is now "stagecraft". The desktop's
    // dual-write path is already running the engine locally, so it must pin
    // `source` to "opc-direct" to prevent stagecraft from dispatching a
    // `factory.run.request` envelope back to the same (or another) OPC.
    source: &'static str,
}

#[derive(Clone, Serialize)]
pub struct BusinessDocRef {
    pub name: String,
    pub storage_ref: String,
}

#[derive(Serialize)]
pub struct PolicyOverrides {
    pub max_retry_per_feature: Option<u32>,
    pub token_budget_total: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct InitResponse {
    pub pipeline_id: String,
    pub adapter: String,
    pub policy_bundle_id: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Serialize)]
struct ConfirmRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    #[serde(rename = "actorUserId")]
    actor_user_id: String,
    #[serde(rename = "orgId")]
    org_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmResponse {
    pub stage: String,
    pub confirmed_by: String,
    pub confirmed_at: String,
    pub audit_entry_id: String,
}

#[derive(Serialize)]
struct RejectRequest {
    feedback: String,
    #[serde(rename = "actorUserId")]
    actor_user_id: String,
    #[serde(rename = "orgId")]
    org_id: String,
}

#[derive(Debug, Deserialize)]
pub struct RejectResponse {
    pub stage: String,
    pub rejected_by: String,
    pub rejected_at: String,
    pub feedback: String,
    pub audit_entry_id: String,
}

#[derive(Serialize)]
struct StatusUpdateRequest {
    pipeline_id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phase: Option<String>,
    #[serde(rename = "actorUserId")]
    actor_user_id: String,
    #[serde(rename = "orgId")]
    org_id: String,
}

#[derive(Debug, Deserialize)]
pub struct StatusUpdateResponse {
    pub pipeline_id: String,
    pub status: String,
    pub audit_entry_id: String,
}

#[derive(Clone, Serialize)]
pub struct ScaffoldFeatureReport {
    pub feature_id: String,
    pub category: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_created: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u64>,
}

#[derive(Serialize)]
struct ScaffoldProgressRequest {
    pipeline_id: String,
    features: Vec<ScaffoldFeatureReport>,
    #[serde(rename = "actorUserId")]
    actor_user_id: String,
    #[serde(rename = "orgId")]
    org_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ScaffoldProgressResponse {
    pub upserted: u32,
    pub audit_entry_id: String,
}

#[derive(Serialize)]
struct CancelRequest {
    reason: String,
    #[serde(rename = "actorUserId")]
    actor_user_id: String,
    #[serde(rename = "orgId")]
    org_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CancelResponse {
    pub pipeline_id: String,
    pub cancelled_at: String,
    pub audit_entry_id: String,
}

#[derive(Clone, Serialize)]
pub struct OrchestratorEventReport {
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct EventIngestionRequest {
    pipeline_id: String,
    events: Vec<OrchestratorEventReport>,
    #[serde(rename = "orgId")]
    org_id: String,
}

#[derive(Debug, Deserialize)]
pub struct EventIngestionResponse {
    pub ingested: u32,
}

#[derive(Serialize)]
struct TokenSpendRequest {
    run_id: String,
    stage_id: String,
    prompt_tokens: u64,
    completion_tokens: u64,
    model: String,
    #[serde(rename = "orgId")]
    org_id: String,
}

// ---------------------------------------------------------------------------
// Artifact types (082 Phase 3)
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize)]
pub struct ArtifactRecord {
    pub artifact_type: String,
    pub content_hash: String,
    pub storage_path: String,
    pub size_bytes: u64,
}

#[derive(Serialize)]
struct RecordArtifactsRequest {
    pipeline_id: String,
    stage_id: String,
    artifacts: Vec<ArtifactRecord>,
    #[serde(rename = "orgId")]
    org_id: String,
}

#[derive(Debug, Deserialize)]
pub struct RecordArtifactsResponse {
    pub recorded: u32,
}

#[derive(Debug, Deserialize)]
pub struct LookupArtifactResponse {
    pub found: bool,
    pub artifact: Option<ArtifactInfo>,
}

// ---------------------------------------------------------------------------
// Agent catalog draft (spec 111 Phase 6)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct CreateAgentDraftRequest {
    name: String,
    frontmatter: serde_json::Value,
    body_markdown: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAgentDraftResponse {
    pub agent: CatalogAgentWire,
}

/// Subset of stagecraft's `CatalogAgent` that the desktop cares about after a
/// draft create. Fields mirror the snake_cased wire shape defined in
/// `catalog.ts`.
#[derive(Debug, Deserialize)]
pub struct CatalogAgentWire {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub version: u32,
    pub status: String,
    pub content_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct ArtifactInfo {
    pub pipeline_id: String,
    pub stage_id: String,
    pub artifact_type: String,
    pub content_hash: String,
    pub storage_path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Spec 112 §6.3 — OPC bundle types (mirrors stagecraft's OpcBundleResponse)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpcBundleProject {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub org_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpcBundleRepo {
    pub clone_url: String,
    pub github_org: String,
    pub repo_name: String,
    pub default_branch: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpcBundleAdapter {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source_sha: String,
    pub synced_at: String,
    pub manifest: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpcBundleContract {
    pub name: String,
    pub version: String,
    pub source_sha: String,
    pub synced_at: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpcBundleProcess {
    pub name: String,
    pub version: String,
    pub source_sha: String,
    pub synced_at: String,
    pub definition: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpcBundleAgent {
    pub id: String,
    pub name: String,
    pub version: i64,
    pub status: String,
    pub content_hash: String,
    pub frontmatter: serde_json::Value,
    pub body_markdown: String,
}

/// Spec 112 §6.4 — short-lived clone token derived from spec 109 state.
/// `expires_at` is set for `github_installation` (~1h TTL) and null for
/// `project_github_pat`. The bundle returns `clone_token: None` for
/// public repos (anonymous clone path); a hard-resolution failure on
/// the stagecraft side surfaces as a 503 instead.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpcBundleCloneToken {
    pub value: String,
    pub source: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpcBundleResponse {
    pub project: OpcBundleProject,
    pub repo: Option<OpcBundleRepo>,
    pub deep_link: Option<String>,
    pub adapter: Option<OpcBundleAdapter>,
    pub contracts: Vec<OpcBundleContract>,
    pub processes: Vec<OpcBundleProcess>,
    pub agents: Vec<OpcBundleAgent>,
    pub clone_token: Option<OpcBundleCloneToken>,
}

/// Spec 112 §6.4.2 — refresh-endpoint response shape.
/// Lightweight sibling of the bundle: just the token field.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CloneTokenResponse {
    pub clone_token: Option<OpcBundleCloneToken>,
}

/// Internal-only: shape of `POST /auth/desktop/refresh`. Mirrors the
/// `RefreshResponse` in `commands/auth.rs` but lives here so the client
/// can drive a silent refresh on 401 without re-entering the auth
/// command surface.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshTokenResponse {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    #[allow(dead_code)]
    expires_in: i64,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum StagecraftError {
    /// Transport-level failure (DNS, timeout, connection refused).
    Network(reqwest::Error),
    /// Stagecraft returned a non-2xx status.
    Api(u16, String),
    /// Response body could not be decoded.
    Decode(reqwest::Error),
}

impl std::fmt::Display for StagecraftError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "stagecraft network error: {e}"),
            Self::Api(status, body) => {
                write!(f, "stagecraft API error {status}: {body}")
            }
            Self::Decode(e) => write!(f, "stagecraft decode error: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec 112 §6.3 — pin the wire format. The stagecraft endpoint
    /// returns camelCase fields; the desktop's serde rename_all must
    /// keep up. If this test starts failing, the TS side likely renamed
    /// a field on `OpcBundleResponse` and the desktop is decoding the
    /// wrong shape.
    #[test]
    fn oap_bundle_response_decodes_camelcase_payload() {
        let payload = r##"{
            "project": {
                "id": "p-1",
                "name": "FV Portal",
                "slug": "fv-portal",
                "orgId": "org-1"
            },
            "repo": {
                "cloneUrl": "https://github.com/acme/fv.git",
                "githubOrg": "acme",
                "repoName": "fv",
                "defaultBranch": "main"
            },
            "deepLink": "opc://project/open?project_id=p-1&url=https%3A%2F%2Fgithub.com%2Facme%2Ffv.git",
            "adapter": {
                "id": "a-1",
                "name": "aim-vue-node",
                "version": "3.0.0",
                "sourceSha": "abc",
                "syncedAt": "2026-04-22T10:00:00.000Z",
                "manifest": {"k":"v"}
            },
            "contracts": [{
                "name": "build-spec",
                "version": "1.0.0",
                "sourceSha": "c1",
                "syncedAt": "2026-04-22T10:00:00.000Z",
                "schema": {}
            }],
            "processes": [],
            "agents": [{
                "id": "ag-1",
                "name": "explorer",
                "version": 2,
                "status": "published",
                "contentHash": "h1",
                "frontmatter": {},
                "bodyMarkdown": "# explorer"
            }],
            "cloneToken": {
                "value": "ghs_FAKE_INSTALL_TOKEN",
                "source": "github_installation",
                "expiresAt": "2026-04-22T11:00:00.000Z"
            }
        }"##;

        let bundle: OpcBundleResponse =
            serde_json::from_str(payload).expect("valid bundle decodes");

        assert_eq!(bundle.project.id, "p-1");
        assert_eq!(bundle.project.org_id, "org-1");
        assert_eq!(bundle.repo.as_ref().unwrap().clone_url, "https://github.com/acme/fv.git");
        assert!(bundle.deep_link.is_some());
        assert_eq!(bundle.adapter.as_ref().unwrap().name, "aim-vue-node");
        assert_eq!(bundle.contracts.len(), 1);
        assert_eq!(bundle.processes.len(), 0);
        assert_eq!(bundle.agents.len(), 1);
        assert_eq!(bundle.agents[0].status, "published");
        let token = bundle.clone_token.as_ref().expect("clone token present");
        assert_eq!(token.source, "github_installation");
        assert_eq!(token.value, "ghs_FAKE_INSTALL_TOKEN");
        assert_eq!(token.expires_at.as_deref(), Some("2026-04-22T11:00:00.000Z"));
    }

    #[test]
    fn oap_bundle_response_handles_null_repo_and_adapter() {
        let payload = r##"{
            "project": {
                "id": "p-1",
                "name": "Legacy",
                "slug": "legacy",
                "orgId": "org-1"
            },
            "repo": null,
            "deepLink": null,
            "adapter": null,
            "contracts": [],
            "processes": [],
            "agents": [],
            "cloneToken": null
        }"##;

        let bundle: OpcBundleResponse =
            serde_json::from_str(payload).expect("nulls decode");
        assert!(bundle.repo.is_none());
        assert!(bundle.deep_link.is_none());
        assert!(bundle.adapter.is_none());
        assert!(bundle.clone_token.is_none());
    }

    #[test]
    fn oap_bundle_response_decodes_pat_clone_token_with_null_expiry() {
        let payload = r##"{
            "project": {
                "id": "p-1",
                "name": "External",
                "slug": "external",
                "orgId": "org-1"
            },
            "repo": {
                "cloneUrl": "https://github.com/external-org/foo.git",
                "githubOrg": "external-org",
                "repoName": "foo",
                "defaultBranch": "main"
            },
            "deepLink": "opc://project/open?project_id=p-1&url=https%3A%2F%2Fgithub.com%2Fexternal-org%2Ffoo.git",
            "adapter": null,
            "contracts": [],
            "processes": [],
            "agents": [],
            "cloneToken": {
                "value": "ghp_FAKE_PAT",
                "source": "project_github_pat",
                "expiresAt": null
            }
        }"##;

        let bundle: OpcBundleResponse =
            serde_json::from_str(payload).expect("PAT bundle decodes");
        let token = bundle.clone_token.as_ref().expect("clone token present");
        assert_eq!(token.source, "project_github_pat");
        assert!(token.expires_at.is_none());
    }

    #[test]
    fn clone_token_refresh_envelope_decodes() {
        let payload = r##"{
            "cloneToken": {
                "value": "ghs_REFRESHED_TOKEN",
                "source": "github_installation",
                "expiresAt": "2026-04-22T12:00:00.000Z"
            }
        }"##;

        let resp: CloneTokenResponse =
            serde_json::from_str(payload).expect("refresh envelope decodes");
        let token = resp.clone_token.expect("token present");
        assert_eq!(token.value, "ghs_REFRESHED_TOKEN");
    }
}
