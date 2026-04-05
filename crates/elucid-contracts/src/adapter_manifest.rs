// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Rust types for the Elucid Adapter Manifest schema.
//!
//! An Adapter Manifest declares a technology adapter's identity, capabilities,
//! supported auth methods, commands, directory conventions, patterns, agents,
//! scaffolding, and validation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ── Top-level Adapter Manifest ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterManifest {
    pub schema_version: String,
    pub adapter: AdapterIdentity,
    pub stack: StackSpec,
    pub capabilities: Capabilities,
    pub supported_auth: Vec<SupportedAuth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supported_session_stores: Option<Vec<SessionStoreEntry>>,
    pub commands: Commands,
    pub directory_conventions: DirectoryConventions,
    pub patterns: Patterns,
    pub agents: Agents,
    pub scaffold: Scaffold,
    pub validation: Validation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dual_stack: Option<DualStack>,
}

// ── Adapter Identity ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterIdentity {
    pub name: String,
    pub display_name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ── Stack ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackSpec {
    /// e.g., "typescript", "rust", "go", "java", "csharp"
    pub language: String,
    /// e.g., "node-22", "deno-2", "bun-1", "jvm-21", "dotnet-9"
    pub runtime: String,
    pub backend: BackendSpec,
    pub frontend: FrontendSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendSpec {
    /// e.g., "express-5", "axum", "spring-boot-3", "fastapi"
    pub framework: String,
    /// How the backend is structured
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendSpec {
    /// e.g., "vue-3", "react-19", "svelte-5", "htmx", "server-rendered"
    pub framework: String,
    /// e.g., "pinia", "zustand", "signals", "none"
    pub state_management: String,
    /// e.g., "goa-web-components", "shadcn", "none"
    pub design_system: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSpec {
    /// e.g., ["postgresql", "mysql", "sqlite"]
    pub supported: Vec<String>,
    /// e.g., "custom-ddl", "prisma", "diesel", "flyway", "alembic"
    pub migration_tool: String,
    /// e.g., "none", "prisma", "diesel", "sqlalchemy", "typeorm"
    pub orm: String,
}

// ── Capabilities ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Capabilities {
    // Deployment topology
    #[serde(default)]
    pub dual_stack: bool,
    #[serde(default)]
    pub bff_pattern: bool,
    #[serde(default)]
    pub single_stack: bool,

    // Auth patterns
    #[serde(default)]
    pub session_auth: bool,
    #[serde(default)]
    pub token_auth: bool,
    #[serde(default)]
    pub api_key_auth: bool,

    // Features
    #[serde(default)]
    pub module_system: bool,
    #[serde(default)]
    pub file_uploads: bool,
    #[serde(default)]
    pub background_jobs: bool,
    #[serde(default)]
    pub realtime: bool,
    #[serde(default)]
    pub email_notifications: bool,
    #[serde(default)]
    pub audit_logging: bool,

    // Data access
    #[serde(default)]
    pub direct_sql: bool,
    #[serde(default)]
    pub orm_based: bool,
    #[serde(default)]
    pub api_proxy: bool,

    /// Adapter-specific capabilities not in the standard set.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

// ── Supported Auth ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedAuth {
    /// Matches auth.audiences.<name>.method in Build Spec
    pub method: String,
    /// Adapter's internal driver name
    pub driver: String,
    /// Supported identity providers (empty = any)
    #[serde(default)]
    pub providers: Vec<String>,
    pub description: String,
}

// ── Supported Session Stores ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStoreEntry {
    /// e.g., "redis", "postgresql", "memory", "dynamodb"
    #[serde(rename = "type")]
    pub store_type: String,
    pub description: String,
}

// ── Commands ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commands {
    pub install: String,
    pub compile: String,
    pub test: String,
    pub lint: String,
    pub dev: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format_check: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_check: Option<String>,
    /// Per-feature verification (fast checks only, not full build)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feature_verify: Vec<String>,

    /// Adapter-specific commands not in the standard set.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

// ── Directory Conventions ─────────────────────────────────────────────
//
// Template strings for file paths. Placeholders:
//   {stack}     — "api", "api-public", "api-internal", "web", "web-public", "web-internal"
//   {resource}  — Kebab-case resource name (e.g., "funding-requests")
//   {Resource}  — PascalCase (e.g., "FundingRequests")
//   {entity}    — Kebab-case entity (e.g., "funding-request")
//   {Entity}    — PascalCase entity (e.g., "FundingRequest")
//   {PageName}  — PascalCase page name (e.g., "Dashboard")
//   {org}       — Organization slug from Build Spec
//   {timestamp} — Migration timestamp (e.g., "20260327100000")
//   {name}      — Migration name (e.g., "create_organizations")

/// Directory convention templates. All fields are optional because different
/// adapters use different project structures (e.g., Encore uses services not
/// controllers). Extra adapter-specific keys are captured in `extra`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectoryConventions {
    // Backend
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_service: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_controller: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_route: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_test: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_types: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_middleware: Option<String>,

    // Frontend
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_view: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_store: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_route_config: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_test: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_component: Option<String>,

    // Data layer
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub migration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_types: Option<String>,

    // Shared
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_file: Option<String>,
    /// For dual-stack: per-stack env file paths keyed by "public" / "internal"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_file_per_stack: Option<HashMap<String, String>>,

    /// Adapter-specific conventions not covered by the standard fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

// ── Patterns ──────────────────────────────────────────────────────────
//
// Pointers to code-generation pattern files. Each pattern is a focused
// document (<200 lines) containing convention, template code, naming rules,
// and a concrete example. Scaffolding agents load ONE pattern at a time.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patterns {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<ApiPatterns>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<UiPatterns>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<DataPatterns>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_types: Option<PageTypePatterns>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiPatterns {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub middleware: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub types: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UiPatterns {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataPatterns {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub migration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_schema: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageTypePatterns {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dashboard: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub list: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub login: Option<String>,
}

