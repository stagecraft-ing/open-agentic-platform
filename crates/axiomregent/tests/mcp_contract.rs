// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use axiomregent::agent_tools::AgentTools;
use axiomregent::feature_tools::FeatureTools;
use axiomregent::router::{JsonRpcRequest, Router};
use axiomregent::snapshot::{lease::LeaseStore, tools::SnapshotTools};
use axiomregent::workspace::WorkspaceTools;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn test_mcp_tools_list_contract() {
    // 1. Setup minimal harness
    let dir = tempdir().unwrap();
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
    let feature_tools = Arc::new(FeatureTools::new());
    let xray_tools = Arc::new(axiomregent::xray::tools::XrayTools::new());
    let agent_tools = Arc::new(AgentTools::new(
        workspace_tools.clone(),
        snapshot_tools.clone(),
        feature_tools.clone(),
    ));
    let run_tools = Arc::new(axiomregent::run_tools::RunTools::new(dir.path()));

    let router = Router::new(
        lease_store.clone(),
        snapshot_tools,
        workspace_tools,
        featuregraph_tools,
        xray_tools,
        agent_tools,
        run_tools,
    );

    // 2. Call tools/list
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: None,
        id: Some(json!(1)),
    };
    let resp = router.handle_request(&req);

    assert!(resp.error.is_none());
    let result = resp.result.unwrap();

    // 3. Serialize and save/compare (Golden test)
    let actual_json = serde_json::to_string_pretty(&result).unwrap();

    let golden_path = PathBuf::from("tests/golden/tools_list.json");

    // If UPDATE_GOLDEN env var is set, update the golden file
    if std::env::var("UPDATE_GOLDEN").is_ok() {
        std::fs::write(&golden_path, &actual_json).unwrap();
    }

    // If golden file exists, compare
    if golden_path.exists() {
        let expected = std::fs::read_to_string(&golden_path).unwrap();
        // Normalize line endings and whitespace if needed, but pretty print should be consistent.
        let expected = expected.replace("\r\n", "\n");
        let actual = actual_json.replace("\r\n", "\n");

        // Simple string comparison for now.
        // If this is flaky due to JSON object key ordering (serde preserves order for maps if "preserve_order" feature is on, but `json!` macro behavior varies),
        // we might rely on the fact that `tools` is a list and individual tool props are small.
        // `tools` array order might differ if not sorted?
        // The router implementation hardcodes the list in order, so it should be deterministic.

        assert_eq!(
            expected, actual,
            "Protocol contract mismatch! Run with UPDATE_GOLDEN=1 to update."
        );
    } else {
        // First run initialization
        std::fs::write(&golden_path, &actual_json).unwrap();
    }
}
