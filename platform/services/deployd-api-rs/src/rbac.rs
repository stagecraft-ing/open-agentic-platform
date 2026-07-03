//! Self-provisioned per-namespace RBAC (additive, opt-in).
//!
//! Background: the deployd-api Helm chart splits its permissions into two
//! ClusterRoles: `deployd-controller-namespaces` (cluster-scoped, always
//! bound cluster-wide) and `deployd-controller-workloads` (namespaced,
//! bound either per-namespace via `rbac.namespaces` or, by default,
//! cluster-wide as a fallback). The cluster-wide workloads fallback exists
//! only because deployd creates tenant namespaces on demand and had no way
//! to grant itself workload rights in a namespace it just created.
//!
//! This module closes that gap: when `DEPLOYD_SELF_PROVISION_RBAC` is on,
//! deployd creates a RoleBinding in each target namespace granting its own
//! ServiceAccount the `deployd-controller-workloads` ClusterRole, right
//! before `helm upgrade --install` runs there. Once operators run in this
//! mode the chart can drop the cluster-wide workloads ClusterRoleBinding
//! entirely.
//!
//! This is ADDITIVE and opt-in: the default (`enabled = false`) is a no-op,
//! so the existing cluster-wide fallback keeps working untouched. FLIPPING
//! the chart default to scoped is intentionally NOT done here; it is gated
//! on real-cluster validation (see the chart's rbac.yaml header and the
//! spec 136 Phase 3 precedent).

use k8s_openapi::api::core::v1::Namespace;
use k8s_openapi::api::rbac::v1::{RoleBinding, RoleRef, Subject};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::{Api, PostParams};
use kube::Client;
use std::collections::BTreeMap;

/// Configuration for self-provisioned per-namespace RBAC, read from the
/// process environment (mirrors `HelmRunner::from_env` / `BackupConfig`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelfProvisionConfig {
    /// Master switch. Off by default so this whole path is inert unless an
    /// operator opts in.
    pub enabled: bool,
    /// deployd's own ServiceAccount name (the RoleBinding subject).
    pub service_account: String,
    /// The namespace deployd's ServiceAccount lives in.
    pub service_account_namespace: String,
    /// The ClusterRole the RoleBinding references (and its own name).
    pub cluster_role: String,
}

impl SelfProvisionConfig {
    pub fn from_env() -> Self {
        Self::from_var_lookup(|k| std::env::var(k).ok())
    }

    fn from_var_lookup<F>(get: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let v = |k: &str| get(k).filter(|s| !s.is_empty());
        let enabled = v("DEPLOYD_SELF_PROVISION_RBAC")
            .map(|s| s == "true" || s == "1")
            .unwrap_or(false);
        Self {
            enabled,
            service_account: v("DEPLOYD_SERVICE_ACCOUNT")
                .unwrap_or_else(|| "deployd-api".to_string()),
            service_account_namespace: v("DEPLOYD_POD_NAMESPACE")
                .unwrap_or_else(|| "deployd-system".to_string()),
            cluster_role: v("DEPLOYD_WORKLOADS_CLUSTER_ROLE")
                .unwrap_or_else(|| "deployd-controller-workloads".to_string()),
        }
    }
}

/// Pure constructor for the RoleBinding object. Kept separate from the I/O
/// so its shape (roleRef, subject, name, namespace) is unit-testable
/// without a cluster.
pub fn workload_rolebinding(namespace: &str, cfg: &SelfProvisionConfig) -> RoleBinding {
    let mut labels = BTreeMap::new();
    labels.insert(
        "app.kubernetes.io/managed-by".to_string(),
        "deployd-api".to_string(),
    );
    RoleBinding {
        metadata: ObjectMeta {
            name: Some(cfg.cluster_role.clone()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        role_ref: RoleRef {
            api_group: "rbac.authorization.k8s.io".to_string(),
            kind: "ClusterRole".to_string(),
            name: cfg.cluster_role.clone(),
        },
        subjects: Some(vec![Subject {
            kind: "ServiceAccount".to_string(),
            name: cfg.service_account.clone(),
            namespace: Some(cfg.service_account_namespace.clone()),
            ..Default::default()
        }]),
    }
}

#[derive(Debug)]
pub enum RbacError {
    Client(String),
    Namespace(String),
    RoleBinding(String),
}

impl std::fmt::Display for RbacError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RbacError::Client(m) => write!(f, "kube client: {m}"),
            RbacError::Namespace(m) => write!(f, "ensure namespace: {m}"),
            RbacError::RoleBinding(m) => write!(f, "ensure rolebinding: {m}"),
        }
    }
}

impl std::error::Error for RbacError {}

