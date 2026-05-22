//! Spec 154 Segment 3 — Logical-unit resolver.
//!
//! Takes a parsed `LogicalUnit` plus the shared `ResolverContext` and
//! emits a deterministic `Vec<ResolvedLocation>`. Per-kind dispatch
//! lives in the sibling modules; this module owns the shared types
//! (`ResolverContext`, `ResolveError`), the dispatch function, and the
//! `resolve_all` batch entry point the indexer's `compile` pass calls.

use crate::spec_scanner::{SpecRecord, UnitEntry};
use crate::types::{Diagnostic, ResolvedLocation, ResolvedUnit};
use open_agentic_spec_types::LogicalUnit;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub mod anchor_parsers;
pub mod crate_kind;
pub mod directory_kind;
pub mod file_kind;
pub mod module_index;
pub mod module_kind;
pub mod section_kind;
pub mod symbol_index;
pub mod symbol_kind;

use anchor_parsers::AnchorParserRegistry;
use module_index::ModuleIndex;
use symbol_index::SymbolIndex;

// ── ResolverContext ─────────────────────────────────────────────────────────

/// Per-compile context. Built once at the top of the indexer's
/// `compile` pass and passed by reference to every `resolve` call.
///
/// Holds the inputs that are expensive to compute (workspace-member
/// index, symbol/module indices) and the dispatch table for
/// section-kind anchor parsers.
pub struct ResolverContext {
    /// Absolute repo root. Used to existence-check paths.
    pub repo_root: PathBuf,
    /// Workspace-member name → workspace-relative directory path.
    /// Built from the `Vec<PackageRecord>` already produced by the
    /// manifest pass, filtered to Rust crates. The keys match
    /// spec-compiler's `discover_workspace_crate_ids` output (the
    /// `[package].name` from each member manifest, not the directory
    /// tail).
    pub workspace_members: BTreeMap<String, String>,
    /// Symbol index built by a single tree-sitter pass over the
    /// workspace's `*.rs` files. Keyed by qualified Rust path.
    pub symbol_index: SymbolIndex,
    /// Module index (file-modules + inline modules).
    pub module_index: ModuleIndex,
    /// Anchor-parser dispatch table for `section:` units.
    pub anchor_parsers: AnchorParserRegistry,
}

impl ResolverContext {
    /// Build the context. The symbol and module indices are built
    /// lazily by `symbol_index::build` / `module_index::build`
    /// against the workspace-member roots.
    pub fn build(repo_root: &Path, packages: &[crate::types::PackageRecord]) -> Self {
        let workspace_members = collect_workspace_members(repo_root, packages);
        let symbol_index = symbol_index::build(repo_root, &workspace_members);
        let module_index = module_index::build(repo_root, &workspace_members);
        let anchor_parsers = anchor_parsers::default_anchor_parsers();
        Self {
            repo_root: repo_root.to_path_buf(),
            workspace_members,
            symbol_index,
            module_index,
            anchor_parsers,
        }
    }
}

/// Read each workspace member's manifest `name` from disk into the
/// resolver's lookup table. Mirrors spec-compiler's
/// `discover_workspace_crate_ids` so the indexer's resolver and the
/// compiler's type-checker share one truth about which crate ids are
/// valid (spec 154 §3.1 — workspace membership is the manifest
/// boundary, not the language; Rust crates AND npm packages declared
/// as workspace members of `product/` both contribute).
fn collect_workspace_members(
    _repo_root: &Path,
    packages: &[crate::types::PackageRecord],
) -> BTreeMap<String, String> {
    use crate::types::PackageKind;
    let mut out = BTreeMap::new();
    for pkg in packages {
        if !matches!(
            pkg.kind,
            PackageKind::RustLib
                | PackageKind::RustBin
                | PackageKind::RustLibBin
                | PackageKind::NpmPackage
        ) {
            continue;
        }
        // `PackageRecord.name` is the manifest's canonical name field
        // (Rust `[package].name` or npm `package.json:name`). Both
        // serve as `crate:` unit ids under spec 154 §3.1.
        out.insert(pkg.name.clone(), pkg.path.clone());
    }
    out
}

// ── ResolveError ────────────────────────────────────────────────────────────

