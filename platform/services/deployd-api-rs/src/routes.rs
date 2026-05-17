use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::auth;
use crate::helm::{self, AccessGateDescriptor, HelmRunner, InstallRequest};
use crate::k8s;
use crate::store::{self, AppState, Deployment};

pub async fn healthz() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
pub struct DeploymentRequest {
    pub tenant_id: String,
    pub app_id: String,
    pub env_id: String,
    pub release_sha: String,
    pub artifact_ref: String,
    pub lane: String,
    pub app_slug: Option<String>,
    pub env_slug: Option<String>,
    pub desired_routes: Option<Vec<RouteSpec>>,
    /// Chart name resolved by stagecraft's chartSelector (spec 136 Phase 2).
    /// Optional for backwards compatibility — defaults to "tenant-hello",
    /// the only registered shape today.
    pub chart: Option<String>,
    /// Chart version, mirrors the chartSelector output. Currently advisory:
    /// the chart bundled into deployd-api is pinned by the image build.
    pub chart_version: Option<String>,
    /// Spec 137 — per-environment access-gate descriptor. When `Some` with
    /// `enabled: true`, the tenant chart renders auth-url annotations and
    /// the oauth2-proxy-gate chart is installed alongside via
    /// [`HelmRunner::install_with_gate`]. Absent or `enabled: false` flows
    /// through as a direct-exposure tenant deploy (existing behaviour).
    #[serde(default)]
    pub access_gate: Option<AccessGateDescriptor>,
}

#[derive(Deserialize, Serialize)]
pub struct RouteSpec {
    pub host: Option<String>,
    pub path: Option<String>,
}

