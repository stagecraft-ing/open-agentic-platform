//! Helm-driven deploy/destroy. Spec 136 Phase 2.b replaces the kube-rs
//! raw-object construction with `helm upgrade --install --wait` against
//! charts embedded into the binary at compile time, so the deployd-api
//! image ships the charts it deploys — no out-of-band chart registry,
//! no Docker build-context refactor.
//!
//! The runtime model is small:
//!   * `HelmRunner::from_env()` reads `DEPLOYD_HELM_BIN` and
//!     `DEPLOYD_HELM_TIMEOUT`. The chart bytes live in this module via
//!     `include_str!`.
//!   * `prepare_chart(name)` materialises the requested chart into a
//!     per-runner temp dir (cached after the first call) and returns
//!     the path that `helm` will be pointed at.
//!   * `install` / `uninstall` / `template` shell out to `helm` with a
//!     JSON values file (helm accepts JSON since JSON is valid YAML).
//!
//! `template` is the test seam: it executes `helm template` against the
//! embedded chart without touching a cluster, which is what the unit
//! tests assert against. Spec 136 Phase 3 (negative-path validation
//! against a live cluster) remains a follow-up.

use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Embedded chart bytes — spec 136 ships tenant-hello. New charts register
// here by mirroring the const block + the materialiser arm in `write_chart`.
// ---------------------------------------------------------------------------

const TENANT_HELLO_CHART_YAML: &str = include_str!("../../../charts/tenant-hello/Chart.yaml");
const TENANT_HELLO_VALUES_YAML: &str = include_str!("../../../charts/tenant-hello/values.yaml");
const TENANT_HELLO_HELPERS_TPL: &str =
    include_str!("../../../charts/tenant-hello/templates/_helpers.tpl");
const TENANT_HELLO_DEPLOYMENT_YAML: &str =
    include_str!("../../../charts/tenant-hello/templates/deployment.yaml");
const TENANT_HELLO_SERVICE_YAML: &str =
    include_str!("../../../charts/tenant-hello/templates/service.yaml");
const TENANT_HELLO_INGRESS_YAML: &str =
    include_str!("../../../charts/tenant-hello/templates/ingress.yaml");
const TENANT_HELLO_SA_YAML: &str =
    include_str!("../../../charts/tenant-hello/templates/serviceaccount.yaml");

#[derive(Debug)]
pub enum HelmError {
    /// Requested chart is not embedded in this binary.
    UnknownChart(String),
    /// Helm binary failed to spawn (not installed, not executable).
    BinarySpawn(String),
    /// Helm sub-command exited non-zero. `stderr` is the captured tail.
    Invocation {
        stage: &'static str,
        code: Option<i32>,
        stderr: String,
    },
    /// JSON / filesystem step around the helm call failed.
    Io(String),
}

impl std::fmt::Display for HelmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HelmError::UnknownChart(c) => write!(f, "unknown chart: {c}"),
            HelmError::BinarySpawn(m) => write!(f, "helm binary spawn failed: {m}"),
            HelmError::Invocation {
                stage,
                code,
                stderr,
            } => write!(f, "{stage} exited code={code:?}: {stderr}"),
            HelmError::Io(m) => write!(f, "helm io: {m}"),
        }
    }
}

impl std::error::Error for HelmError {}

/// Inputs to a Helm install. `values` is JSON; helm accepts JSON
/// values files because JSON is a strict subset of YAML.
pub struct InstallRequest {
    pub chart: String,
    pub namespace: String,
    pub release: String,
    pub values: Value,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InstallResult {
    pub release: String,
    pub namespace: String,
    pub revision: u32,
    pub status: String,
}

pub struct HelmRunner {
    bin: OsString,
    timeout: String,
    chart_cache: OnceLock<PathBuf>,
}

impl HelmRunner {
    pub fn from_env() -> Self {
        let bin = std::env::var_os("DEPLOYD_HELM_BIN").unwrap_or_else(|| OsString::from("helm"));
        let timeout = std::env::var("DEPLOYD_HELM_TIMEOUT").unwrap_or_else(|_| "5m".into());
        Self {
            bin,
            timeout,
            chart_cache: OnceLock::new(),
        }
    }

