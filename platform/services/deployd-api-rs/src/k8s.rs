//! Kubernetes deployment operations using kube-rs.
//!
//! Creates a Namespace, Deployment, Service, and optional Ingress for each
//! deployment request. Falls back gracefully if no K8s cluster is reachable.

use k8s_openapi::api::apps::v1 as apps;
use k8s_openapi::api::core::v1 as core;
use k8s_openapi::api::networking::v1 as networking;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::Client;
use kube::api::{Api, PostParams};
use serde_json::json;
use std::collections::BTreeMap;

use crate::store::Deployment;

/// Errors from K8s operations.
#[derive(Debug)]
pub enum K8sError {
    /// No K8s cluster is reachable (e.g., no kubeconfig, no in-cluster config).
    NoCluster(String),
    /// K8s API call failed.
    Api(String),
}

impl std::fmt::Display for K8sError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            K8sError::NoCluster(msg) => write!(f, "no K8s cluster: {msg}"),
            K8sError::Api(msg) => write!(f, "K8s API error: {msg}"),
        }
    }
}

/// Check if a K8s cluster is reachable. Returns a kube Client or an error.
pub async fn try_connect() -> Result<Client, K8sError> {
    Client::try_default()
        .await
        .map_err(|e| K8sError::NoCluster(e.to_string()))
}

/// Deploy a container to Kubernetes.
///
/// Creates (or updates if existing):
/// 1. Namespace: `{app_id}-{env_id}`
/// 2. Deployment: single-replica pod running `artifact_ref`
/// 3. Service: ClusterIP on port 8080
/// 4. Ingress: one rule per desired_route (if any)
///
/// Returns the list of endpoint URLs.
pub async fn deploy(
    client: &Client,
    deployment: &Deployment,
    routes: &[(String, String)], // (host, path) pairs
) -> Result<Vec<String>, K8sError> {
    let namespace = format!("{}-{}", deployment.app_id, deployment.env_id);
    let labels = BTreeMap::from([
        ("app".to_string(), deployment.app_id.clone()),
        ("env".to_string(), deployment.env_id.clone()),
        ("release".to_string(), deployment.release_sha.clone()),
    ]);

    // 1. Ensure namespace exists
    ensure_namespace(client, &namespace).await?;

    // 2. Create Deployment
    create_deployment(client, &namespace, &labels, &deployment.artifact_ref).await?;

    // 3. Create Service
    create_service(client, &namespace, &labels).await?;

    // 4. Create Ingress if routes provided
    let mut endpoints = Vec::new();
    if !routes.is_empty() {
        create_ingress(client, &namespace, routes).await?;
        for (host, path) in routes {
            endpoints.push(format!("https://{host}{path}"));
        }
    }

    Ok(endpoints)
}

/// Delete all resources in the deployment namespace.
pub async fn destroy(client: &Client, app_id: &str, env_id: &str) -> Result<(), K8sError> {
    let namespace = format!("{app_id}-{env_id}");
    let ns_api: Api<core::Namespace> = Api::all(client.clone());

    ns_api
        .delete(&namespace, &Default::default())
        .await
        .map_err(|e| K8sError::Api(format!("delete namespace {namespace}: {e}")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async fn ensure_namespace(client: &Client, name: &str) -> Result<(), K8sError> {
    let ns_api: Api<core::Namespace> = Api::all(client.clone());

    // Check if exists
    if ns_api
        .get_opt(name)
        .await
        .map_err(|e| K8sError::Api(e.to_string()))?
        .is_some()
    {
        return Ok(());
    }

    let ns = core::Namespace {
        metadata: ObjectMeta {
            name: Some(name.to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    ns_api
        .create(&PostParams::default(), &ns)
        .await
        .map_err(|e| K8sError::Api(format!("create namespace {name}: {e}")))?;
    Ok(())
}

async fn create_deployment(
    client: &Client,
    namespace: &str,
    labels: &BTreeMap<String, String>,
    image: &str,
) -> Result<(), K8sError> {
    let deploy_api: Api<apps::Deployment> = Api::namespaced(client.clone(), namespace);
    let name = labels.get("app").cloned().unwrap_or_else(|| "app".into());

    let deploy = serde_json::from_value(json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": name,
            "namespace": namespace,
            "labels": labels,
        },
        "spec": {
            "replicas": 1,
            "selector": {
                "matchLabels": { "app": name }
            },
            "template": {
                "metadata": {
                    "labels": { "app": name }
                },
                "spec": {
                    "containers": [{
                        "name": name,
                        "image": image,
                        "ports": [{ "containerPort": 8080 }],
                        "resources": {
                            "requests": { "cpu": "100m", "memory": "128Mi" },
                            "limits": { "cpu": "500m", "memory": "512Mi" }
                        }
                    }]
                }
            }
        }
    }))
    .map_err(|e| K8sError::Api(format!("serialize deployment: {e}")))?;

    deploy_api
        .create(&PostParams::default(), &deploy)
        .await
        .map_err(|e| K8sError::Api(format!("create deployment {name}: {e}")))?;

    Ok(())
}

async fn create_service(
    client: &Client,
    namespace: &str,
    labels: &BTreeMap<String, String>,
) -> Result<(), K8sError> {
    let svc_api: Api<core::Service> = Api::namespaced(client.clone(), namespace);
    let name = labels.get("app").cloned().unwrap_or_else(|| "app".into());

    let svc = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": {
            "name": name,
            "namespace": namespace,
        },
        "spec": {
            "selector": { "app": name },
            "ports": [{
                "port": 80,
                "targetPort": 8080,
                "protocol": "TCP"
            }],
            "type": "ClusterIP"
        }
    }))
    .map_err(|e| K8sError::Api(format!("serialize service: {e}")))?;

    svc_api
        .create(&PostParams::default(), &svc)
        .await
        .map_err(|e| K8sError::Api(format!("create service {name}: {e}")))?;

    Ok(())
}

async fn create_ingress(
    client: &Client,
    namespace: &str,
    routes: &[(String, String)],
) -> Result<(), K8sError> {
    let ingress_api: Api<networking::Ingress> = Api::namespaced(client.clone(), namespace);

    let rules: Vec<serde_json::Value> = routes
        .iter()
        .map(|(host, path)| {
            json!({
                "host": host,
                "http": {
                    "paths": [{
                        "path": path,
                        "pathType": "Prefix",
                        "backend": {
                            "service": {
                                "name": namespace.split('-').next().unwrap_or("app"),
                                "port": { "number": 80 }
                            }
                        }
                    }]
                }
            })
        })
        .collect();

    let ingress = serde_json::from_value(json!({
        "apiVersion": "networking.k8s.io/v1",
        "kind": "Ingress",
        "metadata": {
            "name": format!("{namespace}-ingress"),
            "namespace": namespace,
            "annotations": {
                "nginx.ingress.kubernetes.io/ssl-redirect": "true"
            }
        },
        "spec": {
            "ingressClassName": "nginx",
            "rules": rules
        }
    }))
    .map_err(|e| K8sError::Api(format!("serialize ingress: {e}")))?;

    ingress_api
        .create(&PostParams::default(), &ingress)
        .await
        .map_err(|e| K8sError::Api(format!("create ingress: {e}")))?;

    Ok(())
}
