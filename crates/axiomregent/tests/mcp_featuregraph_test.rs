// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use axiomregent::agent_tools::AgentTools;
use axiomregent::feature_tools::FeatureTools;

use axiomregent::router::JsonRpcRequest;
use axiomregent::router::Router;
use axiomregent::snapshot::tools::SnapshotTools;
use axiomregent::workspace::WorkspaceTools;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;

/// Create a self-contained test workspace with a minimal spec registry so the
/// featuregraph scanner can initialise without requiring `spec-compiler compile`.
fn create_test_workspace() -> TempDir {
    let dir = TempDir::new().expect("failed to create temp dir");
    let registry_dir = dir.path().join("build/spec-registry");
    std::fs::create_dir_all(&registry_dir).unwrap();
    std::fs::write(
        registry_dir.join("registry.json"),
        r#"{"features":[{"id":"test-feature","title":"Test Feature","specPath":"specs/test/spec.md","status":"active","codeAliases":[]}]}"#,
    )
    .unwrap();
    // Create a dummy source file so features.impact has something to scan
    let src_dir = dir.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("feature_tools.rs"), "// Feature: test-feature\nfn main() {}\n").unwrap();
    dir
}

fn create_router() -> Router {
    let lease_store = Arc::new(axiomregent::snapshot::lease::LeaseStore::new());
    let storage_config = axiomregent::config::StorageConfig::default();
    let store = Arc::new(axiomregent::snapshot::store::Store::new(storage_config).unwrap());

    let snapshot_tools = Arc::new(SnapshotTools::new(lease_store.clone(), store.clone()));
    let workspace_tools = Arc::new(WorkspaceTools::new(lease_store.clone(), store.clone()));
    let featuregraph_tools = Arc::new(axiomregent::featuregraph::tools::FeatureGraphTools::new());
    let feature_tools = Arc::new(FeatureTools::new());
    let xray_tools = Arc::new(axiomregent::xray::tools::XrayTools::new());
    let agent_tools = Arc::new(AgentTools::new(
        workspace_tools.clone(),
        snapshot_tools.clone(),
        feature_tools.clone(),
    ));

    let root = std::env::current_dir().unwrap();
    let run_tools = Arc::new(axiomregent::run_tools::RunTools::new(&root));

    Router::new(
        lease_store.clone(),
        snapshot_tools,
        workspace_tools,
        featuregraph_tools,
        xray_tools,
        agent_tools,
        run_tools,
    )
}

#[test]
fn test_features_impact() {
    let workspace = create_test_workspace();
    let router = create_router();

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "features.impact",
            "arguments": {
                "repo_root": workspace.path().to_string_lossy(),
                "paths": ["src/feature_tools.rs"]
            }
        })),
        id: Some(json!(1)),
    };

    let resp = router.handle_request(&req);
    assert!(
        resp.error.is_none(),
        "features.impact should succeed, got error: {:?}",
        resp.error
    );

    let result = resp.result.unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    let impact_json = content[0].get("json").unwrap();

    // The response should be an object with impacts, total_paths, affected_features
    assert!(
        impact_json.get("impacts").is_some(),
        "features.impact should include impacts field, got: {:?}",
        impact_json
    );
    assert!(
        impact_json.get("affected_features").is_some(),
        "features.impact should include affected_features field"
    );
}

#[test]
fn test_gov_drift() {
    let workspace = create_test_workspace();
    let router = create_router();

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "gov.drift",
            "arguments": {
                "repo_root": workspace.path().to_string_lossy()
            }
        })),
        id: Some(json!(1)),
    };

    let resp = router.handle_request(&req);
    assert!(
        resp.error.is_none(),
        "gov.drift should succeed, got error: {:?}",
        resp.error
    );

    let result = resp.result.unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    let drift_json = content[0].get("json").unwrap();

    // Response must include has_violations and violations fields
    assert!(
        drift_json.get("has_violations").is_some(),
        "gov.drift must include has_violations"
    );
    assert!(
        drift_json.get("violations").is_some(),
        "gov.drift must include violations array"
    );
}
