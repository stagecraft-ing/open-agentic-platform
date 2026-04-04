// Feature: MCP_ROUTER
// Spec: spec/core/router.md

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

use crate::snapshot::lease::{LeaseStore, StaleLeaseError};

pub mod audit_http;
pub mod legacy_provider;
pub mod permissions;
pub mod policy_bundle;
pub mod policy_http;
pub mod provider;

use policy_bundle::{build_tool_call_context, evaluate_loaded_policy, PolicyBundleCache};

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<Value>,
    pub id: Option<Value>,
}

/// AxiomRegentError represents MCP-level errors that are surfaced to clients using
/// string error codes defined by the MCP common schema.
///
/// NOTE: We intentionally avoid adding new dependencies here (e.g. `thiserror`).
#[derive(Debug)]
pub enum AxiomRegentError {
    NotFound(String),
    InvalidArgument(String),
    RepoChanged(String),
    PermissionDenied(String),
    /// Policy kernel / compiled bundle denied the tool call (047) — distinct wire code from [`Self::PermissionDenied`].
    PolicyDenied(String),
    TooLarge(String),
    Internal(String),
}

impl AxiomRegentError {
    pub fn code(&self) -> &'static str {
        match self {
            AxiomRegentError::NotFound(_) => "NOT_FOUND",
            AxiomRegentError::InvalidArgument(_) => "INVALID_ARGUMENT",
            AxiomRegentError::RepoChanged(_) => "REPO_CHANGED",
            AxiomRegentError::PermissionDenied(_) => "PERMISSION_DENIED",
            AxiomRegentError::PolicyDenied(_) => "POLICY_DENIED",
            AxiomRegentError::TooLarge(_) => "TOO_LARGE",
            AxiomRegentError::Internal(_) => "INTERNAL",
        }
    }

    fn message(&self) -> &str {
        match self {
            AxiomRegentError::NotFound(m)
            | AxiomRegentError::InvalidArgument(m)
            | AxiomRegentError::RepoChanged(m)
            | AxiomRegentError::PermissionDenied(m)
            | AxiomRegentError::PolicyDenied(m)
            | AxiomRegentError::TooLarge(m)
            | AxiomRegentError::Internal(m) => m.as_str(),
        }
    }
}

impl std::fmt::Display for AxiomRegentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for AxiomRegentError {}

pub struct Router {
    providers: Vec<Arc<dyn provider::ToolProvider>>,
    lease_store: Arc<LeaseStore>,
    policy_bundle_cache: Arc<PolicyBundleCache>,
    /// Seam B: optional fire-and-forget HTTP forwarder for audit records.
    audit_forwarder: Option<Arc<audit_http::AuditForwarder>>,
    /// Platform integration config (read from env at startup). Used by Seams C/D.
    #[allow(dead_code)]
    platform_config: crate::platform_config::PlatformConfig,
}

impl Router {
    pub fn new(
        providers: Vec<Arc<dyn provider::ToolProvider>>,
        lease_store: Arc<LeaseStore>,
    ) -> Self {
        let platform_cfg = crate::platform_config::PlatformConfig::from_env();
        let audit_forwarder = match (&platform_cfg.audit_url, &platform_cfg.m2m_token) {
            (Some(url), Some(token)) => {
                eprintln!("[platform] audit streaming enabled → {url}");
                Some(Arc::new(audit_http::AuditForwarder::new(
                    url.clone(),
                    token.clone(),
                )))
            }
            _ => None,
        };
        let policy_bundle_cache = Arc::new(PolicyBundleCache::new());

        // Seam A: spawn background policy bundle refresh when PLATFORM_POLICY_URL is set.
        if let (Some(url), Some(token)) = (&platform_cfg.policy_url, &platform_cfg.m2m_token) {
            // Use repo root from OPC_REPO_ROOT env var (or "default" workspace).
            let repo_root = std::env::var("OPC_REPO_ROOT").unwrap_or_else(|_| "default".into());
            eprintln!("[platform] policy bundle refresh enabled → {url}");
            policy_http::spawn_policy_refresh(
                Arc::clone(&policy_bundle_cache),
                url.clone(),
                token.clone(),
                repo_root,
            );
        }

        Self {
            providers,
            lease_store,
            policy_bundle_cache,
            audit_forwarder,
            platform_config: platform_cfg,
        }
    }

    /// Forward an audit payload to the platform if the forwarder is configured (Seam B).
    fn maybe_forward_audit(&self, payload: &serde_json::Value) {
        if let Some(fwd) = &self.audit_forwarder {
            fwd.forward(payload.clone());
        }
    }

