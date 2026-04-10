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
/// When `STAGECRAFT_BASE_URL` is set, `0` contains `Some(client)`;
/// otherwise `None` and factory commands run local-only.
pub struct StagecraftState(pub Option<StagecraftClient>);

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
/// Workspace-aware: carries workspace_id and a Rauthy JWT for authenticated
/// workspace endpoints (spec 087 Phase 5). The JWT is stored in the OS
/// keychain via the `keychain` module.
pub struct StagecraftClient {
    client: Client,
    base_url: String,
    /// Default actor identity sent on mutating requests.
    actor_user_id: String,
    /// Active workspace ID (set at runtime after auth).
    workspace_id: RwLock<String>,
    /// Rauthy JWT for authenticated endpoints (loaded from OS keychain).
    auth_token: RwLock<Option<String>>,
}

impl Clone for StagecraftClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            actor_user_id: self.actor_user_id.clone(),
            workspace_id: RwLock::new(
                self.workspace_id.read().unwrap().clone(),
            ),
            auth_token: RwLock::new(
                self.auth_token.read().unwrap().clone(),
            ),
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
            actor_user_id: actor_user_id.to_string(),
            workspace_id: RwLock::new(String::new()),
            auth_token: RwLock::new(None),
        })
    }

    /// Set the active workspace ID (called after workspace selection).
    pub fn set_workspace_id(&self, id: &str) {
        *self.workspace_id.write().unwrap() = id.to_string();
    }

    /// Get the active workspace ID.
    pub fn workspace_id(&self) -> String {
        self.workspace_id.read().unwrap().clone()
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
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
        }
        resp.json().await.map_err(StagecraftError::Decode)
    }

    /// Get a single workspace by ID.
    pub async fn get_workspace(&self, workspace_id: &str) -> Result<GetWorkspaceResponse, StagecraftError> {
        let url = format!("{}/api/workspaces/{}", self.base_url, workspace_id);
        let resp = self
            .authed_get(&url)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
        let url = format!(
            "{}/api/projects/{}/factory/init",
            self.base_url, project_id
        );
        let body = InitRequest {
            adapter: adapter.into(),
            business_docs: if business_docs.is_empty() {
                None
            } else {
                Some(business_docs.to_vec())
            },
            policy_overrides: None,
            actor_user_id: self.actor_user_id.clone(),
            workspace_id: self.workspace_id(),
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
        let ws_id = self.workspace_id();
        let body = ConfirmRequest {
            notes: notes.map(String::from),
            actor_user_id: self.actor_user_id.clone(),
            workspace_id: if ws_id.is_empty() { None } else { Some(ws_id) },
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
        let ws_id = self.workspace_id();
        let body = RejectRequest {
            feedback: feedback.into(),
            actor_user_id: self.actor_user_id.clone(),
            workspace_id: if ws_id.is_empty() { None } else { Some(ws_id) },
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
        let ws_id = self.workspace_id();
        let body = StatusUpdateRequest {
            pipeline_id: pipeline_id.into(),
            status: status.into(),
            current_stage: current_stage.map(String::from),
            error: error.map(String::from),
            phase: phase.map(String::from),
            actor_user_id: self.actor_user_id.clone(),
            workspace_id: if ws_id.is_empty() { None } else { Some(ws_id) },
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
        let ws_id = self.workspace_id();
        let body = ScaffoldProgressRequest {
            pipeline_id: pipeline_id.into(),
            features: features.to_vec(),
            actor_user_id: self.actor_user_id.clone(),
            workspace_id: if ws_id.is_empty() { None } else { Some(ws_id) },
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
        let ws_id = self.workspace_id();
        let body = CancelRequest {
            reason: reason.into(),
            actor_user_id: self.actor_user_id.clone(),
            workspace_id: if ws_id.is_empty() { None } else { Some(ws_id) },
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
        let ws_id = self.workspace_id();
        let body = EventIngestionRequest {
            pipeline_id: pipeline_id.into(),
            events: events.to_vec(),
            workspace_id: if ws_id.is_empty() { None } else { Some(ws_id) },
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
        let ws_id = self.workspace_id();
        let body = RecordArtifactsRequest {
            pipeline_id: pipeline_id.into(),
            stage_id: stage_id.into(),
            artifacts: artifacts.to_vec(),
            workspace_id: if ws_id.is_empty() { None } else { Some(ws_id) },
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
        let ws_id = self.workspace_id();
        let ws_param = if ws_id.is_empty() {
            String::new()
        } else {
            format!("&workspaceId={}", ws_id)
        };
        let url = format!(
            "{}/api/projects/{}/factory/artifacts/lookup?content_hash={}&stage_id={}{}",
            self.base_url, project_id, content_hash, stage_id, ws_param
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
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
        let ws_id = self.workspace_id();
        let body = TokenSpendRequest {
            run_id: run_id.into(),
            stage_id: sc_stage.into(),
            prompt_tokens,
            completion_tokens,
            model: model.into(),
            workspace_id: if ws_id.is_empty() { None } else { Some(ws_id) },
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(StagecraftError::Network)?;
        if !resp.status().is_success() {
            return Err(StagecraftError::Api(resp.status().as_u16(), resp.text().await.unwrap_or_default()));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Workspace types (spec 087)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
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
    #[serde(rename = "workspaceId")]
    workspace_id: String,
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
    #[serde(rename = "workspaceId", skip_serializing_if = "Option::is_none")]
    workspace_id: Option<String>,
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
    #[serde(rename = "workspaceId", skip_serializing_if = "Option::is_none")]
    workspace_id: Option<String>,
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
    #[serde(rename = "workspaceId", skip_serializing_if = "Option::is_none")]
    workspace_id: Option<String>,
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
    #[serde(rename = "workspaceId", skip_serializing_if = "Option::is_none")]
    workspace_id: Option<String>,
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
    #[serde(rename = "workspaceId", skip_serializing_if = "Option::is_none")]
    workspace_id: Option<String>,
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
    #[serde(rename = "workspaceId", skip_serializing_if = "Option::is_none")]
    workspace_id: Option<String>,
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
    #[serde(rename = "workspaceId", skip_serializing_if = "Option::is_none")]
    workspace_id: Option<String>,
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
    #[serde(rename = "workspaceId", skip_serializing_if = "Option::is_none")]
    workspace_id: Option<String>,
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
