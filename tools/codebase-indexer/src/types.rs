//! Core data types mapping to `schemas/codebase-index.schema.json`.

use serde::{Deserialize, Serialize};

/// Schema version — compile-time contract between the indexer and the JSON Schema.
pub const SCHEMA_VERSION: &str = "1.0.0";
pub const INDEXER_ID: &str = "codebase-indexer";

// ── Top-level output ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodebaseIndex {
    pub schema_version: String,
    pub build: BuildInfo,
    pub inventory: Vec<PackageRecord>,
    pub traceability: Traceability,
    pub factory: Vec<AdapterRecord>,
    pub infrastructure: Infrastructure,
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
    pub implementing_paths: Vec<ImplementingPath>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementingPath {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<TraceSource>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TraceSource {
    SpecImplements,
    CargoMetadata,
    Both,
}

// ── Layer 3: Factory Adapter Inventory ──────────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterRecord {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_stack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_coverage: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_runtime: Option<String>,
}

// ── Layer 4: Tool & Infrastructure Inventory ────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Infrastructure {
    pub tools: Vec<ToolEntry>,
    pub agents: Vec<NamedEntry>,
    pub commands: Vec<NamedEntry>,
    pub rules: Vec<NamedEntry>,
    pub schemas: Vec<NamedEntry>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolEntry {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binaries: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedEntry {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

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
