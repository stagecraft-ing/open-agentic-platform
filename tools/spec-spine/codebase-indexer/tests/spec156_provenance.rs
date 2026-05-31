//! Spec 156 — references-edge provenance grammar (indexer side).
//!
//! Tests the resolver short-circuit per spec 156 §6.3: provenance
//! entries are emitted as `ResolvedUnit` values with
//! `source_field: "references"`, `ownership: false`, `locations: []`,
//! and no resolver diagnostics. The reverse-lookup property — that
//! the `unit` JSON carries the canonical URI value `git grep`
//! returns when searching `specs/` — is also exercised.

use jsonschema::validator_for;
use serde_json::Value;
use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn install_schema(root: &Path) {
    let src_dir = repo_root().join("standards/schemas/spec-spine");
    let dst_dir = root.join("standards/schemas/spec-spine");
    fs::create_dir_all(&dst_dir).unwrap();
    // Both schemas `compile()` self-validates against: the broad index and
    // the re-homed config-hash file (spec 188 Phase 4).
    for name in ["codebase-index.schema.json", "config-hash.schema.json"] {
        fs::copy(src_dir.join(name), dst_dir.join(name)).unwrap();
    }
}

const PROJ: &str = "8c4f1234-1234-4abc-9def-1234567890ab";
const KNOWLEDGE: &str = "2a91abcd-1111-4222-a333-444555666777";
const DIGEST: &str = "5e3b00112233445566778899aabbccddeeff00112233445566778899aabbccdd";

fn write_provenance_spec(
    root: &Path,
    id: &str,
    references_yaml: &str,
) {
    let spec_dir = root.join(format!("specs/{id}"));
    fs::create_dir_all(&spec_dir).unwrap();
    let raw = format!(
        "---\nid: \"{id}\"\ntitle: \"Provenance fixture {id}\"\nstatus: draft\ncreated: \"2026-05-22\"\nsummary: \"Spec 156 indexer fixture.\"\nreferences:\n{references_yaml}---\n# {id}\n",
    );
    fs::write(spec_dir.join("spec.md"), raw).unwrap();
}

fn find_mapping<'a>(index: &'a Value, spec_id: &str) -> Option<&'a Value> {
    index["traceability"]["mappings"]
        .as_array()?
        .iter()
        .find(|m| m["specId"].as_str() == Some(spec_id))
}

fn resolved_units_for<'a>(index: &'a Value, spec_id: &str) -> &'a [Value] {
    find_mapping(index, spec_id)
        .and_then(|m| m["resolvedUnits"].as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}

/// §6.3 — knowledge provenance entry emits a ResolvedUnit with
/// the canonical URI, ownership: false, empty locations, no diagnostic.
#[test]
fn knowledge_provenance_emits_resolved_unit_short_circuit() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    install_schema(root);
    write_provenance_spec(
        root,
        "850-knowledge-provenance",
        &format!(
            "  - role: derivation\n    provenance:\n      kind: knowledge\n      ref: \"stagecraft://project/{PROJ}/knowledge/{KNOWLEDGE}\"\n",
        ),
    );

    let out = open_agentic_codebase_indexer::compile(root).expect("compile");
    let index: Value = serde_json::from_slice(&out.index_json).expect("index JSON");

    let units = resolved_units_for(&index, "850-knowledge-provenance");
    assert_eq!(units.len(), 1, "expected one resolved provenance unit: {units:?}");
    let u = &units[0];
    assert_eq!(u["kind"].as_str(), Some("knowledge"));
    assert_eq!(u["sourceField"].as_str(), Some("references"));
    assert_eq!(u["ownership"].as_bool(), Some(false));
    // Per spec 156 §6.3, empty `locations` is by design and serialised
    // as the omitted-field shape; either an empty array or missing
    // field is acceptable.
    let locs = u["locations"].as_array().map(|a| a.len()).unwrap_or(0);
    assert_eq!(locs, 0, "provenance locations must be empty (by design)");
    // Reverse-lookup substrate: the canonical URI must be present in
    // the unit JSON so `.derived/codebase-index/index.json` answers the
    // same query as `git grep` over specs/.
    assert_eq!(
        u["unit"]["ref"].as_str(),
        Some(format!("stagecraft://project/{PROJ}/knowledge/{KNOWLEDGE}").as_str())
    );
    assert_eq!(u["unit"]["kind"].as_str(), Some("knowledge"));
    // No resolver diagnostic fires for a dangling provenance.
    let resolver_diags: Vec<&Value> = index["diagnostics"]["errors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| {
            d["message"]
                .as_str()
                .map(|m| m.contains("850-knowledge-provenance"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        resolver_diags.is_empty(),
        "no diagnostic should fire for provenance entries: {resolver_diags:?}"
    );
}

/// §6.3 — code-fingerprint provenance entry emits the canonical
/// xray scheme URI in the resolved-unit JSON.
#[test]
fn code_fingerprint_provenance_emits_canonical_uri() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    install_schema(root);
    write_provenance_spec(
        root,
        "851-fingerprint-provenance",
        &format!(
            "  - role: derivation\n    provenance:\n      kind: code-fingerprint\n      ref: \"xray-fingerprint://{DIGEST}\"\n",
        ),
    );

    let out = open_agentic_codebase_indexer::compile(root).expect("compile");
    let index: Value = serde_json::from_slice(&out.index_json).expect("index JSON");

    let units = resolved_units_for(&index, "851-fingerprint-provenance");
    assert_eq!(units.len(), 1);
    let u = &units[0];
    assert_eq!(u["kind"].as_str(), Some("code-fingerprint"));
    assert_eq!(u["sourceField"].as_str(), Some("references"));
    assert_eq!(u["ownership"].as_bool(), Some(false));
    assert_eq!(
        u["unit"]["ref"].as_str(),
        Some(format!("xray-fingerprint://{DIGEST}").as_str())
    );
}

