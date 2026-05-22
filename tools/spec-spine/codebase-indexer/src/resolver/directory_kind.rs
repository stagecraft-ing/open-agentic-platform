//! `directory:` resolver. Expand `<path>/**` with the spec 154 §3.7
//! exclusion set. Each matched file emits `ResolvedLocation { file,
//! span: None }`.
//!
//! Determinism: walkdir is wrapped with `sort_by` and the resulting
//! file list sorts again at the resolver-mod boundary (the function
//! contract from `super::resolve`).

use super::{ResolveError, ResolverContext};
use crate::types::ResolvedLocation;
use open_agentic_spec_types::RESOLVER_EXCLUSIONS;
use std::path::Path;
use walkdir::WalkDir;

pub fn resolve_directory(
    path: &str,
    ctx: &ResolverContext,
) -> Result<Vec<ResolvedLocation>, ResolveError> {
    let abs = ctx.repo_root.join(path);
    if !abs.is_dir() {
        return Err(ResolveError::MissingDirectory {
            path: path.to_string(),
        });
    }
    let mut out = Vec::new();
    let walker = WalkDir::new(&abs).sort_by_file_name();
    for ent in walker {
        let Ok(ent) = ent else {
            continue;
        };
        if !ent.file_type().is_file() {
            continue;
        }
        let rel = match ent.path().strip_prefix(&ctx.repo_root) {
            Ok(r) => r.to_path_buf(),
            Err(_) => continue,
        };
        if is_excluded(&rel) {
            continue;
        }
        out.push(ResolvedLocation {
            file: rel.to_string_lossy().replace('\\', "/"),
            span: None,
        });
    }
    Ok(out)
}

/// Spec 154 §3.7 exclusion set. Matches a path if any of its
/// components equals one of the exclusion directory names. This is
/// the operational reading of the `target/**`, `node_modules/**`,
/// `.derived/**`, `dist/**`, `build/**`, `.next/**` globs declared in
/// `open_agentic_spec_types::RESOLVER_EXCLUSIONS`.
pub(super) fn is_excluded(rel: &Path) -> bool {
    let names: std::collections::BTreeSet<&str> = RESOLVER_EXCLUSIONS
        .iter()
        .map(|glob| glob.trim_end_matches("/**"))
        .collect();
    rel.components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .any(|name| names.contains(name))
}