// ── Agents ────────────────────────────────────────────────────────────
//
// Pointers to agent prompt files. Each agent is a focused Markdown file
// (<2K tokens) that a scaffolding agent reads before generating code.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agents {
    // Required agents
    /// Generates service + controller + route + test for one endpoint
    pub api_scaffolder: String,
    /// Generates view + state + route + test for one page
    pub ui_scaffolder: String,
    /// Generates DDL/migration + types for entities
    pub data_scaffolder: String,
    /// Applies project identity, fills env vars, wires auth
    pub configurer: String,
    /// Removes unused scaffold artifacts
    pub trimmer: String,

    // Optional agents
    /// Reviews generated code for quality/consistency
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,
    /// Checks for security issues
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_auditor: Option<String>,
}

// ── Scaffold ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scaffold {
    /// Path to base project template, relative to adapter root (e.g., "scaffold/")
    pub source: String,
    /// What the scaffold provides out of the box
    pub description: String,
    /// Which modules to install per deployment variant (variant → list of module names)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub modules: HashMap<String, Vec<String>>,
    /// Commands run after copying the scaffold, before feature scaffolding
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub setup_commands: Vec<String>,
}

// ── Validation ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validation {
    pub invariants: Vec<Invariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invariant {
    /// e.g., "INV-001"
    pub id: String,
    /// Human-readable description
    pub description: String,
    pub check: InvariantCheck,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantCheck {
    /// How to evaluate this invariant
    #[serde(rename = "type")]
    pub check_type: CheckType,
    /// Regex for grep, glob for file, or shell command
    pub pattern: String,
    /// Where to check (e.g., "apps/", "packages/", ".")
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckType {
    /// Pattern must NOT appear in codebase
    GrepAbsent,
    /// Pattern must appear in codebase
    GrepPresent,
    /// File must exist
    FileExists,
    /// File must not exist
    FileAbsent,
    /// Shell command must exit 0
    CommandSucceeds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Error,
    Warning,
}

// ── Dual Stack ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualStack {
    /// Maps audience name → stack name (e.g., "citizen" → "public")
    pub audience_to_stack: HashMap<String, String>,
    /// Named stacks with their API/web apps and ports
    pub stacks: DualStackStacks,
    /// Data flow constraints per stack name (e.g., "public" → "proxy-only")
    pub data_access: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualStackStacks {
    pub public: StackEndpoint,
    pub internal: StackEndpoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackEndpoint {
    /// API application name used in the {stack} placeholder (e.g., "api-public")
    pub api: String,
    /// Web application name used in the {stack} placeholder (e.g., "web-public")
    pub web: String,
    /// Port number for the API application
    pub port_api: u16,
    /// Port number for the web application
    pub port_web: u16,
}
