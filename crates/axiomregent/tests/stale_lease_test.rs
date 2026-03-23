// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use axiomregent::agent_tools::AgentTools;
use axiomregent::feature_tools::FeatureTools;
use axiomregent::router::{JsonRpcRequest, Router};
use axiomregent::snapshot::{lease::LeaseStore, tools::SnapshotTools};
use axiomregent::workspace::WorkspaceTools;
use serde_json::json;
use std::process::Command;
use std::sync::Arc;
use tempfile::TempDir;

fn setup_repo() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Ignore output to avoid clogging test logs
    let _ = Command::new("git")
        .arg("init")
        .current_dir(root)
        .output()
        .unwrap();
    let _ = Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(root)
        .output()
        .unwrap();
    let _ = Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Test")
        .current_dir(root)
        .output()
        .unwrap();

    // Create initial commit
    std::fs::write(root.join("file.txt"), "initial").unwrap();
    // Use proper git commands
    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output()
        .unwrap();
    let _ = Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(root)
        .output()
        .unwrap();

    dir
}

#[test]
fn test_stale_lease_error_structure() {
    let repo = setup_repo();
    let repo_path = repo.path().to_str().unwrap();

    let db_dir = tempfile::tempdir().unwrap();
    let config = axiomregent::config::StorageConfig {
        data_dir: db_dir.path().to_path_buf(),
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
    let run_tools = Arc::new(axiomregent::run_tools::RunTools::new(repo.path()));

    let router = Router::new(
        snapshot_tools,
        workspace_tools,
        featuregraph_tools,
        xray_tools,
        agent_tools,
        run_tools,
    );

    // 1. Get a lease via snapshot.list (worktree mode)
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "snapshot.list",
            "arguments": {
                "repo_root": repo_path,
                "path": ".",
                "mode": "worktree"
            }
        })),
        id: Some(json!(1)),
    };
    let resp = router.handle_request(&req);

    // Debug output if fails
    if let Some(err) = &resp.error {
        println!("Initial request failed: {:?}", err);
    }

    assert!(resp.result.is_some(), "Initial request failed");
    let res = resp.result.unwrap();
    let content = &res["content"][0]["json"];

    // Worktree mode returns lease_id at top level of result?
    // Check snapshot.list response schema.
    // It returns { "entries": [...], "lease_id": "...", "fingerprint": ... }

    let lease_id_val = content.get("lease_id");
    if lease_id_val.is_none() {
        println!("Response content: {:?}", content);
        panic!("lease_id missing from response");
    }
    let lease_id = lease_id_val
        .unwrap()
        .as_str()
        .expect("lease_id strings")
        .to_string();

    // 2. Modify repo (commit a change to change HEAD oid)
    std::fs::write(repo.path().join("new_file.txt"), "modified").unwrap();
    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(repo.path())
        .output()
        .unwrap();
    let _ = Command::new("git")
        .args(["commit", "-m", "update"])
        .current_dir(repo.path())
        .output()
        .unwrap();

    // 3. Call snapshot.list with old lease
    let req2 = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "snapshot.list",
            "arguments": {
                "repo_root": repo_path,
                "path": ".",
                "mode": "worktree",
                "lease_id": lease_id
            }
        })),
        id: Some(json!(2)),
    };
    let resp2 = router.handle_request(&req2);

    // 4. Expect Error
    let err = resp2.error.expect("Should return error for stale lease");
    println!("Error details: {:?}", err);

    assert_eq!(err["code"], "STALE_LEASE");
    assert_eq!(err["message"], "Lease is stale (repo changed)");
    let data = &err["data"];
    assert!(data["current_fingerprint"].is_object());
    assert!(data["current_fingerprint"]["head_oid"].is_string());
    assert_eq!(data["lease_id"], lease_id);
}
