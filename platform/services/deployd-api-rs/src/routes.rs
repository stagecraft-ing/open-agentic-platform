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
use crate::helm::{self, HelmRunner, InstallRequest};
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

            let values = helm::build_values(&body.artifact_ref, &release_name, &route_pairs);
            let req = InstallRequest {
                chart: chart.clone(),
                namespace: namespace.clone(),
                release: release_name.clone(),
                values,
            };
            let runner = HelmRunner::from_env();
            match tokio::task::spawn_blocking(move || runner.install(&req)).await {
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
    if k8s::probe_cluster().await.is_ok() {
        let namespace = format!("{}-{}", deployment.app_id, deployment.env_id);
        let release = deployment
            .app_slug
            .clone()
            .unwrap_or_else(|| deployment.app_id.clone());
        let runner = HelmRunner::from_env();
        let result =
            tokio::task::spawn_blocking(move || runner.uninstall(&namespace, &release)).await;
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
