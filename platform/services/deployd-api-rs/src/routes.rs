use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::auth;
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
            )
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

    let deployment = Deployment {
        deployment_id: deployment_id.clone(),
        deployment_key,
        tenant_id: body.tenant_id,
        app_id: body.app_id,
        env_id: body.env_id,
        release_sha: body.release_sha,
        artifact_ref: body.artifact_ref,
        lane: body.lane.clone(),
        status: "PENDING".to_string(),
        app_slug: body.app_slug,
        env_slug: body.env_slug,
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

    // For Lane A: namespace + helm deploy would go here.
    // For now, mark as ROLLED_OUT (K8s integration deferred to when kube crate is wired).
    let _ = store::update_status(&state.client, &deployment_id, "ROLLED_OUT").await;
    let _ = store::add_event(
        &state.client,
        &deployment_id,
        "rolled_out",
        Some("deployment recorded"),
    )
    .await;

    (
        StatusCode::OK,
        Json(json!({
            "release_id": deployment_id,
            "status": "ROLLED_OUT",
            "endpoints": endpoints,
            "logs_pointer": format!("/v1/deployments/{}/logs", deployment_id),
        })),
    )
}

pub async fn get_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(release_id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) =
        auth::verify_jwt(&headers, &state.config.oidc_endpoint, &state.config.audience).await
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
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not_found"})),
        ),
    }
}

pub async fn get_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(release_id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) =
        auth::verify_jwt(&headers, &state.config.oidc_endpoint, &state.config.audience).await
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
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "not_found"})),
        ),
    }
}
