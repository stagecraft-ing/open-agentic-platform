//! Spec 179 — `--domain` filter on the registry-consumer surface.
//!
//! Covers both the Value-based [`filter_features`] entry point and the
//! typed `Registry::filter` path (used by `registry-consumer list
//! --domain ...`), plus the status-report narrowing path used by
//! `registry-consumer status-report --domain ...`.

use open_agentic_spec_registry_reader::{
    FeatureFilter, Registry, filter_features, load,
};
use serde_json::json;
use std::path::PathBuf;
use tempfile::tempdir;

fn corpus() -> Vec<serde_json::Value> {
    vec![
        json!({"id": "032-cockpit", "status": "approved", "domain": "opc"}),
        json!({"id": "077-services", "status": "approved", "domain": "platform"}),
        json!({"id": "000-bootstrap", "status": "approved", "domain": "substrate"}),
        json!({"id": "001-compiler", "status": "approved", "domain": "tooling"}),
        json!({"id": "999-no-domain", "status": "approved"}),
    ]
}

fn ids(v: &[serde_json::Value]) -> Vec<&str> {
    v.iter().map(|f| f["id"].as_str().unwrap()).collect()
}

#[test]
fn value_based_filter_returns_only_matching_domain() {
    for value in ["opc", "platform", "substrate", "tooling"] {
        let out = filter_features(
            corpus(),
            FeatureFilter {
                domain: Some(value),
                ..Default::default()
            },
        );
        assert_eq!(out.len(), 1, "single match expected for domain={value:?}");
        assert!(
            out[0]["domain"].as_str() == Some(value),
            "wrong domain in filtered output"
        );
    }
}

#[test]
fn value_based_filter_excludes_features_without_domain() {
    let out = filter_features(
        corpus(),
        FeatureFilter {
            domain: Some("opc"),
            ..Default::default()
        },
    );
    assert!(ids(&out).iter().all(|id| *id != "999-no-domain"));
}

fn write_registry(path: &PathBuf, features: serde_json::Value) {
    let registry = json!({
        "specVersion": "1.5.0",
        "build": {
            "compilerId": "test",
            "compilerVersion": "0.1.0",
            "inputRoot": ".",
            "contentHash": "deadbeef"
        },
        "features": features,
        "validation": { "passed": true, "violations": [] }
    });
    std::fs::write(path, serde_json::to_string(&registry).unwrap()).unwrap();
}

#[test]
fn typed_filter_narrows_by_domain() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("r.json");
    write_registry(
        &p.to_path_buf(),
        json!([
            {"id": "032-cockpit", "status": "approved", "kind": "desktop", "domain": "opc",
             "specPath": "specs/032-cockpit/spec.md", "title": "OPC", "created": "2026-01-01"},
            {"id": "077-services", "status": "approved", "kind": "platform", "domain": "platform",
             "specPath": "specs/077-services/spec.md", "title": "Services", "created": "2026-01-01"}
        ]),
    );
    let registry: Registry = load(&p).expect("load");
    let opc = registry.filter(FeatureFilter {
        domain: Some("opc"),
        ..Default::default()
    });
    assert_eq!(opc.len(), 1);
    assert_eq!(opc[0].id, "032-cockpit");

    let platform = registry.filter(FeatureFilter {
        domain: Some("platform"),
        ..Default::default()
    });
    assert_eq!(platform.len(), 1);
    assert_eq!(platform[0].id, "077-services");
}

#[test]
fn status_report_filtered_by_domain_counts_only_matching_specs() {
    let dir = tempdir().unwrap();
    let p = dir.path().join("r.json");
    write_registry(
        &p.to_path_buf(),
        json!([
            {"id": "032-cockpit", "status": "approved", "domain": "opc",
             "specPath": "specs/032-cockpit/spec.md", "title": "OPC", "created": "2026-01-01"},
            {"id": "033-other", "status": "draft", "domain": "opc",
             "specPath": "specs/033-other/spec.md", "title": "Other", "created": "2026-01-01"},
            {"id": "077-services", "status": "approved", "domain": "platform",
             "specPath": "specs/077-services/spec.md", "title": "Services", "created": "2026-01-01"}
        ]),
    );
    let registry: Registry = load(&p).expect("load");
    let report = registry.status_report_filtered(FeatureFilter {
        domain: Some("opc"),
        ..Default::default()
    });
    let approved = report.iter().find(|(s, _, _)| s == "approved").unwrap();
    let draft = report.iter().find(|(s, _, _)| s == "draft").unwrap();
    assert_eq!(approved.1, 1, "1 opc spec approved");
    assert_eq!(draft.1, 1, "1 opc spec draft");
    assert_eq!(
        approved.2,
        vec!["032-cockpit".to_string()],
        "platform spec must not leak into opc filter"
    );
}