/// Ensure the target namespace exists and carries a RoleBinding granting
/// deployd's ServiceAccount the workloads ClusterRole. Idempotent: an
/// already-existing namespace or RoleBinding (HTTP 409) is treated as
/// success. A no-op (returns `Ok(())`) when `cfg.enabled` is false.
///
/// Must run BEFORE `helm upgrade --install` for the namespace so helm's
/// workload objects land with deployd already holding rights there under
/// scoped RBAC. Under the default cluster-wide fallback this is unnecessary
/// (hence opt-in).
pub async fn ensure_workload_rbac(
    namespace: &str,
    cfg: &SelfProvisionConfig,
) -> Result<(), RbacError> {
    if !cfg.enabled {
        return Ok(());
    }

    let client = Client::try_default()
        .await
        .map_err(|e| RbacError::Client(e.to_string()))?;

    // 1. Ensure the namespace exists. deployd holds cluster-wide namespace
    //    create via `deployd-controller-namespaces`, so this works before
    //    the RoleBinding (which is namespace-scoped) can be created.
    let ns_api: Api<Namespace> = Api::all(client.clone());
    let exists = ns_api
        .get_opt(namespace)
        .await
        .map_err(|e| RbacError::Namespace(e.to_string()))?
        .is_some();
    if !exists {
        let ns = Namespace {
            metadata: ObjectMeta {
                name: Some(namespace.to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        match ns_api.create(&PostParams::default(), &ns).await {
            Ok(_) => {}
            // Lost a create race with a concurrent deploy, which is fine.
            Err(kube::Error::Api(ae)) if ae.code == 409 => {}
            Err(e) => return Err(RbacError::Namespace(e.to_string())),
        }
    }

    // 2. Ensure the RoleBinding. Creating it requires deployd to hold the
    //    `bind` verb on the referenced ClusterRole (Kubernetes privilege-
    //    escalation prevention) plus `create` on rolebindings, both granted
    //    by the chart when self-provisioning is enabled.
    let rb_api: Api<RoleBinding> = Api::namespaced(client, namespace);
    let rb = workload_rolebinding(namespace, cfg);
    match rb_api.create(&PostParams::default(), &rb).await {
        Ok(_) => Ok(()),
        // Already provisioned on a previous deploy to this namespace.
        Err(kube::Error::Api(ae)) if ae.code == 409 => Ok(()),
        Err(e) => Err(RbacError::RoleBinding(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SelfProvisionConfig {
        SelfProvisionConfig {
            enabled: true,
            service_account: "deployd-api".to_string(),
            service_account_namespace: "deployd-system".to_string(),
            cluster_role: "deployd-controller-workloads".to_string(),
        }
    }

    #[test]
    fn from_env_defaults_disabled_with_sane_names() {
        let c = SelfProvisionConfig::from_var_lookup(|_| None);
        assert!(!c.enabled);
        assert_eq!(c.service_account, "deployd-api");
        assert_eq!(c.service_account_namespace, "deployd-system");
        assert_eq!(c.cluster_role, "deployd-controller-workloads");
    }

    #[test]
    fn from_env_enabled_only_on_true_or_one() {
        let map = |val: &str| {
            let owned = val.to_string();
            SelfProvisionConfig::from_var_lookup(move |k| {
                if k == "DEPLOYD_SELF_PROVISION_RBAC" {
                    Some(owned.clone())
                } else {
                    None
                }
            })
        };
        assert!(map("true").enabled);
        assert!(map("1").enabled);
        assert!(!map("false").enabled);
        assert!(!map("0").enabled);
        assert!(!map("yes").enabled);
        assert!(!map("").enabled);
    }

    #[test]
    fn from_env_overrides_names() {
        let c = SelfProvisionConfig::from_var_lookup(|k| match k {
            "DEPLOYD_SELF_PROVISION_RBAC" => Some("true".to_string()),
            "DEPLOYD_SERVICE_ACCOUNT" => Some("custom-sa".to_string()),
            "DEPLOYD_POD_NAMESPACE" => Some("custom-ns".to_string()),
            "DEPLOYD_WORKLOADS_CLUSTER_ROLE" => Some("custom-role".to_string()),
            _ => None,
        });
        assert!(c.enabled);
        assert_eq!(c.service_account, "custom-sa");
        assert_eq!(c.service_account_namespace, "custom-ns");
        assert_eq!(c.cluster_role, "custom-role");
    }

    #[test]
    fn rolebinding_references_clusterrole_and_binds_the_sa() {
        let rb = workload_rolebinding("tenant-acme-prod", &cfg());

        // Named after the ClusterRole, in the target namespace.
        assert_eq!(
            rb.metadata.name.as_deref(),
            Some("deployd-controller-workloads")
        );
        assert_eq!(rb.metadata.namespace.as_deref(), Some("tenant-acme-prod"));

        // roleRef points at the workloads ClusterRole.
        assert_eq!(rb.role_ref.kind, "ClusterRole");
        assert_eq!(rb.role_ref.name, "deployd-controller-workloads");
        assert_eq!(rb.role_ref.api_group, "rbac.authorization.k8s.io");

        // Subject is deployd's own ServiceAccount in its own namespace.
        let subjects = rb.subjects.expect("subjects present");
        assert_eq!(subjects.len(), 1);
        assert_eq!(subjects[0].kind, "ServiceAccount");
        assert_eq!(subjects[0].name, "deployd-api");
        assert_eq!(subjects[0].namespace.as_deref(), Some("deployd-system"));
    }

    #[tokio::test]
    async fn ensure_is_a_noop_when_disabled() {
        let mut c = cfg();
        c.enabled = false;
        // No kube client is built when disabled, so this succeeds with no
        // cluster reachable (proves the opt-in guard short-circuits first).
        assert!(ensure_workload_rbac("tenant-acme-prod", &c).await.is_ok());
    }
}
