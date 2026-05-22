//! Core data types mapping to `standards/schemas/spec-spine/codebase-index.schema.json`.

use serde::{Deserialize, Serialize};

/// Schema version — compile-time contract between the indexer and the JSON Schema.
/// Bumped to 1.1.0 in spec 118 (additive: optional `workflowTraceability` block).
/// Bumped to 1.2.0 in spec 129 (TraceSource extended: cargo-metadata renamed
/// to cargo-metadata-crate; cargo-metadata-module reserved; comment-header
/// added for file-level annotations; both → multiple for any 2+ overlap).
/// Bumped to 1.3.0 in spec 133 (TraceMapping extended: `amends` list and
/// `amendmentRecord` string surface the spec 119 amendment protocol so
/// the spec/code coupling gate can recognise amender→amended edits as
/// valid coupling alongside `implements:`).
/// Bumped to 1.4.0 in spec 147 (ImplementingPath extended with optional
/// `primary` boolean. Surfaces per-claim primary ownership for paths
/// declared via the new `implements:` list-item shape; downgrades to
/// the any-one-claimant heuristic when absent, preserving backward
/// compatibility with paths not yet annotated).
/// Schema version. Cut D W-07c: bumped to 2.0.0 with the lift of
/// Layers 3-5 (factory adapters, infrastructure inventory, workflow
/// traceability) out of the generic indexer and into
/// `tools/oap-code-index-enrich`. The generic schema is now Layer
/// 1 (crate/package inventory) + Layer 2 (spec-to-code traceability)
/// only. Consumers needing Layers 3-5 read `index-oap.json` from the
/// OAP enricher, validated by `standards/schemas/spec-spine/codebase-index-oap.schema.json`.
/// Bumped to 2.1.0 in spec 154 Segment 3 (additive: `resolvedUnits` array
/// on `traceMapping`; each entry pairs a logical-unit declaration with
/// the deterministic set of `(file, span)` physical locations the
/// resolver emits). The field defaults to empty under serde, so 2.0.0
/// consumers keep deserializing 2.1.0 indices unchanged.
pub const SCHEMA_VERSION: &str = "2.1.0";
pub const INDEXER_ID: &str = "codebase-indexer";

// ── Top-level output ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodebaseIndex {
    pub schema_version: String,
    pub build: BuildInfo,
    pub inventory: Vec<PackageRecord>,
    pub traceability: Traceability,
    pub diagnostics: Diagnostics,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub indexer_id: String,
    pub indexer_version: String,
    pub repo_root: String,
    pub content_hash: String,
}

// ── Layer 1: Crate & Package Inventory ──────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PackageRecord {
    pub name: String,
    pub path: String,
    pub kind: PackageKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_points: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_deps: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_deps: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_ref: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PackageKind {
    RustLib,
    RustBin,
    RustLibBin,
    NpmPackage,
    NpmWorkspace,
}

// ── Layer 2: Spec-to-Code Traceability ──────────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Traceability {
    pub mappings: Vec<TraceMapping>,
    pub orphaned_specs: Vec<String>,
    pub untraced_code: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceMapping {
    pub spec_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_status: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Spec 133: spec ids this mapping's spec amends in place via the
    /// spec 119 protocol. Resolved to full `NNN-slug` ids at index-build
    /// time so consumers do not need to re-resolve short forms.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub amends: Vec<String>,
    /// Spec 133: the spec id whose amendment record applies to this
    /// mapping's spec (the reverse-link, set on the amended spec's
    /// frontmatter as `amendment_record:`). Resolved to a full
    /// `NNN-slug` id at index-build time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amendment_record: Option<String>,
    pub implementing_paths: Vec<ImplementingPath>,
    /// Spec 154 Segment 3: per-spec logical-unit declarations paired
    /// with the deterministic set of `(file, span)` physical locations
    /// the resolver emits. Defaults to empty under serde so a spec with
    /// no logical-unit grammar in its frontmatter — or a 2.0.0 consumer
    /// reading a 2.1.0 index — sees the same shape it always has. The
    /// resolver populates this field after path-list traceability is
    /// built; the coupling gate (Segment 4) reads `locations` for
    /// diff-hunk → owning-spec reverse lookup.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_units: Vec<ResolvedUnit>,
}

