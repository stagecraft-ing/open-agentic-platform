//! Spec frontmatter reader (Layer 2 input).

use open_agentic_spec_types::{LogicalUnit, split_frontmatter_required};
use std::fs;
use std::path::{Path, PathBuf};

/// A spec record extracted from frontmatter.
pub struct SpecRecord {
    pub id: String,
    pub status: String,
    pub implementation: Option<String>,
    pub depends_on: Vec<String>,
    pub implements: Vec<ImplementsEntry>,
    /// Spec 133: raw `amends:` list from frontmatter. Entries may be
    /// short-form (`"000"`) or full (`"000-bootstrap-spec-system"`);
    /// `xref::build_traceability` resolves to full ids.
    pub amends: Vec<String>,
    /// Spec 133: raw `amendment_record:` value from frontmatter (single
    /// id today; resolved to a full id by `xref::build_traceability`).
    pub amendment_record: Option<String>,
    /// Spec 154 Segment 3: logical-unit declarations harvested from
    /// every relationship-graph field, tagged with which field carried
    /// each entry. Bare strings parse to `LogicalUnit::File` per spec
    /// 154 §3.6's legacy-form affordance. Drives the codebase-indexer
    /// resolver pass; orthogonal to `implements` (which feeds the
    /// path-list traceability layer).
    pub units: Vec<UnitEntry>,
}

/// A logical-unit declaration with the relationship field that carried
/// it. `ownership` discriminates the seven ownership-bearing fields
/// from `references` (declaratively non-owning per spec 154).
///
/// `was_explicit` carries the surface-syntax distinction the
/// spec-compiler's V-023 already encodes: `true` when the unit was
/// authored as `{ kind: file, path: ... }` (or any other explicit
/// `{ kind: ... }` mapping); `false` for the legacy bare-string form
/// that parses to `LogicalUnit::File` via the spec 154 §3.6 compat
/// affordance. The resolver routes diagnostic severity off this bit
/// — explicit MissingFile is a blocking I-008 (mirroring V-023);
/// bare-string MissingFile is a non-blocking I-108 warning during
/// the compat window, lifted to I-008 by Segment 6's explicit-only
/// flip. Lives indexer-side rather than on `LogicalUnit` because the
/// unit *shape* per spec 154 §3.6 doesn't depend on authoring; only
/// the compat severity does.
pub struct UnitEntry {
    pub unit: LogicalUnit,
    pub source_field: &'static str,
    pub ownership: bool,
    pub was_explicit: bool,
}

/// A single entry from the `implements` frontmatter field.
pub struct ImplementsEntry {
    pub crate_name: Option<String>,
    pub path: String,
    /// Spec 147 — optional `primary: true` flag per implements item.
    /// `None` when absent (the corpus default for all unannotated
    /// items). The codebase-index surfaces this in `ImplementingPath.primary`
    /// so downstream consumers (coupling gate, spec/code traceability)
    /// can choose between corpus-wide primary ownership (when set) and
    /// the any-one-claimant heuristic (when absent).
    pub primary: Option<bool>,
}

/// Scan all `specs/*/spec.md` files and extract frontmatter.
pub fn scan_specs(repo_root: &Path) -> Vec<SpecRecord> {
    let specs_dir = repo_root.join("specs");
    if !specs_dir.is_dir() {
        return vec![];
    }

    let mut records = Vec::new();
    let mut entries: Vec<PathBuf> = Vec::new();

    if let Ok(dir) = fs::read_dir(&specs_dir) {
        for ent in dir.flatten() {
            let p = ent.path();
            if !p.is_dir() {
                continue;
            }
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !is_spec_dir(name) {
                continue;
            }
            let spec_md = p.join("spec.md");
            if spec_md.is_file() {
                entries.push(spec_md);
            }
        }
    }
    entries.sort();

    // Spec 154 §3.1: build the manifest-name → workspace-member-path
    // map once per scan. Used by `parse_implements` to expand `crate:`
    // unit declarations into the same path-list traceability the
    // legacy bare-path declarations contributed.
    let workspace_members = collect_workspace_members(repo_root);

    for spec_path in &entries {
        if let Some(rec) = parse_spec(spec_path, &workspace_members) {
            records.push(rec);
        }
    }

    records
}

