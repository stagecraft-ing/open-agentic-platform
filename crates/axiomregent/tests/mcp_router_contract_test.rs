// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use axiomregent::router::{JsonRpcRequest, Router};
use serde_json::json;
use std::sync::Arc;

// Feature: MCP_ROUTER_CONTRACT
// Spec: spec/core/contract.md

#[test]
fn test_router_contract_routing() {
    // Tools
    use axiomregent::snapshot::{lease::LeaseStore, tools::SnapshotTools};
    use axiomregent::workspace::WorkspaceTools;

    // db_path removed
    // let db_path = std::env::temp_dir().join("axiomregent_test_db_router");
    use tempfile;
    let dir = tempfile::tempdir().unwrap();
    let config = axiomregent::config::StorageConfig {
        data_dir: dir.path().to_path_buf(),
        blob_backend: axiomregent::config::BlobBackend::Fs,
        compression: axiomregent::config::Compression::None,
    };
    let store = Arc::new(axiomregent::snapshot::store::Store::new(config).unwrap());
    let lease_store = Arc::new(LeaseStore::new());

    let snapshot_tools = Arc::new(SnapshotTools::new(lease_store.clone(), store.clone()));
    let workspace_tools = Arc::new(WorkspaceTools::new(lease_store.clone(), store.clone()));
    let featuregraph_tools = Arc::new(axiomregent::featuregraph::tools::FeatureGraphTools::new());
    let feature_tools = Arc::new(axiomregent::feature_tools::FeatureTools::new());
    let xray_tools = Arc::new(axiomregent::xray::tools::XrayTools::new());
    let agent_tools = Arc::new(axiomregent::agent_tools::AgentTools::new(
        workspace_tools.clone(),
        snapshot_tools.clone(),
        feature_tools.clone(),
    ));
    let run_tools = Arc::new(axiomregent::run_tools::RunTools::new(dir.path()));

    let router = Router::new(
        snapshot_tools,
        workspace_tools,
        featuregraph_tools,
        xray_tools,
        agent_tools,
        run_tools,
    );

    // 1. Unknown Method -> Error -32601
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "unknown/method".to_string(),
        params: None,
        id: Some(json!(1)),
    };
    let resp = router.handle_request(&req);
    assert!(resp.error.is_some());
    let err = resp.error.unwrap();
    assert_eq!(err["code"], -32601);

    // 2. initialize -> OK
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({})),
        id: Some(json!(2)),
    };
    let resp = router.handle_request(&req);
    assert!(resp.result.is_some());
    let res = resp.result.unwrap();
    assert!(res["capabilities"].is_object());
    assert!(res["serverInfo"].is_object());
}