// ── Spec 154 Segment 3 — Resolved logical-unit graph ────────────────────────

/// A resolved logical unit: the unit declaration as authored, paired
/// with the deterministic set of physical locations the resolver
/// emitted for it.
///
/// `locations` is sorted by the derived `Ord` of `ResolvedLocation`
/// (file lexicographic, then span). For ownership-bearing units the
/// list is non-empty when resolution succeeded; resolution failures
/// are downgraded to `Diagnostic` entries and produce an empty
/// `locations` list (the failing unit is still preserved here so
/// consumers can correlate the diagnostic back to its declaration).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedUnit {
    /// The logical-unit declaration as it appeared in frontmatter,
    /// serialized via the canonical `{ kind, ... }` shape (mirrors
    /// `LogicalUnit::to_json`).
    pub unit: serde_json::Value,
    /// Stable kind discriminator (`crate` / `symbol` / `module` /
    /// `section` / `directory` / `file`). Duplicates `unit.kind` for
    /// consumers that want the discriminator without re-parsing.
    pub kind: String,
    /// Which relationship field carried the declaration. One of
    /// `establishes`, `extends`, `refines`, `supersedes`, `amends`,
    /// `co_authority`, `constrains`, `references`. `ownership` is
    /// `false` only for `references`; the coupling gate ignores
    /// non-ownership units.
    pub source_field: String,
    /// Whether this unit confers authority over its locations.
    /// `true` for the seven ownership-bearing relationships;
    /// `false` for `references`.
    pub ownership: bool,
    /// Deterministic, sorted list of physical locations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<ResolvedLocation>,
}

/// A single resolved physical location for a logical unit.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedLocation {
    /// Workspace-relative file path (POSIX separators).
    pub file: String,
    /// Optional span within the file. `None` means "whole file"; the
    /// coupling gate's line-range overlap check treats `None` as
    /// `[1, ∞]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<LineSpan>,
}

/// Inclusive 1-indexed line range. Aligned with the `@@ -a,b +c,d @@`
/// shape of `git diff -U0` hunks so the Segment 4 gate can do a
/// line-range overlap check without byte-offset conversion or any
/// tree-sitter dependency in the gate itself.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct LineSpan {
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementingPath {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<TraceSource>,
    /// Spec 147 — true when this path is the primary owner of the
    /// claim. Omitted when the spec did not annotate the item with
    /// `primary: true`. Spec 147 V-016 enforces corpus-wide uniqueness
    /// (at most one spec declares primary for any given path); when
    /// the flag is absent across all claimants, downstream consumers
    /// fall back to spec 130's any-one-claimant heuristic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TraceSource {
    /// Path declared in a spec's `implements:` frontmatter list.
    SpecImplements,
    /// `[package.metadata.oap].spec` in the crate's root Cargo.toml
    /// (renamed from the legacy `cargo-metadata` in schema 1.1; spec 129).
    CargoMetadataCrate,
    /// Reserved for future per-target `[<lib|bin>.metadata.oap]` annotations.
    /// Schema 1.2 declares the variant; the indexer does not yet emit it.
    CargoMetadataModule,
    /// `// Spec: specs/NNN-slug/spec.md` doc-comment header at file root
    /// (within the leading comment block, before any non-comment statement).
    /// Spec 129.
    CommentHeader,
    /// Two or more sources independently asserted the same (spec, path).
    /// Replaces the legacy `Both` variant which only modelled the
    /// SpecImplements + CargoMetadataCrate overlap.
    Multiple,
}

// Cut D W-07c: Layer 3 (AdapterRecord), Layer 4 (Infrastructure /
// ToolEntry / NamedEntry), and Layer 5 (WorkflowTrace /
// WorkflowTraceSource) types lifted to
// `tools/oap-code-index-enrich/src/types.rs`. The generic schema is
// now Layer 1+2 only.

// ── Diagnostics ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub warnings: Vec<Diagnostic>,
    pub errors: Vec<Diagnostic>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}
