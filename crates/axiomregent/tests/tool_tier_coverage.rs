// Feature 036 — safety tier governance
// Asserts that every tool in the router's tools/list has an explicit tier assignment.
// Adding a tool to the router without classifying it in get_tool_tier() will fail this test.

use axiomregent::router::{JsonRpcRequest, Router};
use axiomregent::snapshot::lease::LeaseStore;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

fn create_router() -> Router {
    let lease_store = Arc::new(LeaseStore::new());
    let storage_config = axiomregent::config::StorageConfig::default();
    let store = Arc::new(axiomregent::snapshot::store::Store::new(storage_config).unwrap());

    let snapshot_tools = Arc::new(axiomregent::snapshot::tools::SnapshotTools::new(
        lease_store.clone(),
        store.clone(),
    ));
    let workspace_tools = Arc::new(axiomregent::workspace::WorkspaceTools::new(
        lease_store.clone(),
        store.clone(),
    ));
    let featuregraph_tools = Arc::new(axiomregent::featuregraph::tools::FeatureGraphTools::new());
    let feature_tools = Arc::new(axiomregent::feature_tools::FeatureTools::new());
    let xray_tools = Arc::new(axiomregent::xray::tools::XrayTools::new());
    let agent_tools = Arc::new(axiomregent::agent_tools::AgentTools::new(
        workspace_tools.clone(),
        snapshot_tools.clone(),
        feature_tools.clone(),
    ));
    let root = std::env::current_dir().unwrap();
    let run_tools = Arc::new(axiomregent::run_tools::RunTools::new(&root));

    Router::new(
        lease_store,
        snapshot_tools,
        workspace_tools,
        featuregraph_tools,
        xray_tools,
        agent_tools,
        run_tools,
    )
}

#[test]
fn every_router_tool_has_explicit_tier() {
    let router = create_router();

    // Get tools/list response
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: None,
        id: Some(json!(1)),
    };
    let resp = router.handle_request(&req);
    let result = resp.result.expect("tools/list should return result");
    let tools = result["tools"].as_array().expect("tools should be an array");

    let router_tool_names: HashSet<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("tool name should be string"))
        .collect();

    let classified: HashSet<&str> = agent::safety::explicitly_classified_tools()
        .iter()
        .copied()
        .collect();

    // Every router tool must be in the explicitly classified set
    let unclassified: Vec<&&str> = router_tool_names
        .iter()
        .filter(|name| !classified.contains(**name))
        .collect();

    assert!(
        unclassified.is_empty(),
        "FR-002 FAIL: The following router tools have no explicit tier assignment \
         (they fall through to the Tier3 catch-all). Add them to get_tool_tier() \
         in crates/agent/src/safety.rs:\n  {:?}",
        unclassified
    );

    // Sanity: classified set should cover all router tools
    eprintln!(
        "Tool tier coverage: {}/{} router tools explicitly classified",
        router_tool_names.len(),
        router_tool_names.len()
    );
}

#[test]
fn explicitly_classified_tools_matches_router() {
    let router = create_router();

    // Get tools/list response
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: None,
        id: Some(json!(1)),
    };
    let resp = router.handle_request(&req);
    let result = resp.result.expect("tools/list should return result");
    let tools = result["tools"].as_array().expect("tools should be an array");

    let router_tool_names: HashSet<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("tool name should be string"))
        .collect();

    let classified: HashSet<&str> = agent::safety::explicitly_classified_tools()
        .iter()
        .copied()
        .collect();

    // Every explicitly classified tool MUST exist in the router,
    // EXCEPT "write_file", which is a legacy alias kept for internal use.
    let missing_in_router: Vec<&&str> = classified
        .iter()
        .filter(|&&name| name != "write_file" && !router_tool_names.contains(name))
        .collect();

    assert!(
        missing_in_router.is_empty(),
        "The following tools are listed in explicitly_classified_tools() but DO NOT exist in the router:\n  {:?}",
        missing_in_router
    );
}

#[test]
fn tier_assignments_match_spec() {
    // Verify specific tier assignments from the 036 spec
    use agent::safety::{ToolTier, get_tool_tier};

    // Tier 1: read-only / diagnostic
    for tool in &[
        "gov.preflight", "gov.drift", "features.impact", "snapshot.info",
        "snapshot.list", "snapshot.read", "snapshot.grep", "snapshot.diff",
        "snapshot.changes", "snapshot.export", "xray.scan",
        "run.status", "run.logs", "agent.verify",
    ] {
        assert_eq!(
            get_tool_tier(tool), ToolTier::Tier1,
            "{tool} should be Tier1"
        );
    }

    // Tier 2: bounded mutations
    for tool in &[
        "workspace.apply_patch", "workspace.write_file", "workspace.delete",
        "snapshot.create", "agent.propose",
    ] {
        assert_eq!(
            get_tool_tier(tool), ToolTier::Tier2,
            "{tool} should be Tier2"
        );
    }

    // Tier 3: dangerous / execution
    for tool in &["run.execute", "agent.execute"] {
        assert_eq!(
            get_tool_tier(tool), ToolTier::Tier3,
            "{tool} should be Tier3"
        );
    }

    // Unknown tools should also be Tier3
    assert_eq!(get_tool_tier("unknown.tool"), ToolTier::Tier3);
}