/// Map crate `id` (Rust `[package].name` or npm `package.json:name`)
/// → workspace-relative directory. Mirrors spec-compiler's
/// `discover_workspace_crate_ids` shape (spec 154 §3.1 — workspace
/// boundary is the manifest, not the language).
fn collect_workspace_members(
    repo_root: &Path,
) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    // Rust workspace members.
    let root_manifest = repo_root.join("Cargo.toml");
    if let Ok(raw) = fs::read_to_string(&root_manifest) {
        if let Ok(parsed) = raw.parse::<toml::Value>() {
            if let Some(members) = parsed
                .get("workspace")
                .and_then(|w| w.get("members"))
                .and_then(|m| m.as_array())
            {
                for member in members {
                    let Some(rel) = member.as_str() else {
                        continue;
                    };
                    let member_manifest = repo_root.join(rel).join("Cargo.toml");
                    let Ok(member_raw) = fs::read_to_string(&member_manifest) else {
                        continue;
                    };
                    let Ok(member_parsed) = member_raw.parse::<toml::Value>() else {
                        continue;
                    };
                    if let Some(name) = member_parsed
                        .get("package")
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                    {
                        out.insert(name.to_string(), rel.to_string());
                    }
                }
            }
        }
    }
    // npm packages under product/.
    for pkg_root in ["product/apps", "product/packages"] {
        let dir = repo_root.join(pkg_root);
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let mut sorted: Vec<_> = entries.flatten().collect();
        sorted.sort_by_key(|e| e.file_name());
        for entry in sorted {
            let pkg_json = entry.path().join("package.json");
            let Ok(raw) = fs::read_to_string(&pkg_json) else {
                continue;
            };
            let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) else {
                continue;
            };
            if let Some(name) = parsed.get("name").and_then(|n| n.as_str()) {
                if let Ok(rel) = entry.path().strip_prefix(repo_root) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    out.insert(name.to_string(), rel_str);
                }
            }
        }
    }
    out
}

fn is_spec_dir(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() >= 5 && b[..3].iter().all(|u| u.is_ascii_digit()) && b[3] == b'-'
}

fn parse_spec(
    path: &Path,
    workspace_members: &std::collections::BTreeMap<String, String>,
) -> Option<SpecRecord> {
    let raw = fs::read_to_string(path).ok()?;
    let (yaml_val, _body) = split_frontmatter_required(&raw).ok()?;
    let fm = yaml_val.as_mapping()?;

    let id = fm.get("id")?.as_str()?.to_string();
    let status = fm.get("status")?.as_str()?.to_string();
    let implementation = fm
        .get("implementation")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let depends_on = parse_depends_on(fm);
    let implements = parse_implements(fm, workspace_members);
    let amends = parse_string_list(fm, "amends");
    let amendment_record = fm
        .get("amendment_record")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let units = parse_units(fm);

    Some(SpecRecord {
        id,
        status,
        implementation,
        depends_on,
        implements,
        amends,
        amendment_record,
        units,
    })
}

/// Parse a list-of-strings frontmatter field (`amends`, `depends_on`-like).
/// Returns the entries in declaration order; resolution to full spec ids
/// happens later in `xref::build_traceability`.
fn parse_string_list(fm: &serde_yaml::Mapping, key: &str) -> Vec<String> {
    let Some(val) = fm.get(key) else {
        return vec![];
    };
    let Some(seq) = val.as_sequence() else {
        return vec![];
    };
    seq.iter()
        .filter_map(|item| item.as_str().map(|s| s.to_string()))
        .collect()
}

/// Parse the `depends_on` field from raw YAML frontmatter.
/// Returns a sorted list of spec IDs (string values from the YAML sequence).
fn parse_depends_on(fm: &serde_yaml::Mapping) -> Vec<String> {
    let Some(val) = fm.get("depends_on") else {
        return vec![];
    };
    let Some(seq) = val.as_sequence() else {
        return vec![];
    };

    let mut ids: Vec<String> = seq
        .iter()
        .filter_map(|item| item.as_str().map(|s| s.to_string()))
        .collect();
    ids.sort();
    ids
}

