// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use axiomregent::router::{JsonRpcRequest, Router};
use axiomregent::snapshot::{lease::LeaseStore, tools::SnapshotTools};
use axiomregent::workspace::WorkspaceTools;
use serde_json::json;
use std::sync::Arc;

// Feature: MCP_TOOLS
// Spec: spec/core/tools.md

fn make_router(dir: &std::path::Path) -> Router {
    let config = axiomregent::config::StorageConfig {
        data_dir: dir.to_path_buf(),
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
    let run_tools = Arc::new(axiomregent::run_tools::RunTools::new(dir));

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
fn test_mcp_tools_list() {
    let dir = tempfile::tempdir().unwrap();
    let router = make_router(dir.path());

    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: None,
        id: Some(json!(1)),
    };
    let resp = router.handle_request(&req);
    assert!(resp.result.is_some());
    let res = resp.result.unwrap();

    let tools = res["tools"].as_array().expect("tools should be an array");

    // Check for core tools that are kept in the trimmed server
    let required_tools = vec!["agent.propose", "features.impact", "gov.preflight", "snapshot.create"];
    for req_tool in required_tools {
        let found = tools.iter().any(|t| t["name"] == req_tool);
        assert!(found, "Tool {} not found in tools list", req_tool);
    }

    // Verify removed tools are absent
    let removed_tools = vec!["resolve_mcp", "list_mounts", "get_capabilities", "features.overview", "features.locate"];
    for removed in removed_tools {
        let found = tools.iter().any(|t| t["name"] == removed);
        assert!(!found, "Tool {} should have been removed", removed);
    }
}

#[test]
fn test_mcp_tools_call_validation() {
    let dir = tempfile::tempdir().unwrap();
    let router = make_router(dir.path());

    // Call an unknown tool — expect error
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "nonexistent_tool",
            "arguments": {}
        })),
        id: Some(json!(2)),
    };
    let resp = router.handle_request(&req);
    assert!(resp.error.is_some(), "Calling unknown tool should return an error");

    // Call features.impact without required repo_root -> error
    let req2 = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "features.impact",
            "arguments": {}
        })),
        id: Some(json!(3)),
    };
    let resp2 = router.handle_request(&req2);
    assert!(resp2.error.is_some(), "Missing repo_root should return an error");
}
