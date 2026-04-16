//! Cross-reference engine (Layer 2 traceability).

use crate::spec_scanner::SpecRecord;
use crate::types::{
    Diagnostic, ImplementingPath, PackageRecord, TraceMapping, TraceSource, Traceability,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Build the traceability layer from spec records and package inventory.
///
/// `extra_known_paths` contains non-package paths that are valid implements
/// targets (e.g. factory adapter directories). `repo_root` is used as a
/// fallback to verify that declared paths exist on disk.
pub fn build_traceability(
    specs: &[SpecRecord],
    packages: &[PackageRecord],
    extra_known_paths: &BTreeSet<String>,
    repo_root: &Path,
) -> (Traceability, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let package_paths: BTreeSet<String> = packages.iter().map(|p| p.path.clone()).collect();

    // Forward mapping: spec → code (from spec `implements` field)
    let mut mappings: BTreeMap<String, TraceMapping> = BTreeMap::new();

    for spec in specs {
        for imp in &spec.implements {
            // Validate the path exists: check packages, then adapters, then disk
            if !package_paths.contains(&imp.path)
                && !extra_known_paths.contains(&imp.path)
                && !repo_root.join(&imp.path).is_dir()
            {
                diagnostics.push(Diagnostic {
                    code: "I-101".into(),
                    message: format!(
                        "spec {:?} declares implements path {:?} which is not a known package, adapter, or directory",
                        spec.id, imp.path
                    ),
                    path: None,
                });
            }

            let entry = mappings
                .entry(spec.id.clone())
                .or_insert_with(|| TraceMapping {
                    spec_id: spec.id.clone(),
                    spec_status: Some(spec.status.clone()),
                    depends_on: spec.depends_on.clone(),
                    implementing_paths: Vec::new(),
                });

            entry.implementing_paths.push(ImplementingPath {
                path: imp.path.clone(),
                name: imp.crate_name.clone(),
                source: Some(TraceSource::SpecImplements),
            });
        }
    }

    // Reverse mapping: code → spec (from [package.metadata.oap].spec)
    for pkg in packages {
        if let Some(ref spec_id) = pkg.spec_ref {
            let entry = mappings.entry(spec_id.clone()).or_insert_with(|| {
                // Find the spec record for status and depends_on
                let spec_rec = specs.iter().find(|s| &s.id == spec_id);
                let status = spec_rec.map(|s| s.status.clone());
                let deps = spec_rec.map(|s| s.depends_on.clone()).unwrap_or_default();
                TraceMapping {
                    spec_id: spec_id.clone(),
                    spec_status: status,
                    depends_on: deps,
                    implementing_paths: Vec::new(),
                }
            });

            // Check if this path is already declared via spec `implements`
            if let Some(existing) = entry
                .implementing_paths
                .iter_mut()
                .find(|p| p.path == pkg.path)
            {
                existing.source = Some(TraceSource::Both);
            } else {
                entry.implementing_paths.push(ImplementingPath {
                    path: pkg.path.clone(),
                    name: Some(pkg.name.clone()),
                    source: Some(TraceSource::CargoMetadata),
                });
            }
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