/// Parse the relationship-graph fields (`establishes`, `extends`,
/// `refines`, `co_authority`) from raw YAML frontmatter and emit the
/// union of code paths as `ImplementsEntry` values.
///
/// Spec 130 + side-quest-II: the legacy `implements:` list-form is
/// excised from the corpus. The scalar form (`implements: "<spec-id>"`,
/// capability proving-ground per spec 147) carries no code paths and
/// is intentionally not read here. The indexer reads relationship
/// fields directly because the spec-compiler's extraFrontmatter rejects
/// nested mappings (V-002); the indexer has its own read path.
fn parse_implements(
    fm: &serde_yaml::Mapping,
    workspace_members: &std::collections::BTreeMap<String, String>,
) -> Vec<ImplementsEntry> {
    let mut entries = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    // Spec 130 relationship-graph derivation. Each helper extracts
    // paths from a specific field and pushes any new ones into the
    // entry list with no crate hint and no primary flag.
    let mut push_path = |path: String| {
        if seen.insert(path.clone()) {
            entries.push(ImplementsEntry {
                crate_name: None,
                path,
                primary: None,
            });
        }
    };

    // Spec 154 §3 typed-unit extractor: walks a `unit:` mapping and
    // emits the resolved path. `crate:` maps to its workspace-member
    // directory via `workspace_members`. `symbol:` / `module:` carry
    // sub-file-level identity that does not map to a flat path; the
    // resolver's symbol/module index is the authority for those and
    // the gate consumes them via `resolved_units`, not via the
    // path-list traceability.
    let unit_path = |unit: &serde_yaml::Value| -> Option<String> {
        let map = unit.as_mapping()?;
        let kind = map
            .get(serde_yaml::Value::String("kind".to_string()))?
            .as_str()?;
        match kind {
            "crate" => {
                let id = map
                    .get(serde_yaml::Value::String("id".to_string()))?
                    .as_str()?;
                workspace_members.get(id).cloned()
            }
            "directory" | "file" => map
                .get(serde_yaml::Value::String("path".to_string()))?
                .as_str()
                .map(|s| s.to_string()),
            "section" => map
                .get(serde_yaml::Value::String("file".to_string()))?
                .as_str()
                .map(|s| s.to_string()),
            _ => None,
        }
    };

    // `establishes:` — flat list. Items can be bare strings (legacy
    // `file:` form), `{ unit: <unit> }`, or a direct unit mapping (per
    // spec 154 §5 the canonical form is `{ unit: <unit> }`).
    if let Some(val) = fm.get("establishes") {
        if let Some(seq) = val.as_sequence() {
            for item in seq {
                if let Some(s) = item.as_str() {
                    push_path(s.to_string());
                    continue;
                }
                if let Some(mapping) = item.as_mapping() {
                    if let Some(unit_val) = mapping.get("unit") {
                        if let Some(p) = unit_path(unit_val) {
                            push_path(p);
                        }
                    }
                }
            }
        }
    }

    // Extract paths from each item in a structured field (`extends`,
    // `refines`, `co_authority`, `constrains`, `supersedes`, `amends`).
    // Items may carry legacy `paths: [...]` (string list) OR typed
    // `unit: <unit>`; both sources contribute to the traceability
    // path list.
    let extract_paths_from = |fm: &serde_yaml::Mapping,
                              key: &str,
                              push: &mut dyn FnMut(String)| {
        if let Some(val) = fm.get(key) {
            if let Some(seq) = val.as_sequence() {
                for item in seq {
                    if let Some(mapping) = item.as_mapping() {
                        if let Some(paths_val) = mapping.get("paths") {
                            if let Some(paths_seq) = paths_val.as_sequence() {
                                for p in paths_seq {
                                    if let Some(s) = p.as_str() {
                                        push(s.to_string());
                                    }
                                }
                            }
                        }
                        if let Some(unit_val) = mapping.get("unit") {
                            if let Some(p) = unit_path(unit_val) {
                                push(p);
                            }
                        }
                    }
                }
            }
        }
    };

    for field in &["extends", "refines", "supersedes", "amends", "co_authority", "constrains"] {
        extract_paths_from(fm, field, &mut push_path);
    }

    entries
}

/// Spec 154 Segment 3: harvest logical-unit declarations from every
/// relationship-graph field. Each entry is paired with the field name
/// that carried it and an `ownership` flag (`false` only for
/// `references`). Parse failures are silently dropped — the
/// spec-compiler is the authoritative validator of unit grammar
/// (V-021..V-024); this indexer-side reader stays permissive and only
/// consumes what parses cleanly.
fn parse_units(fm: &serde_yaml::Mapping) -> Vec<UnitEntry> {
    let mut out = Vec::new();

    // Flat fields: each item is a unit (string, mapping with `kind:`,
    // or — only for `references` — a mapping with a `unit:` key).
    push_flat_units(fm, "establishes", true, &mut out);
    push_flat_units(fm, "references", false, &mut out);

    // Structured fields: each item is a relationship-edge mapping with
    // a nested `paths: [...]` (string list) and/or `unit: <unit>` /
    // `units: [<unit>]` member that carries the logical-unit forms.
    for field in &["extends", "refines", "supersedes", "amends", "co_authority", "constrains"] {
        let ownership = true;
        push_structured_units(fm, field, ownership, &mut out);
    }

    out
}

