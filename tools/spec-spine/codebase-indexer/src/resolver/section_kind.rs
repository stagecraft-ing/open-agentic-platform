//! `section:` resolver. Dispatches by file extension to one of the
//! four anchor parsers (Makefile / workflow YAML / region marker /
//! markdown heading). The target file must exist; the anchor must be
//! present in the file. Both are hard errors when violated, per spec
//! 154 §3.4.

use super::anchor_parsers::dispatch;
use super::{ResolveError, ResolverContext};
use crate::types::ResolvedLocation;
use std::fs;

pub fn resolve_section(
    file: &str,
    anchor: &str,
    ctx: &ResolverContext,
) -> Result<Vec<ResolvedLocation>, ResolveError> {
    let abs = ctx.repo_root.join(file);
    if !abs.is_file() {
        return Err(ResolveError::SectionFileMissing {
            file: file.to_string(),
        });
    }
    let content = fs::read_to_string(&abs).map_err(|_| ResolveError::SectionFileMissing {
        file: file.to_string(),
    })?;
    let parser = dispatch(&ctx.anchor_parsers, file);
    let span = parser
        .find_anchor(&content, anchor)
        .map_err(|reason| ResolveError::MalformedAnchorFile {
            file: file.to_string(),
            reason,
        })?
        .ok_or_else(|| ResolveError::AnchorNotFound {
            file: file.to_string(),
            anchor: anchor.to_string(),
        })?;
    Ok(vec![ResolvedLocation {
        file: file.replace('\\', "/"),
        span: Some(span),
    }])
}