/// Mixed shape: a spec with both a unit-arm and a provenance-arm
/// references entry (across two different references items, not the
/// same one — V-025 only fires on a single entry with both arms).
/// Both flows must produce one resolved unit each, with distinct
/// `kind` values.
#[test]
fn mixed_unit_and_provenance_references_both_emit_resolved_units() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    install_schema(root);

    // Create a fixture file the unit-arm can resolve to.
    let spec_dir = root.join("specs/852-mixed-refs");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(spec_dir.join("notes.md"), "fixture\n").unwrap();

    let raw = format!(
        "---\nid: \"852-mixed-refs\"\ntitle: \"Mixed refs\"\nstatus: draft\ncreated: \"2026-05-22\"\nsummary: \"mixed shapes\"\nreferences:\n  - role: evidence\n    unit: {{ kind: file, path: specs/852-mixed-refs/notes.md }}\n  - role: derivation\n    provenance:\n      kind: knowledge\n      ref: \"stagecraft://project/{PROJ}/knowledge/{KNOWLEDGE}\"\n---\n# Mixed refs\n",
    );
    fs::write(spec_dir.join("spec.md"), raw).unwrap();

    let out = open_agentic_codebase_indexer::compile(root).expect("compile");
    let index: Value = serde_json::from_slice(&out.index_json).expect("index JSON");

    let units = resolved_units_for(&index, "852-mixed-refs");
    assert_eq!(units.len(), 2);
    let kinds: Vec<&str> = units.iter().filter_map(|u| u["kind"].as_str()).collect();
    assert!(kinds.contains(&"file"));
    assert!(kinds.contains(&"knowledge"));
    // Both entries have sourceField=references and ownership=false.
    for u in units {
        assert_eq!(u["sourceField"].as_str(), Some("references"));
        assert_eq!(u["ownership"].as_bool(), Some(false));
    }
}

/// Schema conformance: the emitted index validates against the
/// codebase-index schema with the spec 156 enum widening.
#[test]
fn provenance_index_validates_against_schema() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    install_schema(root);
    write_provenance_spec(
        root,
        "853-schema-conformance",
        &format!(
            "  - role: derivation\n    provenance:\n      kind: knowledge\n      ref: \"stagecraft://project/{PROJ}/knowledge/{KNOWLEDGE}\"\n  - role: derivation\n    provenance:\n      kind: code-fingerprint\n      ref: \"xray-fingerprint://{DIGEST}\"\n",
        ),
    );

    let out = open_agentic_codebase_indexer::compile(root).expect("compile");

    let schema_raw =
        fs::read_to_string(root.join("standards/schemas/spec-spine/codebase-index.schema.json"))
            .unwrap();
    let mut schema: Value = serde_json::from_str(&schema_raw).unwrap();
    if let Some(o) = schema.as_object_mut() {
        o.remove("$schema");
    }
    let validator = validator_for(&schema).expect("schema compiles");
    let instance: Value = serde_json::from_slice(&out.index_json).expect("index JSON");
    if let Err(e) = validator.validate(&instance) {
        panic!("index.json does not validate against spec 156 schema: {e}");
    }
}