/// Walk a flat list field (each entry is itself a unit declaration).
fn push_flat_units(
    fm: &serde_yaml::Mapping,
    field: &'static str,
    ownership: bool,
    out: &mut Vec<UnitEntry>,
) {
    let Some(seq) = fm.get(field).and_then(|v| v.as_sequence()) else {
        return;
    };
    for item in seq {
        // `references` entries can take the role-tagged form
        // (`- role: <r>\n  unit: <u>`); the resolver only cares about
        // `unit:`. Detect by presence of the `unit:` key. The
        // role-tagged form's compat severity is governed by the
        // *inner* `unit:` value's shape (mapping → explicit;
        // string → bare), not the outer wrapper.
        if let Some(m) = item.as_mapping() {
            if let Some(unit_val) = m.get("unit") {
                let was_explicit = is_explicit_unit_shape(unit_val);
                if let Ok(u) = LogicalUnit::from_yaml(unit_val) {
                    out.push(UnitEntry {
                        unit: u,
                        source_field: field,
                        ownership,
                        was_explicit,
                    });
                    continue;
                }
            }
        }
        let was_explicit = is_explicit_unit_shape(item);
        if let Ok(u) = LogicalUnit::from_yaml(item) {
            out.push(UnitEntry {
                unit: u,
                source_field: field,
                ownership,
                was_explicit,
            });
        }
    }
}

/// Walk a structured relationship field. Each item is a mapping like:
/// ```yaml
/// extends:
///   - spec: 130
///     paths: [<string>, ...]    # legacy path-list form
///     unit: <unit>              # spec 154 unit form (singular)
///     units: [<unit>, ...]      # spec 154 unit form (plural)
/// ```
/// We harvest from `paths` (each path → File unit via legacy parsing),
/// `unit` (singular), and `units` (plural). Other keys (`spec`,
/// `nature`, `aspect`, etc.) are ignored — they are relationship-graph
/// metadata, not ownership claims.
fn push_structured_units(
    fm: &serde_yaml::Mapping,
    field: &'static str,
    ownership: bool,
    out: &mut Vec<UnitEntry>,
) {
    let Some(seq) = fm.get(field).and_then(|v| v.as_sequence()) else {
        return;
    };
    for item in seq {
        let Some(m) = item.as_mapping() else {
            continue;
        };
        // `paths:` entries are always bare strings (the legacy
        // path-list authoring form). The compat-window severity
        // applies — `was_explicit = false`.
        if let Some(paths) = m.get("paths").and_then(|v| v.as_sequence()) {
            for p in paths {
                let was_explicit = is_explicit_unit_shape(p);
                if let Ok(u) = LogicalUnit::from_yaml(p) {
                    out.push(UnitEntry {
                        unit: u,
                        source_field: field,
                        ownership,
                        was_explicit,
                    });
                }
            }
        }
        // `unit:` / `units:` carry the explicit spec 154 form
        // (mapping shape with `kind:`). `was_explicit` follows the
        // value's surface shape, not the wrapper.
        if let Some(unit_val) = m.get("unit") {
            let was_explicit = is_explicit_unit_shape(unit_val);
            if let Ok(u) = LogicalUnit::from_yaml(unit_val) {
                out.push(UnitEntry {
                    unit: u,
                    source_field: field,
                    ownership,
                    was_explicit,
                });
            }
        }
        if let Some(units_seq) = m.get("units").and_then(|v| v.as_sequence()) {
            for unit_val in units_seq {
                let was_explicit = is_explicit_unit_shape(unit_val);
                if let Ok(u) = LogicalUnit::from_yaml(unit_val) {
                    out.push(UnitEntry {
                        unit: u,
                        source_field: field,
                        ownership,
                        was_explicit,
                    });
                }
            }
        }
    }
}

/// Surface-syntax test that mirrors V-023's bare-vs-explicit split.
/// `true` when the YAML value is a mapping (explicit `{ kind: ..., ... }`
/// form); `false` when it is a bare string (the legacy compat
/// affordance that parses to `LogicalUnit::File`).
fn is_explicit_unit_shape(v: &serde_yaml::Value) -> bool {
    v.is_mapping()
}
