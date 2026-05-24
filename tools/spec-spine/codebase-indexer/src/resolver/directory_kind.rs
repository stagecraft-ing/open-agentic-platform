//! `directory:` resolver. Expand `<path>/**` with the spec 154 §3.7
//! exclusion set. Each matched file emits `ResolvedLocation { file,
//! span: None }`.
//!
//! Determinism: the walk is wrapped with `sort_by_file_name` and the
//! resulting file list sorts again at the resolver-mod boundary (the
//! function contract from `super::resolve`).
//!
//! Per spec 154 §3.7 (amended 2026-05-24), exclusion is two-layered:
//!   1. Baseline list (contract floor): `target/**`, `node_modules/**`,
//!      `.derived/**`, `dist/**`, `build/**`, `.next/**`. Sourced from
//!      `open_agentic_spec_types::RESOLVER_EXCLUSIONS`.
//!   2. Committed `.gitignore` files (worktree-derived, additive).
//!      `ignore::WalkBuilder` is configured with `git_ignore(true)`,
//!      `git_exclude(false)`, `git_global(false)` so per-clone and
//!      per-user exclusion sources are NOT honored — keeping the walk
//!      deterministic across machines.

use super::{ResolveError, ResolverContext};
use crate::types::ResolvedLocation;
use ignore::WalkBuilder;
use open_agentic_spec_types::RESOLVER_EXCLUSIONS;
use std::cmp::Ordering;
use std::path::Path;

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
    let walker = WalkBuilder::new(&abs)
        .git_ignore(true)
        .git_exclude(false)
        .git_global(false)
        .ignore(false)
        .hidden(false)
        .parents(true)
        .sort_by_file_name(|a, b| match (a.to_str(), b.to_str()) {
            (Some(a), Some(b)) => a.cmp(b),
            _ => Ordering::Equal,
        })
        .build();
    let mut out = Vec::new();
    for ent in walker {
        let Ok(ent) = ent else {
            continue;
        };
        let file_type = match ent.file_type() {
            Some(ft) => ft,
            None => continue,
        };
        if !file_type.is_file() {
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

/// Spec 154 §3.7 baseline-exclusion floor. Matches a path if any of
/// its components equals one of the exclusion directory names. This
/// is the operational reading of the `target/**`, `node_modules/**`,
/// `.derived/**`, `dist/**`, `build/**`, `.next/**` globs declared in
/// `open_agentic_spec_types::RESOLVER_EXCLUSIONS`. Kept as the
/// contract floor even when `.gitignore` filtering is active.
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