pub async fn create_deployment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DeploymentRequest>,
) -> impl IntoResponse {
    // Auth
    let claims = match auth::verify_jwt(
        &headers,
        &state.config.oidc_endpoint,
        &state.config.audience,
    )
    .await
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized", "message": e.to_string()})),
            );
        }
    };
    if !auth::has_scope(&claims, &state.config.required_scope) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "forbidden",
                "message": format!("missing scope {}", state.config.required_scope)
            })),
        );
    }

    let deployment_key = format!("{}|{}|{}", body.app_id, body.env_id, body.release_sha);

    // Idempotent check
    if let Ok(Some(existing)) = store::get_by_key(&state.client, &deployment_key).await {
        return (
            StatusCode::OK,
            Json(json!({
                "release_id": existing.deployment_id,
                "status": existing.status,
                "endpoints": existing.endpoints,
                "idempotent_replay": true,
            })),
        );
    }

    let deployment_id = format!("rel_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();

    let endpoints: Vec<String> = body
        .desired_routes
        .as_ref()
        .map(|routes| {
            routes
                .iter()
                .map(|r| {
                    let host = r.host.as_deref().unwrap_or("unknown-host");
                    let path = r.path.as_deref().unwrap_or("/");
                    format!("https://{host}{path}")
                })
                .collect()
        })
        .unwrap_or_default();

    let chart = body.chart.clone().unwrap_or_else(|| "tenant-hello".into());
    let chart_version = body
        .chart_version
        .clone()
        .unwrap_or_else(|| "0.1.0".into());

    let deployment = Deployment {
        deployment_id: deployment_id.clone(),
        deployment_key,
        tenant_id: body.tenant_id,
        app_id: body.app_id.clone(),
        env_id: body.env_id.clone(),
        release_sha: body.release_sha,
        artifact_ref: body.artifact_ref.clone(),
        lane: body.lane.clone(),
        status: "PENDING".to_string(),
        app_slug: body.app_slug.clone(),
        env_slug: body.env_slug.clone(),
        desired_routes: body
            .desired_routes
            .as_ref()
            .map(|r| serde_json::to_string(r).unwrap_or_default()),
        endpoints: Some(serde_json::to_string(&endpoints).unwrap_or_default()),
        created_at: now.clone(),
        updated_at: now,
    };

    if let Err(e) = store::insert_deployment(&state.client, &deployment).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "store_error", "message": e.to_string()})),
        );
    }
    let _ = store::add_event(&state.client, &deployment_id, "requested", None).await;

    // Parse routes into (host, path) pairs for the Helm values builder.
    let route_pairs: Vec<(String, String)> = body
        .desired_routes
        .as_ref()
        .map(|routes| {
            routes
                .iter()
                .map(|r| {
                    (
                        r.host.clone().unwrap_or_else(|| "unknown-host".into()),
                        r.path.clone().unwrap_or_else(|| "/".into()),
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let release_name = body
        .app_slug
        .clone()
        .unwrap_or_else(|| body.app_id.clone());
    let namespace = format!("{}-{}", body.app_id, body.env_id);

    // Probe the cluster first. When no cluster is reachable (local dev,
    // record-only deployments), short-circuit to ROLLED_OUT without
    // shelling helm. When the cluster IS reachable, drive helm against
    // the chart resolved upstream by stagecraft's chartSelector.
    let (final_status, final_endpoints) = match k8s::probe_cluster().await {
        Ok(()) => {
            let _ = store::update_status(&state.client, &deployment_id, "DEPLOYING").await;
            let _ = store::add_event(
                &state.client,
                &deployment_id,
                "deploying",
                Some(&format!("applying chart {chart} ({chart_version})")),
            )
            .await;

            // Spec 137 — if the request carries an enabled access-gate
            // descriptor, drive the dual-release install. Otherwise the
            // single-release path (existing spec 136 behaviour).
            let gate_active = body
                .access_gate
                .as_ref()
                .map(|g| g.enabled)
                .unwrap_or(false);
            let values = helm::build_values(
                &body.artifact_ref,
                &release_name,
                &route_pairs,
                body.access_gate.as_ref(),
            );
            let tenant_req = InstallRequest {
                chart: chart.clone(),
                namespace: namespace.clone(),
                release: release_name.clone(),
                values,
            };
            let runner = HelmRunner::from_env();
            let access_gate = body.access_gate.clone();
            let tenant_release_for_gate = release_name.clone();
            let first_host = route_pairs
                .first()
                .map(|(h, _)| h.clone())
                .unwrap_or_default();
            // Spec 137 T045 / FR-009: reconcile flows. When the descriptor
            // toggles enabled true → false (or remains false on a re-deploy
            // of a previously-gated tenant), we must also uninstall any
            // surviving gate release so the tenant Ingress doesn't keep
            // dangling auth-url annotations pointing at a torn-down Service.
            //
            // `helm uninstall` treats "release not found" as success, so the
            // !gate_active branch's gate-cleanup is a no-op when no prior
            // gate existed — safe to invoke unconditionally.
            let namespace_for_gate_cleanup = namespace.clone();
            let release_for_gate_cleanup = release_name.clone();
            let install_outcome = tokio::task::spawn_blocking(move || {
                if gate_active {
                    let descriptor = access_gate.expect("gate_active implies access_gate.is_some");
                    let gate_values = helm::build_gate_values(
                        &descriptor,
                        &tenant_release_for_gate,
                        &first_host,
                    );
                    runner.install_with_gate(&tenant_req, "oauth2-proxy-gate", gate_values)
                } else {
                    let tenant_result = runner.install(&tenant_req)?;
                    // Best-effort gate teardown for the off-transition. Log
                    // but don't fail the deploy: a stale gate is a leak but
                    // not a correctness break for the tenant traffic path
                    // (the tenant Ingress no longer references it).
                    let gate_release = helm::gate_release_name(&release_for_gate_cleanup);
                    if let Err(e) = runner.uninstall(&namespace_for_gate_cleanup, &gate_release) {
                        tracing::warn!(
                            "gate teardown on off-transition failed for {gate_release}: {e}"
                        );
                    }
                    Ok(tenant_result)
                }
            })
            .await;
            match install_outcome {
                Ok(Ok(result)) => {
                    let _ = store::update_status(&state.client, &deployment_id, "ROLLED_OUT").await;
                    let _ = store::add_event(
                        &state.client,
                        &deployment_id,
                        "rolled_out",
                        Some(&format!(
                            "helm release {}/{} revision {} status {}",
                            result.namespace, result.release, result.revision, result.status
                        )),
                    )
                    .await;
                    ("ROLLED_OUT".to_string(), endpoints.clone())
                }
                Ok(Err(e)) => {
                    let _ = store::update_status(&state.client, &deployment_id, "FAILED").await;
                    let _ = store::add_event(
                        &state.client,
                        &deployment_id,
                        "failed",
                        Some(&format!("helm install failed: {e}")),
                    )
                    .await;
                    ("FAILED".to_string(), endpoints.clone())
                }
                Err(join_err) => {
                    let _ = store::update_status(&state.client, &deployment_id, "FAILED").await;
                    let _ = store::add_event(
                        &state.client,
                        &deployment_id,
                        "failed",
                        Some(&format!("helm task join error: {join_err}")),
                    )
                    .await;
                    ("FAILED".to_string(), endpoints.clone())
                }
            }
        }
        Err(_) => {
            let _ = store::update_status(&state.client, &deployment_id, "ROLLED_OUT").await;
            let _ = store::add_event(
                &state.client,
                &deployment_id,
                "rolled_out",
                Some("deployment recorded (no K8s cluster)"),
            )
            .await;
            ("ROLLED_OUT".to_string(), endpoints.clone())
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "release_id": deployment_id,
            "status": final_status,
            "endpoints": final_endpoints,
            "logs_pointer": format!("/v1/deployments/{}/logs", deployment_id),
            "chart": chart,
            "chart_version": chart_version,
        })),
    )
}

pub async fn get_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(release_id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = auth::verify_jwt(
        &headers,
        &state.config.oidc_endpoint,
        &state.config.audience,
    )
    .await
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "unauthorized", "message": e.to_string()})),
        );
    }

    match store::get_by_release_id(&state.client, &release_id).await {
        Ok(Some(d)) => {
            let events = store::get_events(&state.client, &release_id)
                .await
                .unwrap_or_default();
            (
                StatusCode::OK,
                Json(json!({
                    "release_id": d.deployment_id,
                    "status": d.status,
                    "events": events,
                })),
            )
        }
        _ => (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"}))),
    }
}

pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(release_id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = auth::verify_jwt(
        &headers,
        &state.config.oidc_endpoint,
        &state.config.audience,
    )
    .await
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "unauthorized", "message": e.to_string()})),
        );
    }

    match store::get_by_release_id(&state.client, &release_id).await {
        Ok(Some(_)) => {
            let events = store::get_events(&state.client, &release_id)
                .await
                .unwrap_or_default();
            (
                StatusCode::OK,
                Json(json!({
                    "release_id": release_id,
                    "logs": events,
                })),
            )
        }
        _ => (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"}))),
    }
}

pub async fn delete_deployment(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(release_id): Path<String>,
) -> impl IntoResponse {
    // Auth
    let claims = match auth::verify_jwt(
        &headers,
        &state.config.oidc_endpoint,
        &state.config.audience,
    )
    .await
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized", "message": e.to_string()})),
            );
        }
    };
    if !auth::has_scope(&claims, &state.config.required_scope) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "forbidden",
                "message": format!("missing scope {}", state.config.required_scope)
            })),
        );
    }

    let deployment = match store::get_by_release_id(&state.client, &release_id).await {
        Ok(Some(d)) => d,
        _ => return (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"}))),
    };

    // Best-effort helm uninstall; ignore failure to keep delete idempotent.
    // Spec 137 — `uninstall_with_gate` is the universal teardown: it removes
    // the gate release first (no-op if no gate was installed for this
    // deployment) and then the tenant. Both halves treat "release not found"
    // as success, so the call is correct whether the deployment had a gate or
    // not — no per-deployment branch needed.
    if k8s::probe_cluster().await.is_ok() {
        let namespace = format!("{}-{}", deployment.app_id, deployment.env_id);
        let release = deployment
            .app_slug
            .clone()
            .unwrap_or_else(|| deployment.app_id.clone());
        let runner = HelmRunner::from_env();
        let result = tokio::task::spawn_blocking(move || {
            runner.uninstall_with_gate(&namespace, &release)
        })
        .await;
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::warn!("helm uninstall failed for {release_id}: {e}"),
            Err(join_err) => tracing::warn!("helm task join error for {release_id}: {join_err}"),
        }
    }

    let _ = store::update_status(&state.client, &release_id, "DESTROYED").await;
    let _ = store::add_event(
        &state.client,
        &release_id,
        "destroyed",
        Some("deployment destroyed"),
    )
    .await;

    (
        StatusCode::OK,
        Json(json!({
            "release_id": release_id,
            "status": "DESTROYED",
        })),
    )
}
