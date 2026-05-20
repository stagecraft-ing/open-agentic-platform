//! Spec frontmatter reader (Layer 2 input).

use open_agentic_spec_types::split_frontmatter_required;
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

    for spec_path in &entries {
        if let Some(rec) = parse_spec(spec_path) {
            records.push(rec);
        }
    }

    records
}

fn is_spec_dir(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() >= 5 && b[..3].iter().all(|u| u.is_ascii_digit()) && b[3] == b'-'
}

fn parse_spec(path: &Path) -> Option<SpecRecord> {
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
    let implements = parse_implements(fm);
    let amends = parse_string_list(fm, "amends");
    let amendment_record = fm
        .get("amendment_record")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(SpecRecord {
        id,
        status,
        implementation,
        depends_on,
        implements,
        amends,
        amendment_record,
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

/// Parse the `implements` field from raw YAML frontmatter, with the
/// spec-130 relationship-graph derivation applied: when the spec carries
/// no explicit `implements:` (or carries one alongside relationship
/// fields), the indexer also surfaces paths from `establishes`,
/// `extends[].paths`, `refines[].paths`, and `co_authority[].paths` so
/// the gate sees the full claim surface for the new relationship-graph
/// authoring mode.
///
/// This reads the YAML directly because the spec-compiler's extraFrontmatter
/// rejects nested mappings (V-002). The indexer has its own read path.
fn parse_implements(fm: &serde_yaml::Mapping) -> Vec<ImplementsEntry> {
    let mut entries = Vec::new();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    // Explicit `implements:` (legacy authoring shape) — preserved.
    if let Some(val) = fm.get("implements") {
        if let Some(seq) = val.as_sequence() {
            for item in seq {
                if let Some(mapping) = item.as_mapping() {
                    let crate_name = mapping
                        .get("crate")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let path = mapping
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    // Spec 147 — optional `primary: true` per implements item.
                    // Only Some(true) is meaningful; explicit `false` is treated
                    // identically to absent (the corpus default).
                    let primary = mapping
                        .get("primary")
                        .and_then(|v| v.as_bool())
                        .filter(|b| *b)
                        .map(|_| true);
                    if let Some(path) = path {
                        if seen.insert(path.clone()) {
                            entries.push(ImplementsEntry { crate_name, path, primary });
                        }
                    }
                }
            }
        }
    }

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

    // `establishes:` — flat string list.
    if let Some(val) = fm.get("establishes") {
        if let Some(seq) = val.as_sequence() {
            for item in seq {
                if let Some(s) = item.as_str() {
                    push_path(s.to_string());
                }
            }
        }
    }

    // Extract `paths:` from each item in a structured field
    // (`extends`, `refines`, `co_authority`).
    let extract_paths_from = |fm: &serde_yaml::Mapping, key: &str| -> Vec<String> {
        let mut out = Vec::new();
        if let Some(val) = fm.get(key) {
            if let Some(seq) = val.as_sequence() {
                for item in seq {
                    if let Some(mapping) = item.as_mapping() {
                        if let Some(paths_val) = mapping.get("paths") {
                            if let Some(paths_seq) = paths_val.as_sequence() {
                                for p in paths_seq {
                                    if let Some(s) = p.as_str() {
                                        out.push(s.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    };

    for path in extract_paths_from(fm, "extends") {
        push_path(path);
    }
    for path in extract_paths_from(fm, "refines") {
        push_path(path);
    }
    for path in extract_paths_from(fm, "co_authority") {
        push_path(path);
    }

    entries
}
