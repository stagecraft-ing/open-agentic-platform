// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use axiomregent::agent_tools::AgentTools;
use axiomregent::feature_tools::FeatureTools;

use axiomregent::router::JsonRpcRequest;
use axiomregent::router::Router;
use axiomregent::snapshot::tools::SnapshotTools;
use axiomregent::workspace::WorkspaceTools;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

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
    let router = create_router();
    let repo_root: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "features.impact",
            "arguments": {
                "repo_root": repo_root.to_string_lossy(),
                "paths": ["src/feature_tools.rs"]
            }
        })),
        id: Some(json!(1)),
    };

    let resp = router.handle_request(&req);
    assert!(resp.error.is_none(), "features.impact should succeed");

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
    let router = create_router();
    let repo_root: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "gov.drift",
            "arguments": {
                "repo_root": repo_root.to_string_lossy()
            }
        })),
        id: Some(json!(1)),
    };

    let resp = router.handle_request(&req);
    assert!(resp.error.is_none(), "gov.drift should succeed");

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
