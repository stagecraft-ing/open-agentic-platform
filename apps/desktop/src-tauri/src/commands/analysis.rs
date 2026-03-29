use tauri::command;
use xray::scan_target;
use featuregraph::tools::FeatureGraphTools;
use serde::Serialize;
use serde_json::{json, Value};
use specta::Type;
use std::fs;
use std::path::PathBuf;

#[command]
pub async fn xray_scan_project(path: String) -> Result<serde_json::Value, String> {
    let target = PathBuf::from(&path);
    let index = scan_target(&target, None).map_err(|e| e.to_string())?;
    serde_json::to_value(&index).map_err(|e| e.to_string())
}

#[command]
pub async fn featuregraph_overview(features_yaml_path: String) -> Result<serde_json::Value, String> {
    let repo_root = resolve_repo_root(&features_yaml_path);
    let registry_path = repo_root.join("build/spec-registry/registry.json");

    let registry = match read_registry_summary(&registry_path) {
        Ok(summary) => json!({
            "status": "ok",
            "path": registry_path,
            "summary": summary,
        }),
        Err(err) => json!({
            "status": "unavailable",
            "path": registry_path,
            "message": err,
        }),
    };

    let fg_tools = FeatureGraphTools::new();
    let featuregraph = match fg_tools.features_overview(&repo_root, None) {
        Ok(graph) => {
            let feature_count = graph
                .get("features")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0);
            let violations_count = graph
                .get("violations")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .unwrap_or(0);

            json!({
                "status": "ok",
                "summary": {
                    "featureCount": feature_count,
                    "violationsCount": violations_count,
                },
            })
        }
        Err(err) => json!({
            "status": "unavailable",
            "message": err.to_string(),
        }),
    };

    let overall_status = if registry["status"] == "ok" && featuregraph["status"] == "ok" {
        "success"
    } else {
        "degraded"
    };

    Ok(json!({
        "status": overall_status,
        "repoRoot": repo_root,
        "registry": registry,
        "featuregraph": featuregraph,
    }))
}

/// Read-only labels for `featuregraph::preflight::SafetyTier` (governance UI).
#[derive(Debug, Clone, Serialize, Type)]
pub struct SafetyTierRef {
    pub id: String,
    pub label: String,
    pub description: String,
}

#[command]
#[specta::specta]
pub fn get_preflight_safety_tier_reference() -> Vec<SafetyTierRef> {
    vec![
        SafetyTierRef {
            id: "tier1".into(),
            label: "Tier 1".into(),
            description: "Autonomous".into(),
        },
        SafetyTierRef {
            id: "tier2".into(),
            label: "Tier 2".into(),
            description: "Gated".into(),
        },
        SafetyTierRef {
            id: "tier3".into(),
            label: "Tier 3".into(),
            description: "Forbidden".into(),
        },
    ]
}

#[command]
pub async fn featuregraph_impact(file_paths: Vec<String>, features_yaml_path: String) -> Result<serde_json::Value, String> {
    let repo_root = resolve_repo_root(&features_yaml_path);
    let fg_tools = FeatureGraphTools::new();
    fg_tools
        .features_impact(&repo_root.to_string_lossy(), &file_paths)
        .map_err(|e| e.to_string())
}

fn resolve_repo_root(input: &str) -> PathBuf {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    }
    PathBuf::from(trimmed)
}

fn read_registry_summary(path: &PathBuf) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("Failed reading registry: {e}"))?;
    let parsed: Value =
        serde_json::from_str(&raw).map_err(|e| format!("Failed parsing registry JSON: {e}"))?;

    let features = parsed
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| "Registry missing features array".to_string())?;
    let validation = parsed
        .get("validation")
        .and_then(Value::as_object)
        .ok_or_else(|| "Registry missing validation object".to_string())?;

    let validation_passed = validation
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut status_counts = serde_json::Map::new();
    for feature in features {
        let status = feature
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let prev = status_counts
            .get(status)
            .and_then(Value::as_u64)
            .unwrap_or(0);
        status_counts.insert(status.to_string(), Value::from(prev + 1));
    }

    let violations_count = validation
        .get("violations")
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);

    let mut feature_summaries = Vec::new();
    for feature in features {
        let Some(obj) = feature.as_object() else {
            continue;
        };
        let id = obj
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let title = obj
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let spec_path = obj
            .get("specPath")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if spec_path.is_empty() {
            continue;
        }
        feature_summaries.push(json!({
            "id": id,
            "title": title,
            "specPath": spec_path,
        }));
    }

    Ok(json!({
        "featureCount": features.len(),
        "validationPassed": validation_passed,
        "violationsCount": violations_count,
        "statusCounts": status_counts,
        "featureSummaries": feature_summaries,
    }))
}

#[cfg(test)]
mod tests {
    use super::read_registry_summary;
    use std::io::Write;

    #[test]
    fn read_registry_summary_parses_counts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("registry.json");
        let mut file = std::fs::File::create(&path).expect("file");
        writeln!(
            file,
            r#"{{
  "features":[
    {{"id":"001","status":"active","title":"A","specPath":"specs/001-a/spec.md"}},
    {{"id":"002","status":"active","title":"B","specPath":"specs/002-b/spec.md"}},
    {{"id":"003","status":"draft","title":"C","specPath":"specs/003-c/spec.md"}}
  ],
  "validation":{{"passed":true,"violations":[]}}
}}"#
        )
        .expect("write");

        let summary = read_registry_summary(&path).expect("summary");
        assert_eq!(summary["featureCount"], 3);
        assert_eq!(summary["validationPassed"], true);
        assert_eq!(summary["statusCounts"]["active"], 2);
        assert_eq!(summary["statusCounts"]["draft"], 1);
        let fs = summary["featureSummaries"].as_array().expect("featureSummaries");
        assert_eq!(fs.len(), 3);
        assert_eq!(fs[0]["specPath"], "specs/001-a/spec.md");
    }

    #[test]
    fn read_registry_summary_rejects_missing_features() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("registry.json");
        let mut file = std::fs::File::create(&path).expect("file");
        writeln!(file, r#"{{"validation":{{"passed":true}}}}"#).expect("write");

        let err = read_registry_summary(&path).expect_err("expected error");
        assert!(err.contains("features array"));
    }
}
