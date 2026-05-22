//! `crate:` resolver. Look up the canonical workspace-member name in
//! `ctx.workspace_members`, then expand the member's directory under
//! the spec 154 §3.7 exclusion set (same machinery as `directory:`).
//!
//! Spec 154 §3.1: missing crate id is a hard error.

use super::{ResolveError, ResolverContext};
use crate::types::ResolvedLocation;

pub fn resolve_crate(
    id: &str,
    ctx: &ResolverContext,
) -> Result<Vec<ResolvedLocation>, ResolveError> {
    let Some(dir) = ctx.workspace_members.get(id) else {
        return Err(ResolveError::UnknownCrate { id: id.to_string() });
    };
    // Crate expansion is directory expansion under the spec 154 §3.7
    // exclusion set — share the implementation rather than duplicate
    // the walkdir loop.
    super::directory_kind::resolve_directory(dir, ctx)
}
