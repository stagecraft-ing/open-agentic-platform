// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: FEATUREGRAPH_REGISTRY

//! Load feature manifest entries from **`build/spec-registry/registry.json`**
//! (spec-compiler output) for the featuregraph scanner.
//!
//! Cut D W-05 deletes the local `CompiledRegistry` /
//! `RegistryFeatureRecord` duplicates and consumes the typed-reader
//! library introduced in W-03 (crate
//! `open_agentic_spec_registry_reader`). The thin adapters here keep
//! the typed reader as the single sanctioned site that parses
//! `registry.json` (spec 103).
//!
//! Side quest II (2026-05-20): the `load_from_registry` /
//! `ImplementsField` re-export pair was confirmed dead by audit (called
//! only by its own unit tests; no external consumer reached for it via
//! featuregraph's public API). They were removed alongside the
//! `implements:` list-form excision. The Scanner now reads spec→code
//! traceability through `index_bridge::load_from_index`
//! (`build/codebase-index/index.json`'s `implementingPaths`).

use open_agentic_spec_registry_reader as srr;
use std::path::Path;

/// Adapter alias for the typed Feature record. Featuregraph used to
/// hold its own `RegistryFeatureRecord` declaration; that duplicate is
/// gone in W-05.
pub type RegistryFeatureRecord = srr::Feature;