/// Resolution failure. All variants are hard errors per spec 154
/// §3.1..§3.6 (as amended by spec 155); callers downgrade them to
/// `Diagnostic` entries (codes `I-003`..`I-009`) so the index still
/// emits, but the failing unit's `locations` is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// `crate:` id is not a workspace member (spec 154 §3.1).
    UnknownCrate { id: String },
    /// `symbol:` id is not in the symbol index (spec 154 §3.2).
    UnknownSymbol { id: String },
    /// `module:` id is not in the module index (spec 155 §2.2).
    MissingModule { id: String },
    /// `section:` anchor is absent from the target file (spec 154 §3.4).
    AnchorNotFound { file: String, anchor: String },
    /// `section:` target file is absent.
    SectionFileMissing { file: String },
    /// `directory:` path is not a directory on disk (spec 155 §2.3).
    MissingDirectory { path: String },
    /// `file:` path does not exist on disk (spec 155 §2.4: rename-trace
    /// follow is a Segment 4 gate concern, not a compile-context
    /// resolver concern).
    MissingFile { path: String },
    /// `region:` marker without a matching `endregion` (or similar
    /// malformed anchor file).
    MalformedAnchorFile { file: String, reason: String },
}

impl ResolveError {
    /// Stable diagnostic code emitted by the indexer when this error
    /// is downgraded to a `Diagnostic`. Lands in `diagnostics.errors`
    /// per the existing `I-0xx → errors` convention.
    ///
    /// `was_explicit` carries the V-023 bare-vs-explicit split for
    /// `MissingFile`: explicit `{kind: file, path: X}` whose `X` is
    /// absent fires the blocking `I-008` (the resolver mirror of
    /// V-023); a bare-string compat parse whose path is absent fires
    /// `I-108` in the warning band — visible, capturable for the
    /// Segment 5 L-005 worklist, non-blocking. Segment 6's
    /// explicit-only flip lifts `I-108 → I-008` in lockstep with the
    /// V-021..V-024 promotion. Other variants do not split on
    /// `was_explicit` today: `crate:`, `symbol:`, `module:`,
    /// `section:`, and `directory:` units have no bare-string form
    /// in the corpus (they require an explicit `kind:`), so the
    /// bit is informational for them.
    ///
    /// Note: only invoked for owning-field units. References-field
    /// entries skip diagnostic emission entirely (4' clarification —
    /// see `build_resolved_unit`).
    pub fn diagnostic_code(&self, _was_explicit: bool) -> &'static str {
        match self {
            ResolveError::UnknownCrate { .. } => "I-003",
            ResolveError::UnknownSymbol { .. } => "I-004",
            ResolveError::MissingModule { .. } => "I-005",
            ResolveError::AnchorNotFound { .. } => "I-006",
            ResolveError::SectionFileMissing { .. } => "I-006",
            ResolveError::MissingDirectory { .. } => "I-007",
            // Segment 6 explicit-only flip: I-108 (the bare-string
            // MissingFile compat-window warning) is retired. The
            // bare-string parse arm is excised from spec-compiler;
            // every owning-field unit reaching the resolver is
            // explicit by construction. Missing files fire the
            // blocking I-008 on both shapes.
            ResolveError::MissingFile { .. } => "I-008",
            ResolveError::MalformedAnchorFile { .. } => "I-009",
        }
    }
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::UnknownCrate { id } => {
                write!(f, "crate: id {id:?} is not a workspace member")
            }
            ResolveError::UnknownSymbol { id } => {
                write!(f, "symbol: id {id:?} not found in workspace symbol index")
            }
            ResolveError::MissingModule { id } => {
                write!(f, "module: id {id:?} not found in workspace module index")
            }
            ResolveError::AnchorNotFound { file, anchor } => {
                write!(f, "section: anchor {anchor:?} not found in {file:?}")
            }
            ResolveError::SectionFileMissing { file } => {
                write!(f, "section: target file {file:?} does not exist")
            }
            ResolveError::MissingDirectory { path } => {
                write!(f, "directory: path {path:?} does not exist or is not a directory")
            }
            ResolveError::MissingFile { path } => {
                write!(f, "file: path {path:?} does not exist on disk")
            }
            ResolveError::MalformedAnchorFile { file, reason } => {
                write!(f, "section: anchor file {file:?} is malformed: {reason}")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

// ── Dispatch ────────────────────────────────────────────────────────────────

/// Resolve a single `LogicalUnit` to its deterministic
/// `Vec<ResolvedLocation>`. The returned list is sorted by the derived
/// `Ord` of `ResolvedLocation` (file lexicographic, then span).
pub fn resolve(
    unit: &LogicalUnit,
    ctx: &ResolverContext,
) -> Result<Vec<ResolvedLocation>, ResolveError> {
    let mut locations = match unit {
        LogicalUnit::Crate { id } => crate_kind::resolve_crate(id, ctx)?,
        LogicalUnit::Symbol { id } => symbol_kind::resolve_symbol(id, ctx)?,
        LogicalUnit::Module { id } => module_kind::resolve_module(id, ctx)?,
        LogicalUnit::Section { file, anchor } => {
            section_kind::resolve_section(file, anchor, ctx)?
        }
        LogicalUnit::Directory { path } => directory_kind::resolve_directory(path, ctx)?,
        LogicalUnit::File { path } => file_kind::resolve_file(path, ctx)?,
    };
    // Spec 154 Segment 3 §3 — determinism contract at the function
    // boundary. Sorting here means no caller has to remember to do it,
    // and a future per-kind resolver that emits unsorted locations
    // can't accidentally leak nondeterminism into the index.
    locations.sort();
    locations.dedup();
    Ok(locations)
}

// ── Batch entry point used by `compile` ─────────────────────────────────────

/// Resolve every unit on every spec. Returns the per-spec
/// `Vec<ResolvedUnit>` map and the accumulated diagnostics (one per
/// resolution failure). The unit declaration is preserved in the
/// `ResolvedUnit` even when resolution fails, so consumers can
/// correlate a diagnostic back to its source.
pub fn resolve_all(
    specs: &[SpecRecord],
    ctx: &ResolverContext,
) -> (BTreeMap<String, Vec<ResolvedUnit>>, Vec<Diagnostic>) {
    let mut by_spec: BTreeMap<String, Vec<ResolvedUnit>> = BTreeMap::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    for spec in specs {
        if spec.units.is_empty() {
            continue;
        }
        let entries = by_spec.entry(spec.id.clone()).or_default();
        for unit_entry in &spec.units {
            let resolved = build_resolved_unit(unit_entry, ctx, &spec.id, &mut diagnostics);
            entries.push(resolved);
        }
        // Determinism: sort the per-spec resolved-unit list by a stable
        // composite key so the index round-trips byte-identically.
        entries.sort_by(|a, b| {
            a.source_field
                .cmp(&b.source_field)
                .then_with(|| a.kind.cmp(&b.kind))
                .then_with(|| canonical_unit_key(&a.unit).cmp(&canonical_unit_key(&b.unit)))
        });
    }

    (by_spec, diagnostics)
}

fn build_resolved_unit(
    entry: &UnitEntry,
    ctx: &ResolverContext,
    spec_id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> ResolvedUnit {
    let unit_json = entry.unit.to_json();
    let kind = entry.unit.kind_str().to_string();
    let locations = match resolve(&entry.unit, ctx) {
        Ok(locs) => locs,
        Err(err) => {
            // Spec 154 §4 (Segment 6, 4' clarification): references
            // are non-owning by design. Resolution failure for a
            // references entry is not a diagnostic — the spec gestures
            // at a target without claiming ownership. The bare-vs-
            // explicit split (I-008 / I-108) routes only on owning
            // fields. Mirrors spec 156's dangling-provenance
            // treatment: empty `locations`, no diagnostic.
            if entry.ownership {
                diagnostics.push(Diagnostic {
                    code: err.diagnostic_code(entry.was_explicit).to_string(),
                    message: format!("{spec_id}: {err}"),
                    path: None,
                });
            }
            Vec::new()
        }
    };
    ResolvedUnit {
        unit: unit_json,
        kind,
        source_field: entry.source_field.to_string(),
        ownership: entry.ownership,
        locations,
    }
}

/// Stable per-unit sort key. `LogicalUnit::to_json` already emits a
/// canonical `{ kind, ... }` shape; the key is the JSON text in
/// canonical form (keys serialised in declaration order, which is
/// deterministic for each kind).
fn canonical_unit_key(unit: &serde_json::Value) -> String {
    serde_json::to_string(unit).unwrap_or_default()
}
