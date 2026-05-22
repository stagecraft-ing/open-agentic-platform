//! `symbol:` resolver. Bare-path lookup into the symbol index built
//! by `symbol_index::build`. Spec 154 §3.2 (as amended by spec 155
//! §2.1): missing symbol is a hard error.

use super::{ResolveError, ResolverContext};
use crate::types::ResolvedLocation;

pub fn resolve_symbol(
    id: &str,
    ctx: &ResolverContext,
) -> Result<Vec<ResolvedLocation>, ResolveError> {
    match ctx.symbol_index.by_path.get(id) {
        Some(locs) => Ok(locs.clone()),
        None => Err(ResolveError::UnknownSymbol { id: id.to_string() }),
    }
}
