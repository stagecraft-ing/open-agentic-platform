//! `module:` resolver. Looks up the Rust module path in
//! `module_index::ModuleIndex` and returns its single
//! `ResolvedLocation`. Spec 155 §2.2: missing module is a hard error.

use super::{ResolveError, ResolverContext};
use crate::types::ResolvedLocation;

pub fn resolve_module(
    id: &str,
    ctx: &ResolverContext,
) -> Result<Vec<ResolvedLocation>, ResolveError> {
    match ctx.module_index.by_path.get(id) {
        Some(loc) => Ok(vec![loc.clone()]),
        None => Err(ResolveError::MissingModule { id: id.to_string() }),
    }
}
