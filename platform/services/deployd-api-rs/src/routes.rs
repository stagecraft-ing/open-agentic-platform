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
    /// Optional for backwards compatibility; defaults to "acme-vue-encore",
    /// the sole registered shape after the spec 214 retirement.
    pub chart: Option<String>,
    /// Chart version, mirrors the chartSelector output. Currently advisory:
    /// the chart bundled into deployd-api is pinned by the image build.
    pub chart_version: Option<String>,
    // region: gate-overlay
    /// Spec 137 — per-environment access-gate descriptor. When `Some` with
    /// `enabled: true`, the tenant chart renders auth-url annotations and
    /// the oauth2-proxy-gate chart is installed alongside via
    /// [`HelmRunner::install_with_gate`]. Absent or `enabled: false` flows
    /// through as a direct-exposure tenant deploy (existing behaviour).
    #[serde(default)]
    pub access_gate: Option<AccessGateDescriptor>,
    // endregion gate-overlay
    /// Spec 214 FR-004: opaque app config rendered as container env vars
    /// (chart `extraEnv`). The stagecraft proxy rejects reserved
    /// `ENCORE_` / `KUBERNETES_` prefixes before they reach deployd.
    #[serde(default)]
    pub config_refs: Option<std::collections::BTreeMap<String, String>>,
    /// Spec 214 FR-005: dockerconfigjson pull secret reflected into the
    /// namespace (default `ghcr-pull`, applied by the proxy). Rendered into
    /// `imagePullSecrets` when present.
    #[serde(default)]
    pub image_pull_secret_name: Option<String>,
    /// Spec 214 FR-008: effective K8s namespace forwarded from
    /// `environments.k8sNamespace`. When present (and non-empty) it wins over
    /// the computed `{app_id}-{env_id}` and is persisted on the deployment
    /// row so DELETE and status operate on recorded truth.
    #[serde(default)]
    pub namespace: Option<String>,
    /// Spec 214 FR-006: when true, render the chart's opt-in preview-grade
    /// Postgres so an Encore tenant with a `SQLDatabase` boots against an
    /// in-namespace database. Stagecraft sets this for development/preview
    /// environments; absent/false leaves the chart default (no preview DB).
    #[serde(default)]
    pub preview_database: Option<bool>,
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

    // Reject a token with no subject claim before any deployment row is
    // written. `is_owner_or_admin` (auth.rs) degrades a NULL `owner_sub` to
    // "any caller holding the required scope may act on this deployment",
    // which is meant only as a legacy fallback for rows written before
    // ownership tracking existed; a new NULL-owner row would be open to
    // every future caller with the required scope, not just this one.
    let Some(owner_sub) = claims.sub.clone() else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "unauthorized",
                "message": "token missing subject claim"
            })),
        );
    };

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

    let chart = body.chart.clone().unwrap_or_else(|| "acme-vue-encore".into());
    let chart_version = body
        .chart_version
        .clone()
        .unwrap_or_else(|| "0.1.0".into());

    // Spec 214 FR-008: the forwarded namespace (from environments.k8sNamespace)
    // wins over the computed default and is persisted on the row so DELETE and
    // status operate on recorded truth rather than recomputation.
    let namespace = body
        .namespace
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}-{}", body.app_id, body.env_id));

    // Reject a namespace that isn't a well-formed, non-reserved K8s
    // namespace name. Without this check a caller holding the required
    // scope (e.g. a compromised M2M credential) could point `helm install`
    // at any namespace in the cluster, including cluster-system or
    // platform-owned namespaces (rauthy-system, stagecraft-system, etc.),
    // purely by setting `namespace` on the request body. This is a format
    // plus reserved-name check, not tenant-ownership enforcement; see
    // `is_valid_tenant_namespace`'s doc comment for the isolation lever
    // this does NOT provide.
    if !is_valid_tenant_namespace(&namespace) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid_namespace",
                "message": format!("namespace '{namespace}' is not a valid tenant namespace")
            })),
        );
    }

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
        namespace: Some(namespace.clone()),
        desired_routes: body
            .desired_routes
            .as_ref()
            .map(|r| serde_json::to_string(r).unwrap_or_default()),
        endpoints: Some(serde_json::to_string(&endpoints).unwrap_or_default()),
        created_at: now.clone(),
        updated_at: now,
        owner_sub: Some(owner_sub),
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
            // Spec 214: thread config_refs (FR-004), the pull secret (FR-005),
            // and the reflected wildcard TLS secret (User Story 1) into the
            // chart values. The TLS secret name is deployd operational config
            // (the cluster-reflected `tenants-wildcard-tls`), overridable via
            // DEPLOYD_TENANT_TLS_SECRET.
            let tenant_tls_secret = std::env::var("DEPLOYD_TENANT_TLS_SECRET")
                .unwrap_or_else(|_| "tenants-wildcard-tls".to_string());
            let extras = helm::DeployExtras {
                config_refs: body.config_refs.as_ref(),
                image_pull_secret_name: body.image_pull_secret_name.as_deref(),
                tls_secret_name: Some(tenant_tls_secret.as_str()).filter(|s| !s.is_empty()),
                preview_database: body.preview_database.unwrap_or(false),
            };
            let values = helm::build_values(
                &body.artifact_ref,
                &release_name,
                &route_pairs,
                body.access_gate.as_ref(),
                &extras,
            );
            let tenant_req = InstallRequest {
                chart: chart.clone(),
                namespace: namespace.clone(),
                release: release_name.clone(),
                values,
            };
            let runner = HelmRunner::from_env();

            // Self-provision per-namespace RBAC before helm runs its
            // workloads there. Opt-in (DEPLOYD_SELF_PROVISION_RBAC): a no-op
            // under the default cluster-wide fallback, so existing
            // deployments are unaffected. When enabled and it fails, fail the
            // deploy now with a clear cause rather than let helm fail
            // mid-install with an opaque "forbidden".
            let self_provision = crate::rbac::SelfProvisionConfig::from_env();
            if let Err(e) =
                crate::rbac::ensure_workload_rbac(&namespace, &self_provision).await
            {
                tracing::error!(
                    deployment_id = %deployment_id,
                    "self-provision RBAC failed: {e}"
                );
                let _ = store::update_status(&state.client, &deployment_id, "FAILED").await;
                let _ = store::add_event(
                    &state.client,
                    &deployment_id,
                    "failed",
                    Some(&format!(
                        "self-provision RBAC failed for namespace {namespace}: {e}"
                    )),
                )
                .await;
                return (
                    StatusCode::OK,
                    Json(json!({
                        "release_id": deployment_id,
                        "status": "FAILED",
                        "endpoints": endpoints,
                        "logs_pointer": format!("/v1/deployments/{}/logs", deployment_id),
                        "chart": chart,
                        "chart_version": chart_version,
                    })),
                );
            }

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
                    // Log the full helm stderr at ERROR. Without this the only
                    // record of *why* a deploy failed lived in the hiqlite
                    // `deployment_events` row (session-gated GET), so the pod
                    // logs showed nothing and operators were told to "see
                    // deployd logs" that contained no clue. HelmError's Display
                    // carries the captured stderr tail.
                    tracing::error!(
                        deployment_id = %deployment_id,
                        "helm install failed: {e}"
                    );
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
                    tracing::error!(
                        deployment_id = %deployment_id,
                        "helm task join error: {join_err}"
                    );
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

    match store::get_by_release_id(&state.client, &release_id).await {
        Ok(Some(d)) => {
            // Existence oracle (finding LOW): a non-owner caller who is
            // authenticated and scoped must not be able to distinguish "this
            // deployment exists but isn't mine" from "this deployment
            // doesn't exist". Both cases now return the exact same
            // NOT_FOUND response as the missing-release branch below.
            if !auth::is_owner_or_admin(
                &claims,
                d.owner_sub.as_deref(),
                state.config.admin_scope.as_deref(),
            ) {
                return (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"})));
            }
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

    match store::get_by_release_id(&state.client, &release_id).await {
        Ok(Some(d)) => {
            // Existence oracle (finding LOW): see get_status's identical
            // comment. A non-owner caller gets the same NOT_FOUND response
            // as a missing release, not a distinguishing FORBIDDEN.
            if !auth::is_owner_or_admin(
                &claims,
                d.owner_sub.as_deref(),
                state.config.admin_scope.as_deref(),
            ) {
                return (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"})));
            }
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

    // Existence oracle (finding LOW): see get_status's identical comment.
    // A non-owner caller gets the same NOT_FOUND response as a missing
    // release, not a distinguishing FORBIDDEN.
    if !auth::is_owner_or_admin(
        &claims,
        deployment.owner_sub.as_deref(),
        state.config.admin_scope.as_deref(),
    ) {
        return (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"})));
    }

    // Best-effort helm uninstall; ignore failure to keep delete idempotent.
    // Spec 137 — `uninstall_with_gate` is the universal teardown: it removes
    // the gate release first (no-op if no gate was installed for this
    // deployment) and then the tenant. Both halves treat "release not found"
    // as success, so the call is correct whether the deployment had a gate or
    // not — no per-deployment branch needed.
    if k8s::probe_cluster().await.is_ok() {
        // Spec 214 FR-008: tear down using the recorded namespace; fall back
        // to the computed form for legacy rows written before the column.
        let namespace = deployment
            .namespace
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("{}-{}", deployment.app_id, deployment.env_id));
        let release = deployment
            .app_slug
            .clone()
            .unwrap_or_else(|| deployment.app_id.clone());

        // Spec 225 FR-007: self-provision the workloads RoleBinding before
        // teardown. Once the cluster-wide workloads fallback is dropped
        // (`rbac.selfProvision: true`), a namespace created before the flip and
        // never redeployed has no RoleBinding, so `helm uninstall` would hit
        // Forbidden and the best-effort uninstall below would swallow it,
        // orphaning the namespace's k8s resources. Provisioning here (into the
        // existing namespace only; it will not resurrect a namespace already
        // gone) lets this delete tear the namespace down cleanly. Best-effort
        // like the uninstall: log and continue so delete stays idempotent, and
        // a no-op under the default cluster-wide fallback (opt-in on env).
        //
        // Defense-in-depth: only self-provision into a valid tenant namespace.
        // Unlike `create_deployment` (which validates the request namespace up
        // front), the delete path derives `namespace` from the recorded column
        // or the `app_id-env_id` fallback for legacy rows and does not
        // revalidate it. Because the chart's `bind`/`rolebindings create`
        // grants are cluster-wide (they must be: deployd binds in namespaces it
        // creates on demand), an unvalidated namespace resolving to a reserved
        // one (e.g. `kube-system`) would otherwise get a deployd RoleBinding
        // written there. `is_valid_tenant_namespace` is the same reserved-name
        // guard `create_deployment` applies, so the RBAC write is scoped to
        // real tenant namespaces even when the recorded value is untrusted.
        let self_provision = crate::rbac::SelfProvisionConfig::from_env();
        if self_provision.enabled && !is_valid_tenant_namespace(&namespace) {
            tracing::warn!(
                "skipping teardown self-provision for {release_id}: namespace {namespace} is not a valid tenant namespace"
            );
        } else if let Err(e) =
            crate::rbac::ensure_workload_rbac_for_teardown(&namespace, &self_provision).await
        {
            // Best-effort: the uninstall below still runs. Record the failure
            // as an event so a persistent RBAC-provisioning failure (e.g. chart
            // `rbac.selfProvision` desynced from the env var) is visible in the
            // deployment's audit trail, not only the pod logs, mirroring the
            // deploy path's `store::add_event` on the same underlying failure.
            tracing::warn!("self-provision RBAC before teardown failed for {release_id}: {e}");
            let _ = store::add_event(
                &state.client,
                &release_id,
                "rbac_warning",
                Some(&format!(
                    "teardown self-provision RBAC failed for namespace {namespace}: {e}; uninstall attempted best-effort"
                )),
            )
            .await;
        }

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

/// DNS-1123-label validation for a Kubernetes namespace name, plus a small
/// reserved-namespace blocklist. `create_deployment` forwards `namespace`
/// straight from the request body (spec 214 FR-008 lets the caller override
/// the computed default); without this check a caller holding the required
/// scope could point `helm upgrade --install` at any namespace in the
/// cluster, cluster-system or platform-owned, purely by setting the field.
///
/// This function validates DNS-1123 label shape and blocks the reserved /
/// platform namespaces listed below. It does NOT enforce per-tenant
/// isolation: it cannot distinguish "tenant A's namespace" from "tenant B's
/// namespace" (both are well-formed, non-reserved names), so a caller
/// holding the required scope can still target another tenant's namespace
/// by name. True per-tenant isolation is a separate lever: the chart's
/// `rbac.namespaces` allowlist (see the deployd-api Helm chart) scopes what
/// the deployd-api ServiceAccount is actually permitted to touch in the
/// cluster's RBAC.
fn is_valid_tenant_namespace(ns: &str) -> bool {
    const RESERVED: &[&str] = &[
        "kube-system",
        "kube-public",
        "kube-node-lease",
        "default",
        "deployd-system",
        "rauthy-system",
        "stagecraft-system",
        "monitoring",
        "ingress-nginx",
        "flux-system",
        "kube-flannel",
        "cert-manager",
        "external-secrets",
    ];
    if ns.is_empty() || ns.len() > 63 {
        return false;
    }
    if RESERVED.contains(&ns) {
        return false;
    }
    let bytes = ns.as_bytes();
    let edge_ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    if !edge_ok(bytes[0]) || !edge_ok(bytes[bytes.len() - 1]) {
        return false;
    }
    bytes
        .iter()
        .all(|&b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_tenant_namespace_accepts_well_formed_names() {
        assert!(is_valid_tenant_namespace("acme-p-dev"));
        assert!(is_valid_tenant_namespace("app123-env1"));
        assert!(is_valid_tenant_namespace("a"));
    }

    #[test]
    fn is_valid_tenant_namespace_rejects_reserved_names() {
        assert!(!is_valid_tenant_namespace("kube-system"));
        assert!(!is_valid_tenant_namespace("kube-public"));
        assert!(!is_valid_tenant_namespace("kube-node-lease"));
        assert!(!is_valid_tenant_namespace("default"));
        assert!(!is_valid_tenant_namespace("deployd-system"));
    }

    #[test]
    fn is_valid_tenant_namespace_rejects_platform_namespaces() {
        // A deploy-scope holder must not be able to target the platform's
        // own namespaces by setting `namespace` on the request body.
        assert!(!is_valid_tenant_namespace("rauthy-system"));
        assert!(!is_valid_tenant_namespace("stagecraft-system"));
        assert!(!is_valid_tenant_namespace("monitoring"));
        assert!(!is_valid_tenant_namespace("ingress-nginx"));
        assert!(!is_valid_tenant_namespace("flux-system"));
        assert!(!is_valid_tenant_namespace("kube-flannel"));
        assert!(!is_valid_tenant_namespace("cert-manager"));
        assert!(!is_valid_tenant_namespace("external-secrets"));
    }

    #[test]
    fn is_valid_tenant_namespace_rejects_malformed_names() {
        assert!(!is_valid_tenant_namespace(""));
        assert!(!is_valid_tenant_namespace("-leading-dash"));
        assert!(!is_valid_tenant_namespace("trailing-dash-"));
        assert!(!is_valid_tenant_namespace("Has-Upper-Case"));
        assert!(!is_valid_tenant_namespace("has_underscore"));
        assert!(!is_valid_tenant_namespace("has.dot"));
        assert!(!is_valid_tenant_namespace("has/slash"));
        assert!(!is_valid_tenant_namespace(&"a".repeat(64)));
    }
}
