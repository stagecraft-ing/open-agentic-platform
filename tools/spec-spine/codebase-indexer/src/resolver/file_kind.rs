//! `file:` resolver. Literal worktree path check.
//!
//! Spec 154 §3.6 (as amended by spec 155 §2.4): in compile context,
//! the resolver does not follow git rename traces — that is the
//! Segment 4 gate's concern. Missing path is unconditionally a hard
//! error.
//!
//! Compat-window note: bare-string entries from the corpus today
//! parse as `LogicalUnit::File`, and some of those bare strings
//! reference directories (e.g. `tools/spec-spine/spec-compiler` in
//! spec 154's own `references:` list). To preserve compat through
//! Segments 3-5 we accept any existing path here and emit it with
//! `span: None`. The strict file-vs-directory type-check is the
//! explicit-only-flip work tracked in spec 154 Segment 6.

use super::{ResolveError, ResolverContext};
use crate::types::ResolvedLocation;

pub fn resolve_file(
    path: &str,
    ctx: &ResolverContext,
) -> Result<Vec<ResolvedLocation>, ResolveError> {
    let abs = ctx.repo_root.join(path);
    if !abs.exists() {
        return Err(ResolveError::MissingFile {
            path: path.to_string(),
        });
    }
    Ok(vec![ResolvedLocation {
        file: normalize_path(path),
        span: None,
    }])
}

fn normalize_path(p: &str) -> String {
    p.replace('\\', "/")
}