    async fn preflight_tool_permission(
        &self,
        id: Option<Value>,
        tool_name: &str,
        args: &serde_json::Map<String, Value>,
    ) -> Option<JsonRpcResponse> {
        let lease_id_str = args.get("lease_id").and_then(|v| v.as_str());
        let lease = match lease_id_str {
            Some(lid) => self.lease_store.get_lease(lid).await,
            None => None,
        };

        // When no lease is found, fall back to the store's default grants so that
        // tool calls without a lease_id are still subject to tier + permission checks
        // rather than silently bypassing enforcement (Risk 1 fix — post-035 hardening).
        let tier_label = agent::safety::get_tool_tier(tool_name).as_str();
        match &lease {
            Some(l) => {
                match permissions::check_tool_permission(tool_name, l) {
                    Ok(()) => {
                        let audit = permissions::audit_tool_dispatch(
                            tool_name,
                            tier_label,
                            "allowed",
                            lease_id_str,
                        );
                        self.maybe_forward_audit(&audit);
                        self.policy_preflight_response(id, tool_name, args, lease_id_str)
                    }
                    Err(e) => {
                        let audit = permissions::audit_tool_dispatch(
                            tool_name,
                            tier_label,
                            "denied",
                            lease_id_str,
                        );
                        self.maybe_forward_audit(&audit);
                        Some(json_rpc_permission_denied(id, &e.to_string()))
                    }
                }
            }
            None => {
                let fallback = self.lease_store.default_grants();
                match permissions::check_grants(tool_name, &fallback) {
                    Ok(()) => {
                        let audit = permissions::audit_tool_dispatch(
                            tool_name,
                            tier_label,
                            "allowed_no_lease",
                            None,
                        );
                        self.maybe_forward_audit(&audit);
                        self.policy_preflight_response(id, tool_name, args, None)
                    }
                    Err(e) => {
                        let audit = permissions::audit_tool_dispatch(
                            tool_name,
                            tier_label,
                            "denied_no_lease",
                            None,
                        );
                        self.maybe_forward_audit(&audit);
                        Some(json_rpc_permission_denied(id, &e.to_string()))
                    }
                }
            }
        }
    }

    /// 047: after tier + permission grants pass, evaluate compiled policy bundle when `repo_root` + bundle exist.
    fn policy_preflight_response(
        &self,
        id: Option<Value>,
        tool_name: &str,
        args: &serde_json::Map<String, Value>,
        lease_id_for_audit: Option<&str>,
    ) -> Option<JsonRpcResponse> {
        let repo_root = args.get("repo_root").and_then(|v| v.as_str())?;
        let bundle = self.policy_bundle_cache.bundle_for_repo_root(repo_root)?;
        let ctx = build_tool_call_context(tool_name, args);
        let decision = evaluate_loaded_policy(&bundle, &ctx)?;
        let tier_label = agent::safety::get_tool_tier(tool_name).as_str();
        let audit = permissions::audit_tool_dispatch(
            tool_name,
            tier_label,
            "policy_denied",
            lease_id_for_audit,
        );
        self.maybe_forward_audit(&audit);
        let msg = format!("{} {:?}", decision.reason, decision.rule_ids);
        Some(json_rpc_policy_denied(id, &msg))
    }

    pub async fn handle_request(&self, req: &JsonRpcRequest) -> JsonRpcResponse {
        match req.method.as_str() {
            "initialize" => json_rpc_ok(
                req.id.clone(),
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": get_server_capabilities(),
                    "serverInfo": { "name": "mcp", "version": "0.1.0" }
                }),
            ),
            "tools/list" => {
                let mut all_tools = Vec::new();
                for p in &self.providers {
                    all_tools.extend(p.tool_schemas());
                }
                json_rpc_ok(req.id.clone(), json!({ "tools": all_tools }))
            }
            "tools/call" => {
                let params = match req.params.as_ref().and_then(|p| p.as_object()) {
                    Some(p) => p,
                    None => return json_rpc_error(req.id.clone(), -32602, "Invalid params"),
                };
                let name = match params.get("name").and_then(|n| n.as_str()) {
                    Some(n) => n,
                    None => return json_rpc_error(req.id.clone(), -32602, "Missing tool name"),
                };
                let args = match params.get("arguments").and_then(|a| a.as_object()) {
                    Some(a) => a,
                    None => return json_rpc_error(req.id.clone(), -32602, "Missing arguments"),
                };

                // Preflight permission check
                if let Some(resp) = self.preflight_tool_permission(req.id.clone(), name, args).await {
                    return resp;
                }

                // Dispatch to first matching provider
                for p in &self.providers {
                    if let Some(result) = p.handle(name, args).await {
                        return handle_tool_result_value(req.id.clone(), result);
                    }
                }
                json_rpc_error(req.id.clone(), -32601, &format!("Tool not found: {}", name))
            }
            _ => json_rpc_error(req.id.clone(), -32601, "Method not found"),
        }
    }
}

fn json_rpc_ok(id: Option<Value>, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(result),
        error: None,
        id,
    }
}

fn json_rpc_error(id: Option<Value>, code: i64, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(json!({
            "code": code,
            "message": message
        })),
        id,
    }
}

fn json_rpc_permission_denied(id: Option<Value>, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(json!({
            "code": AxiomRegentError::PermissionDenied(message.to_string()).code(),
            "message": message,
        })),
        id,
    }
}

fn json_rpc_policy_denied(id: Option<Value>, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: None,
        error: Some(json!({
            "code": AxiomRegentError::PolicyDenied(message.to_string()).code(),
            "message": message,
        })),
        id,
    }
}

fn get_server_capabilities() -> Value {
    json!({
        "tools": {
            "listChanged": true
        },
        "logging": {}
    })
}

fn handle_tool_result_value(id: Option<Value>, result: anyhow::Result<Value>) -> JsonRpcResponse {
    match result {
        Ok(val) => json_rpc_ok(id, json!({ "content": [{ "type": "json", "json": val }] })),
        Err(e) => handle_tool_error(id, e),
    }
}

fn handle_tool_error(id: Option<Value>, e: anyhow::Error) -> JsonRpcResponse {
    if let Some(stale) = e.downcast_ref::<StaleLeaseError>() {
        return JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(json!({
                "code": "STALE_LEASE",
                "message": stale.msg,
                "data": {
                    "lease_id": stale.lease_id,
                    "current_fingerprint": stale.current_fingerprint
                }
            })),
            id,
        };
    }
    json_rpc_error(id, -32603, &format!("Tool failed: {}", e))
}