/// Parse `registry.json` and return feature records (sorted by id for
/// determinism). Internally delegates to `srr::load`; errors map to
/// `anyhow::Error` to keep the existing call sites unchanged.
pub fn load_registry_records(path: &Path) -> anyhow::Result<Vec<srr::Feature>> {
    let registry = srr::load(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut features = registry.features;
    features.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(features)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    /// Helper: emit a `specVersion: "1.5.0"` fixture. The typed reader
    /// dispatches on the 1.x family; pre-W-05 fixtures lacking the
    /// field decoded under featuregraph's own permissive parser and
    /// now need the explicit version. (Same fixture corpus
    /// semantics — only the schema-version is made explicit.)
    fn write_fixture(path: &std::path::PathBuf, body: &str) {
        std::fs::File::create(path)
            .unwrap()
            .write_all(body.as_bytes())
            .unwrap();
    }

    #[test]
    fn parses_minimal_registry_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        write_fixture(
            &path,
            r#"{
                "specVersion": "1.5.0",
                "features": [
                    {
                        "id": "002-registry-consumer-mvp",
                        "title": "Registry consumer MVP",
                        "specPath": "specs/002-registry-consumer-mvp/spec.md",
                        "status": "draft",
                        "summary": "x",
                        "kind": "platform",
                        "created": "2026-03-22",
                        "authors": ["open-agentic-platform"]
                    }
                ],
                "validation": { "passed": true, "violations": [] }
            }"#,
        );

        let records = load_registry_records(&path).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "002-registry-consumer-mvp");
        assert_eq!(
            records[0].spec_path.as_deref(),
            Some("specs/002-registry-consumer-mvp/spec.md")
        );
        assert_eq!(records[0].status.as_deref(), Some("draft"));
    }

    #[test]
    fn sorts_by_id() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        write_fixture(
            &path,
            r#"{
                "specVersion": "1.5.0",
                "features": [
                    {"id":"b","title":"","specPath":"specs/b/spec.md","status":"draft"},
                    {"id":"a","title":"","specPath":"specs/a/spec.md","status":"draft"}
                ],
                "validation": { "passed": true, "violations": [] }
            }"#,
        );

        let records = load_registry_records(&path).unwrap();
        assert_eq!(records[0].id, "a");
        assert_eq!(records[1].id, "b");
    }

    #[test]
    fn parses_code_aliases_from_registry() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        write_fixture(
            &path,
            r#"{
                "specVersion": "1.5.0",
                "features": [
                    {
                        "id": "034-featuregraph-registry-scanner-fix",
                        "title": "t",
                        "specPath": "specs/034-featuregraph-registry-scanner-fix/spec.md",
                        "status": "active",
                        "summary": "x",
                        "created": "2026-03-29",
                        "sectionHeadings": [],
                        "codeAliases": ["FEATUREGRAPH_REGISTRY", "GOVERNANCE_ENGINE"]
                    }
                ],
                "validation": { "passed": true, "violations": [] }
            }"#,
        );

        let records = load_registry_records(&path).unwrap();
        assert_eq!(
            records[0].code_aliases,
            vec![
                "FEATUREGRAPH_REGISTRY".to_string(),
                "GOVERNANCE_ENGINE".to_string()
            ]
        );
    }

    #[test]
    fn sc091_1_parses_depends_on() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        write_fixture(
            &path,
            r#"{
                "specVersion": "1.5.0",
                "features": [
                    {
                        "id": "091-test",
                        "title": "t",
                        "specPath": "specs/091/spec.md",
                        "status": "active",
                        "dependsOn": ["dep-a", "dep-b"]
                    }
                ],
                "validation": { "passed": true, "violations": [] }
            }"#,
        );

        let records = load_registry_records(&path).unwrap();
        assert_eq!(records[0].depends_on, vec!["dep-a", "dep-b"]);
    }

    #[test]
    fn sc091_1_parses_owner() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        write_fixture(
            &path,
            r#"{
                "specVersion": "1.5.0",
                "features": [
                    {
                        "id": "091-owner",
                        "title": "t",
                        "specPath": "specs/091/spec.md",
                        "status": "active",
                        "owner": "platform-team"
                    }
                ],
                "validation": { "passed": true, "violations": [] }
            }"#,
        );

        let records = load_registry_records(&path).unwrap();
        assert_eq!(records[0].owner.as_deref(), Some("platform-team"));
    }

    #[test]
    fn sc091_1_parses_risk() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        write_fixture(
            &path,
            r#"{
                "specVersion": "1.5.0",
                "features": [
                    {
                        "id": "091-risk",
                        "title": "t",
                        "specPath": "specs/091/spec.md",
                        "status": "active",
                        "risk": "high"
                    }
                ],
                "validation": { "passed": true, "violations": [] }
            }"#,
        );

        let records = load_registry_records(&path).unwrap();
        assert_eq!(records[0].risk.as_deref(), Some("high"));
    }

    #[test]
    fn sc091_2_missing_enriched_fields_default() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        write_fixture(
            &path,
            r#"{
                "specVersion": "1.5.0",
                "features": [
                    {
                        "id": "091-minimal",
                        "title": "t",
                        "specPath": "specs/091/spec.md",
                        "status": "draft"
                    }
                ],
                "validation": { "passed": true, "violations": [] }
            }"#,
        );

        let records = load_registry_records(&path).unwrap();
        assert!(records[0].depends_on.is_empty());
        assert!(records[0].owner.is_none());
        assert!(records[0].risk.is_none());
    }

    #[test]
    fn parses_implements_scalar_form() {
        // Spec 147 capability proving-ground: `implements: "<spec-id>"`
        // is the registry-pointer form, retained after side-quest-II's
        // excision of the legacy path-claiming list form.
        let dir = tempdir().unwrap();
        let path = dir.path().join("registry.json");
        write_fixture(
            &path,
            r#"{
                "specVersion": "1.5.0",
                "features": [
                    {
                        "id": "149-saml-auth-driver",
                        "title": "t",
                        "specPath": "specs/149/spec.md",
                        "status": "draft",
                        "implements": "148-auth-driver-registry"
                    }
                ],
                "validation": { "passed": true, "violations": [] }
            }"#,
        );
        let records = load_registry_records(&path).unwrap();
        let imp = records[0].implements.as_ref().expect("implements present");
        assert_eq!(imp.as_scalar(), Some("148-auth-driver-registry"));
        assert!(imp.paths().is_empty(), "scalar form carries no code paths");
    }
}