    /// Returns the materialised path of an embedded chart, writing it
    /// to a per-runner temp dir on first request. Subsequent calls reuse
    /// the same dir.
    pub fn prepare_chart(&self, name: &str) -> Result<PathBuf, HelmError> {
        let root = self
            .chart_cache
            .get_or_init(|| {
                let dir = std::env::temp_dir().join(format!(
                    "deployd-charts-{}",
                    std::process::id(),
                ));
                let _ = fs::create_dir_all(&dir);
                dir
            })
            .clone();
        let chart_dir = root.join(name);
        if chart_dir.join("Chart.yaml").exists() {
            return Ok(chart_dir);
        }
        write_chart(name, &chart_dir)?;
        Ok(chart_dir)
    }

    /// `helm upgrade --install --create-namespace --wait --output json`.
    pub fn install(&self, req: &InstallRequest) -> Result<InstallResult, HelmError> {
        let chart_dir = self.prepare_chart(&req.chart)?;
        let values_path = write_values_file(&req.values)?;

        let output = Command::new(&self.bin)
            .args([
                "upgrade",
                "--install",
                "--create-namespace",
                "-n",
                &req.namespace,
                &req.release,
                chart_dir.to_str().ok_or_else(|| {
                    HelmError::Io("chart dir path is not valid utf-8".into())
                })?,
                "-f",
                values_path.to_str().ok_or_else(|| {
                    HelmError::Io("values file path is not valid utf-8".into())
                })?,
                "--wait",
                "--timeout",
                &self.timeout,
                "--output",
                "json",
            ])
            .output()
            .map_err(|e| HelmError::BinarySpawn(e.to_string()))?;

        let _ = fs::remove_file(&values_path);

        if !output.status.success() {
            return Err(HelmError::Invocation {
                stage: "helm upgrade --install",
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        let release_info: Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| HelmError::Io(format!("parse helm output JSON: {e}")))?;
        let status = release_info
            .pointer("/info/status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let revision = release_info
            .get("version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        Ok(InstallResult {
            release: req.release.clone(),
            namespace: req.namespace.clone(),
            revision,
            status,
        })
    }

    /// `helm uninstall --wait`. Treats "release not found" as success
    /// so deletes are idempotent against rolled-back deploys.
    pub fn uninstall(&self, namespace: &str, release: &str) -> Result<(), HelmError> {
        let output = Command::new(&self.bin)
            .args([
                "uninstall",
                "-n",
                namespace,
                release,
                "--wait",
                "--timeout",
                &self.timeout,
            ])
            .output()
            .map_err(|e| HelmError::BinarySpawn(e.to_string()))?;

        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not found") {
            return Ok(());
        }
        Err(HelmError::Invocation {
            stage: "helm uninstall",
            code: output.status.code(),
            stderr: stderr.into_owned(),
        })
    }

    /// Run `helm template` against an embedded chart. Used by tests as a
    /// pure-render smoke that never touches a cluster.
    #[allow(dead_code)]
    pub fn template(&self, req: &InstallRequest) -> Result<String, HelmError> {
        let chart_dir = self.prepare_chart(&req.chart)?;
        let values_path = write_values_file(&req.values)?;

        let output = Command::new(&self.bin)
            .args([
                "template",
                &req.release,
                chart_dir.to_str().ok_or_else(|| {
                    HelmError::Io("chart dir path is not valid utf-8".into())
                })?,
                "-n",
                &req.namespace,
                "-f",
                values_path.to_str().ok_or_else(|| {
                    HelmError::Io("values file path is not valid utf-8".into())
                })?,
            ])
            .output()
            .map_err(|e| HelmError::BinarySpawn(e.to_string()))?;

        let _ = fs::remove_file(&values_path);

        if !output.status.success() {
            return Err(HelmError::Invocation {
                stage: "helm template",
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Translate a DeploymentRequest's fields into a values JSON the chart
/// understands. Pure function — no I/O, no helm dependency.
///
/// Mapping:
///   * `image.repository` / `image.tag` derive from `artifact_ref`
///     splitting on the rightmost `:`. An `artifact_ref` with no `:`
///     is treated as repository only with tag `latest`.
///   * `fullnameOverride` defaults to the release name so resources
///     get predictable, slug-driven names.
///   * `ingress` enables when at least one route is supplied, taking
///     the first route's host. The chart's path is hardcoded `/`.
pub fn build_values(
    artifact_ref: &str,
    fullname_override: &str,
    routes: &[(String, String)],
) -> Value {
    let (repository, tag) = split_artifact_ref(artifact_ref);
    let mut values = serde_json::json!({
        "image": {
            "repository": repository,
            "tag": tag,
        },
        "fullnameOverride": fullname_override,
    });
    if let Some((host, _path)) = routes.first() {
        values["ingress"] = serde_json::json!({
            "enabled": true,
            "host": host,
        });
    }
    values
}

fn split_artifact_ref(artifact_ref: &str) -> (String, String) {
    match artifact_ref.rsplit_once(':') {
        Some((repo, tag)) if !tag.contains('/') => (repo.to_string(), tag.to_string()),
        _ => (artifact_ref.to_string(), "latest".to_string()),
    }
}

fn write_values_file(values: &Value) -> Result<PathBuf, HelmError> {
    let json = serde_json::to_vec_pretty(values)
        .map_err(|e| HelmError::Io(format!("serialize values: {e}")))?;
    let path = std::env::temp_dir().join(format!(
        "deployd-values-{}-{}.json",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let mut f = fs::File::create(&path).map_err(|e| HelmError::Io(e.to_string()))?;
    f.write_all(&json).map_err(|e| HelmError::Io(e.to_string()))?;
    Ok(path)
}

fn write_chart(name: &str, dir: &Path) -> Result<(), HelmError> {
    let files: &[(&str, &str)] = match name {
        "tenant-hello" => &[
            ("Chart.yaml", TENANT_HELLO_CHART_YAML),
            ("values.yaml", TENANT_HELLO_VALUES_YAML),
            ("templates/_helpers.tpl", TENANT_HELLO_HELPERS_TPL),
            ("templates/deployment.yaml", TENANT_HELLO_DEPLOYMENT_YAML),
            ("templates/service.yaml", TENANT_HELLO_SERVICE_YAML),
            ("templates/ingress.yaml", TENANT_HELLO_INGRESS_YAML),
            ("templates/serviceaccount.yaml", TENANT_HELLO_SA_YAML),
        ],
        other => return Err(HelmError::UnknownChart(other.to_string())),
    };
    fs::create_dir_all(dir.join("templates")).map_err(|e| HelmError::Io(e.to_string()))?;
    for (rel, contents) in files {
        let path = dir.join(rel);
        fs::write(&path, contents).map_err(|e| HelmError::Io(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn helm_available() -> bool {
        Command::new("helm")
            .arg("version")
            .arg("--short")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[test]
    fn build_values_splits_image_ref_at_colon() {
        let v = build_values(
            "ghcr.io/org/tenant-hello:v1.2.3",
            "myapp-prod",
            &[],
        );
        assert_eq!(v["image"]["repository"], "ghcr.io/org/tenant-hello");
        assert_eq!(v["image"]["tag"], "v1.2.3");
        assert_eq!(v["fullnameOverride"], "myapp-prod");
        assert!(v.get("ingress").is_none(), "no routes ⇒ no ingress block");
    }

    #[test]
    fn build_values_handles_tagless_image_ref() {
        let v = build_values("ghcr.io/org/tenant-hello", "myapp-prod", &[]);
        assert_eq!(v["image"]["repository"], "ghcr.io/org/tenant-hello");
        assert_eq!(v["image"]["tag"], "latest");
    }

    #[test]
    fn build_values_enables_ingress_when_route_present() {
        let v = build_values(
            "ghcr.io/org/tenant-hello:v1",
            "myapp-prod",
            &[("tenant-hello.example.com".into(), "/".into())],
        );
        assert_eq!(v["ingress"]["enabled"], true);
        assert_eq!(v["ingress"]["host"], "tenant-hello.example.com");
    }

    #[test]
    fn prepare_chart_materialises_all_files() {
        let runner = HelmRunner::from_env();
        let dir = runner.prepare_chart("tenant-hello").expect("prepare");
        for rel in [
            "Chart.yaml",
            "values.yaml",
            "templates/_helpers.tpl",
            "templates/deployment.yaml",
            "templates/service.yaml",
            "templates/ingress.yaml",
            "templates/serviceaccount.yaml",
        ] {
            let p = dir.join(rel);
            assert!(p.exists(), "missing materialised file: {rel}");
            let bytes = fs::read(&p).unwrap();
            assert!(!bytes.is_empty(), "empty materialised file: {rel}");
        }
    }

    #[test]
    fn prepare_chart_rejects_unknown_chart() {
        let runner = HelmRunner::from_env();
        let err = runner.prepare_chart("not-a-chart").unwrap_err();
        match err {
            HelmError::UnknownChart(s) => assert_eq!(s, "not-a-chart"),
            other => panic!("expected UnknownChart, got {other:?}"),
        }
    }

    #[test]
    fn template_renders_tenant_hello_without_ingress() {
        if !helm_available() {
            eprintln!("skipping: helm binary not in PATH");
            return;
        }
        let runner = HelmRunner::from_env();
        let req = InstallRequest {
            chart: "tenant-hello".into(),
            namespace: "myapp-prod".into(),
            release: "myapp".into(),
            values: build_values(
                "ghcr.io/org/tenant-hello:v1.2.3",
                "myapp-prod",
                &[],
            ),
        };
        let rendered = runner.template(&req).expect("helm template should succeed");
        // C-clause assertions: chart enforces the contract.
        assert!(rendered.contains("kind: Deployment"), "Deployment present");
        assert!(rendered.contains("kind: Service"), "Service present");
        assert!(rendered.contains("kind: ServiceAccount"), "SA present");
        assert!(
            !rendered.contains("kind: Ingress"),
            "Ingress should be absent when ingress disabled"
        );
        assert!(
            rendered.contains("ghcr.io/org/tenant-hello"),
            "image repo flowed through"
        );
        assert!(rendered.contains("v1.2.3"), "image tag flowed through");
        assert!(rendered.contains("/healthz"), "readiness/liveness probe on /healthz (C-002)");
        assert!(rendered.contains("name: PORT"), "PORT env injected (C-003)");
        assert!(
            rendered.contains("runAsNonRoot: true"),
            "non-root by-policy (C-001)"
        );
        assert!(
            rendered.contains("oap.spec: \"136-tenant-hello-demo-service\""),
            "oap.spec label asserts spec provenance"
        );
    }

    #[test]
    fn template_renders_ingress_when_route_supplied() {
        if !helm_available() {
            eprintln!("skipping: helm binary not in PATH");
            return;
        }
        let runner = HelmRunner::from_env();
        let req = InstallRequest {
            chart: "tenant-hello".into(),
            namespace: "myapp-prod".into(),
            release: "myapp".into(),
            values: build_values(
                "ghcr.io/org/tenant-hello:v1",
                "myapp-prod",
                &[("hello.example.com".into(), "/".into())],
            ),
        };
        let rendered = runner.template(&req).expect("helm template should succeed");
        assert!(rendered.contains("kind: Ingress"));
        assert!(rendered.contains("hello.example.com"));
    }

    #[test]
    fn template_reports_invocation_error_on_unknown_chart() {
        // No helm needed: prepare_chart fails before we shell out.
        let runner = HelmRunner::from_env();
        let req = InstallRequest {
            chart: "no-such-chart".into(),
            namespace: "any".into(),
            release: "x".into(),
            values: serde_json::json!({}),
        };
        let err = runner.template(&req).unwrap_err();
        assert!(matches!(err, HelmError::UnknownChart(_)));
    }
}
