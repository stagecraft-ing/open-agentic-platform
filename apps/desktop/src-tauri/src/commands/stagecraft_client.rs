//! HTTP client for Stagecraft Factory Lifecycle API (Spec 077).
//!
//! Provides dual-write capability: the local Factory engine executes pipelines
//! while this client mirrors lifecycle events to the Stagecraft control plane
//! for centralized audit, token tracking, and governance.
//!
//! All methods are best-effort — callers log warnings on failure but never
//! block local pipeline execution.

use reqwest::Client;
use serde::{Deserialize, Serialize};
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

/// Thin HTTP wrapper around the Stagecraft Factory API.
///
/// Held as Tauri managed state so all commands share one connection pool.
#[derive(Clone)]
pub struct StagecraftClient {
    client: Client,
    base_url: String,
    /// Default actor identity sent on mutating requests.
    actor_user_id: String,
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
        })
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
        let body = ConfirmRequest {
            notes: notes.map(String::from),
            actor_user_id: self.actor_user_id.clone(),
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
        let body = RejectRequest {
            feedback: feedback.into(),
            actor_user_id: self.actor_user_id.clone(),
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
struct TokenSpendRequest {
    run_id: String,
    stage_id: String,
    prompt_tokens: u64,
    completion_tokens: u64,
    model: String,
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
