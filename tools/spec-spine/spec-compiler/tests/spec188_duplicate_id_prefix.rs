//! Spec 188 Phase 2 — V-032 duplicate numeric-id prefix.
//!
//! `seen_ids` keys on the full id string, so two specs allocated the same
//! `NNN` under different slugs each pass the V-003 full-id check and compile
//! as distinct records. V-032 is the cross-corpus pass that flags any
//! leading numeric prefix claimed by 2+ specs — modelled on the real
//! spec-186 collision (`sandbox-k8s-backend` #245 vs `opc-e2e-test-harness`
//! #246) that shipped before being caught by eyeballing.

use serde_json::Value;
use std::fs;
use std::path::Path;

fn write_spec(root: &Path, dir_id: &str, fm_id: &str, title: &str) {
    let d = root.join("specs").join(dir_id);
    fs::create_dir_all(&d).unwrap();
    fs::write(
        d.join("spec.md"),
        format!(
            "---\nid: \"{fm_id}\"\ntitle: \"{title}\"\nstatus: draft\ncreated: \"2026-05-30\"\nsummary: \"fixture\"\n---\n# {title}\n"
        ),
    )
    .unwrap();
}

#[test]
fn v032_fires_on_shared_numeric_prefix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    // The real collision: two specs both allocated id 186.
    write_spec(root, "186-sandbox-k8s-backend", "186-sandbox-k8s-backend", "Sandbox K8s");
    write_spec(root, "186-opc-e2e-test-harness", "186-opc-e2e-test-harness", "OPC e2e harness");

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).expect("registry JSON");
    let viol = v["validation"]["violations"].as_array().unwrap();

    let v032: Vec<_> = viol.iter().filter(|x| x["code"] == "V-032").collect();
    assert!(
        v032.iter().all(|x| x["severity"] == "error"),
        "V-032 must be error severity, got {v032:?}"
    );
    assert_eq!(
        v032.len(),
        2,
        "both specs sharing prefix 186 must be flagged, got {viol:?}"
    );
    assert!(
        v032.iter().any(|x| x["message"].as_str().unwrap().contains("\"186\"")),
        "message must name the shared prefix, got {v032:?}"
    );
    assert_eq!(
        v["validation"]["passed"], false,
        "a duplicate numeric-id prefix must fail validation"
    );
}

#[test]
fn v032_silent_on_distinct_prefixes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    write_spec(root, "186-sandbox-k8s-backend", "186-sandbox-k8s-backend", "Sandbox K8s");
    write_spec(root, "187-opc-e2e-test-harness", "187-opc-e2e-test-harness", "OPC e2e harness");

    let out = open_agentic_spec_compiler::compile(root).expect("compile");
    let v: Value = serde_json::from_slice(&out.registry_json).expect("registry JSON");
    let viol = v["validation"]["violations"].as_array().unwrap();

    assert!(
        !viol.iter().any(|x| x["code"] == "V-032"),
        "distinct numeric prefixes must NOT trip V-032 (this is the renumber that resolved the real collision), got {viol:?}"
    );
}
