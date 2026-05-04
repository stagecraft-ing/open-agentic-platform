//! Cross-reference engine (Layer 2 traceability).

use crate::comment_scanner::CommentHeaderMap;
use crate::spec_scanner::SpecRecord;
use crate::types::{
    Diagnostic, ImplementingPath, PackageRecord, TraceMapping, TraceSource, Traceability,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Build the traceability layer from spec records, package inventory, and
/// per-file comment headers.
///
/// `extra_known_paths` contains non-package paths that are valid implements
/// targets (e.g. factory adapter directories). `comment_headers` maps
/// repo-relative file paths to spec IDs declared via `// Spec:` headers
/// (spec 129). `repo_root` is used as a fallback to verify that declared
/// paths exist on disk (file or directory).
pub fn build_traceability(
    specs: &[SpecRecord],
    packages: &[PackageRecord],
    extra_known_paths: &BTreeSet<String>,
    comment_headers: &CommentHeaderMap,
    repo_root: &Path,
) -> (Traceability, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let package_paths: BTreeSet<String> = packages.iter().map(|p| p.path.clone()).collect();

    // Spec 133: build the full id set up-front so amend links can be
    // resolved from short-form ids (`"000"` → `"000-bootstrap-spec-system"`)
    // exactly the way the spec-compiler resolves them for V-011.
    let all_spec_ids: BTreeSet<String> = specs.iter().map(|s| s.id.clone()).collect();

    // Forward mapping: spec → code (from spec `implements` field)
    let mut mappings: BTreeMap<String, TraceMapping> = BTreeMap::new();

    for spec in specs {
        for imp in &spec.implements {
            // Validate the path exists: package, adapter, or any file/dir on disk.
            // Files (Makefile, AGENTS.md, .github/workflows/*.yml, .claude/**/*.md)
            // are valid governance targets — not every spec implements a whole crate.
            if !package_paths.contains(&imp.path)
                && !extra_known_paths.contains(&imp.path)
                && !repo_root.join(&imp.path).exists()
            {
                diagnostics.push(Diagnostic {
                    code: "I-101".into(),
                    message: format!(
                        "spec {:?} declares implements path {:?} which is not a known package, adapter, or existing file/directory",
                        spec.id, imp.path
                    ),
                    path: None,
                });
            }

            let entry = mappings.entry(spec.id.clone()).or_insert_with(|| {
                mapping_skeleton(&spec.id, Some(spec), &all_spec_ids)
            });

            entry.implementing_paths.push(ImplementingPath {
                path: imp.path.clone(),
                name: imp.crate_name.clone(),
                source: Some(TraceSource::SpecImplements),
            });
        }

        // Spec 133: a spec that participates in the amend protocol but
        // declares no `implements:` paths still needs a mapping so the
        // coupling gate's amend resolver can see it. The mapping starts
        // with empty `implementing_paths`; downstream cross-reference
        // sources may populate them later.
        if !spec.amends.is_empty() || spec.amendment_record.is_some() {
            mappings.entry(spec.id.clone()).or_insert_with(|| {
                mapping_skeleton(&spec.id, Some(spec), &all_spec_ids)
            });
        }
    }

    // Reverse mapping: code → spec (from [package.metadata.oap].spec — crate-level)
    for pkg in packages {
        if let Some(ref spec_id) = pkg.spec_ref {
            let entry = mappings.entry(spec_id.clone()).or_insert_with(|| {
                let spec_rec = specs.iter().find(|s| &s.id == spec_id);
                mapping_skeleton(spec_id, spec_rec, &all_spec_ids)
            });

            // Check if this path is already declared via spec `implements`
            if let Some(existing) = entry
                .implementing_paths
                .iter_mut()
                .find(|p| p.path == pkg.path)
            {
                existing.source = Some(TraceSource::Multiple);
            } else {
                entry.implementing_paths.push(ImplementingPath {
                    path: pkg.path.clone(),
                    name: Some(pkg.name.clone()),
                    source: Some(TraceSource::CargoMetadataCrate),
                });
            }
        }
    }

    // File-level mapping: comment-header → spec (spec 129).
    // Each (file_path, spec_id) becomes its own ImplementingPath entry.
    // If the same path is already claimed by spec-implements or cargo-metadata,
    // the source is upgraded to Multiple.
    for (file_path, spec_id) in comment_headers {
        let entry = mappings.entry(spec_id.clone()).or_insert_with(|| {
            let spec_rec = specs.iter().find(|s| &s.id == spec_id);
            mapping_skeleton(spec_id, spec_rec, &all_spec_ids)
        });

        if let Some(existing) = entry
            .implementing_paths
            .iter_mut()
            .find(|p| &p.path == file_path)
        {
            existing.source = Some(TraceSource::Multiple);
        } else {
            entry.implementing_paths.push(ImplementingPath {
                path: file_path.clone(),
                name: None,
                source: Some(TraceSource::CommentHeader),
            });
        }
    }

    // Sort implementing paths within each mapping
    for mapping in mappings.values_mut() {
        mapping
            .implementing_paths
            .sort_by(|a, b| a.path.cmp(&b.path));
    }

    let mut sorted_mappings: Vec<TraceMapping> = mappings.into_values().collect();
    sorted_mappings.sort_by(|a, b| a.spec_id.cmp(&b.spec_id));

    // Orphan detection
    let traced_spec_ids: BTreeSet<String> =
        sorted_mappings.iter().map(|m| m.spec_id.clone()).collect();
    let traced_code_paths: BTreeSet<String> = sorted_mappings
        .iter()
        .flat_map(|m| m.implementing_paths.iter().map(|p| p.path.clone()))
        .collect();

    // Orphaned specs: specs with implementation != n/a that have no traced code
    let mut orphaned_specs: Vec<String> = specs
        .iter()
        .filter(|s| {
            let impl_status = s.implementation.as_deref().unwrap_or("pending");
            impl_status != "n/a" && !traced_spec_ids.contains(&s.id)
        })
        .map(|s| s.id.clone())
        .collect();
    orphaned_specs.sort();

    // Untraced code: packages with no governing spec
    let mut untraced_code: Vec<String> = packages
        .iter()
        .filter(|p| p.spec_ref.is_none() && !traced_code_paths.contains(&p.path))
        .map(|p| p.path.clone())
        .collect();
    untraced_code.sort();

    let traceability = Traceability {
        mappings: sorted_mappings,
        orphaned_specs,
        untraced_code,
    };

    (traceability, diagnostics)
}

/// Spec 133: build a fresh `TraceMapping` for `spec_id`, populating
/// `amends:` / `amendment_record:` from the spec record's frontmatter
/// when available and resolving short-form ids against the full spec
/// id set (so `amends: ["000"]` lands as `"000-bootstrap-spec-system"`).
/// Cross-reference sources later fill `implementing_paths`.
fn mapping_skeleton(
    spec_id: &str,
    spec_rec: Option<&SpecRecord>,
    all_spec_ids: &BTreeSet<String>,
) -> TraceMapping {
    let status = spec_rec.map(|s| s.status.clone());
    let deps = spec_rec.map(|s| s.depends_on.clone()).unwrap_or_default();
    let amends = spec_rec
        .map(|s| {
            s.amends
                .iter()
                .filter_map(|raw| resolve_spec_id(raw, all_spec_ids))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let amendment_record = spec_rec
        .and_then(|s| s.amendment_record.as_ref())
        .and_then(|raw| resolve_spec_id(raw, all_spec_ids));
    TraceMapping {
        spec_id: spec_id.to_string(),
        spec_status: status,
        depends_on: deps,
        amends,
        amendment_record,
        implementing_paths: Vec::new(),
    }
}

/// Spec 133: resolve a spec-id reference (full `NNN-slug` or short `NNN`)
/// against the canonical id set. Returns the full id when matched,
/// `None` for dangling references. The single-numeric-prefix rule
/// matches the spec-compiler's V-011 short-form resolution (spec 132).
fn resolve_spec_id(raw: &str, all_spec_ids: &BTreeSet<String>) -> Option<String> {
    if all_spec_ids.contains(raw) {
        return Some(raw.to_string());
    }
    // Short form: bare numeric id. Match the unique full id with that
    // 3-digit prefix; ignore on ambiguity (silent — diagnostics are
    // out of scope for this resolver).
    let is_short = raw.len() == 3 && raw.bytes().all(|b| b.is_ascii_digit());
    if is_short {
        let prefix = format!("{raw}-");
        let mut matches = all_spec_ids
            .iter()
            .filter(|id| id.starts_with(&prefix));
        let first = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        return Some(first.clone());
    }
    None
}
