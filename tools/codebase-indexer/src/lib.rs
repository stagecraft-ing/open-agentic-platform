//! Library for compiling the codebase index per specs/101-codebase-index-mvp.

pub mod comment_scanner;
pub mod factory;
pub mod hash;
pub mod infra;
pub mod manifest;
pub mod render;
pub mod schema;
pub mod spec_scanner;
pub mod types;
pub mod workflows;
pub mod xref;

use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use types::{BuildInfo, CodebaseIndex, Diagnostic, Diagnostics, INDEXER_ID, SCHEMA_VERSION};

// ── Error type ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum IndexError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Schema(String),
    Stale { expected: String, actual: String },
    Blocking { code: String, count: usize },
}

impl From<std::io::Error> for IndexError {
    fn from(e: std::io::Error) -> Self {
        IndexError::Io(e)
    }
}

impl From<serde_json::Error> for IndexError {
    fn from(e: serde_json::Error) -> Self {
        IndexError::Json(e)
    }
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::Io(e) => write!(f, "{e}"),
            IndexError::Json(e) => write!(f, "{e}"),
            IndexError::Schema(msg) => write!(f, "{msg}"),
            IndexError::Stale { expected, actual } => {
                write!(f, "index is stale: expected {expected}, got {actual}")
            }
            IndexError::Blocking { code, count } => {
                write!(
                    f,
                    "blocking diagnostic {code} present {count} time(s) — gate failed"
                )
            }
        }
    }
}

/// Diagnostic codes that fail `check`. Non-blocking warnings (e.g. I-101)
/// stay as informational entries in the index. Spec 118 §8 step 3
/// promotes I-105 to blocking once main reaches zero warnings.
const BLOCKING_DIAGNOSTIC_CODES: &[&str] = &["I-105"];

impl std::error::Error for IndexError {}

// ── Typed-reader API (Cut D W-11, mirror of registry-consumer W-03) ─────────

/// Errors returned by the typed reader entry point [`load`]. Separate
/// from [`IndexError`] so consumers (e.g. spec-code-coupling-check)
/// can distinguish "compile failed" from "deserialization failed" /
/// "schema version we don't recognize".
#[derive(Debug)]
pub enum IndexReaderError {
    Io(std::io::Error),
    Json(serde_json::Error),
    UnknownSchemaVersion(String),
}

impl std::fmt::Display for IndexReaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexReaderError::Io(e) => write!(f, "{e}"),
            IndexReaderError::Json(e) => write!(f, "{e}"),
            IndexReaderError::UnknownSchemaVersion(v) => {
                write!(f, "unsupported codebase-index schemaVersion: {v}")
            }
        }
    }
}

impl std::error::Error for IndexReaderError {}

impl From<std::io::Error> for IndexReaderError {
    fn from(e: std::io::Error) -> Self {
        IndexReaderError::Io(e)
    }
}

impl From<serde_json::Error> for IndexReaderError {
    fn from(e: serde_json::Error) -> Self {
        IndexReaderError::Json(e)
    }
}

/// Read a `codebase-index.json` from disk into a typed
/// [`types::CodebaseIndex`].
///
/// Peeks at `schemaVersion` to dispatch. Today only the 1.x family is
/// recognized (`schema_v1` module). W-07c bumps to 2.0.0 with its own
/// dispatch arm.
pub fn load(path: &Path) -> Result<CodebaseIndex, IndexReaderError> {
    let raw = fs::read_to_string(path)?;
    let v: Value = serde_json::from_str(&raw)?;
    let version = v
        .get("schemaVersion")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if version.starts_with("1.") {
        return schema_v1::parse(v);
    }
    Err(IndexReaderError::UnknownSchemaVersion(version))
}

mod schema_v1 {
    //! Schema 1.x dispatch arm. The crate's own `types::CodebaseIndex`
    //! is the deserialization shape — additivity within the 1.x family
    //! is handled by serde's default field handling.
    use super::*;

    pub(super) fn parse(v: Value) -> Result<CodebaseIndex, IndexReaderError> {
        let index: CodebaseIndex = serde_json::from_value(v)?;
        Ok(index)
    }
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Result of a compile: index JSON bytes (deterministic) + build-meta JSON bytes (ephemeral).
pub struct CompileOutput {
    pub index_json: Vec<u8>,
    pub build_meta_json: Vec<u8>,
}

/// Compile the codebase index and write to `build/codebase-index/`.
pub fn compile_and_write(repo_root: &Path) -> Result<CompileOutput, IndexError> {
    let out = compile(repo_root)?;
    let out_dir = repo_root.join("build/codebase-index");
    fs::create_dir_all(&out_dir)?;
    fs::write(out_dir.join("index.json"), &out.index_json)?;
    fs::write(out_dir.join("build-meta.json"), &out.build_meta_json)?;
    Ok(out)
}

/// Build index + build-meta without writing (for tests).
pub fn compile(repo_root: &Path) -> Result<CompileOutput, IndexError> {
    let indexer_version = env!("CARGO_PKG_VERSION").to_string();
    let mut all_diagnostics: Vec<Diagnostic> = Vec::new();

    // ── Layer 1: Discover and parse packages ─────────────────────────────

    let rust_toml_paths = manifest::discover_rust_crates(repo_root);
    let npm_json_paths = manifest::discover_npm_packages(repo_root);

    let mut packages: Vec<types::PackageRecord> = Vec::new();
    let mut dep_map: Vec<(String, Vec<String>)> = Vec::new();

    for toml_path in &rust_toml_paths {
        match manifest::parse_cargo_toml(toml_path, repo_root) {
            Ok(record) => {
                let deps = manifest::get_cargo_dep_names(toml_path);
                dep_map.push((record.path.clone(), deps));
                packages.push(record);
            }
            Err(e) => {
                all_diagnostics.push(Diagnostic {
                    code: "I-001".into(),
                    message: format!("failed to parse Cargo.toml: {e}"),
                    path: Some(normalize_repo_path(repo_root, toml_path)),
                });
            }
        }
    }

    for json_path in &npm_json_paths {
        match manifest::parse_package_json(json_path, repo_root) {
            Ok(record) => {
                let deps = manifest::get_npm_dep_names(json_path);
                dep_map.push((record.path.clone(), deps));
                packages.push(record);
            }
            Err(e) => {
                all_diagnostics.push(Diagnostic {
                    code: "I-002".into(),
                    message: format!("failed to parse package.json: {e}"),
                    path: Some(normalize_repo_path(repo_root, json_path)),
                });
            }
        }
    }

    // Resolve internal vs external deps
    manifest::resolve_internal_deps(&mut packages, &dep_map);

    // Sort packages by path for determinism
    packages.sort_by(|a, b| a.path.cmp(&b.path));

    // ── Layer 3: Factory adapters (before xref so adapter paths are ─────
    // ── available for implements-path validation) ────────────────────────

    let (factory_adapters, factory_diags) = factory::scan_adapters(repo_root);
    all_diagnostics.extend(factory_diags);

    let adapter_paths: std::collections::BTreeSet<String> =
        factory_adapters.iter().map(|a| a.path.clone()).collect();

    // ── Layer 2: Spec scanning + traceability ────────────────────────────

    let specs = spec_scanner::scan_specs(repo_root);

    // Comment-header scan (spec 129): walk every Rust crate path for
    // file-level `// Spec: …` annotations. Merged into the cross-reference
    // engine alongside the spec-implements + cargo-metadata sources.
    let crate_paths: Vec<String> = packages
        .iter()
        .filter(|p| matches!(
            p.kind,
            types::PackageKind::RustLib
                | types::PackageKind::RustBin
                | types::PackageKind::RustLibBin
        ))
        .map(|p| p.path.clone())
        .collect();
    let comment_headers = comment_scanner::scan_packages(repo_root, &crate_paths);

    let (traceability, xref_diags) = xref::build_traceability(
        &specs,
        &packages,
        &adapter_paths,
        &comment_headers,
        repo_root,
    );
    all_diagnostics.extend(xref_diags);

    // ── Layer 4: Infrastructure ──────────────────────────────────────────

    let infrastructure = infra::scan_infrastructure(repo_root);

    // ── Layer 5: Workflow-to-spec traceability (spec 118) ────────────────

    let wf_scan = workflows::scan_workflows(repo_root);
    all_diagnostics.extend(wf_scan.diagnostics);
    let workflow_traceability = wf_scan.traces;

    // ── Collect input files for content hash ─────────────────────────────

    let input_files = collect_input_files(repo_root, &rust_toml_paths, &npm_json_paths);
    let content_hash =
        hash::compute_content_hash(repo_root, &input_files).map_err(IndexError::Io)?;

    // ── Separate warnings and errors ─────────────────────────────────────

    all_diagnostics.sort_by(|a, b| a.code.cmp(&b.code).then_with(|| a.message.cmp(&b.message)));

    let warnings: Vec<Diagnostic> = all_diagnostics
        .iter()
        .filter(|d| !d.code.starts_with("I-0"))
        .cloned()
        .collect();
    let errors: Vec<Diagnostic> = all_diagnostics
        .iter()
        .filter(|d| d.code.starts_with("I-0"))
        .cloned()
        .collect();

    // ── Assemble ─────────────────────────────────────────────────────────

    let index = CodebaseIndex {
        schema_version: SCHEMA_VERSION.to_string(),
        build: BuildInfo {
            indexer_id: INDEXER_ID.to_string(),
            indexer_version: indexer_version.clone(),
            repo_root: ".".to_string(),
            content_hash,
        },
        inventory: packages,
        traceability,
        factory: factory_adapters,
        infrastructure,
        workflow_traceability,
        diagnostics: Diagnostics { warnings, errors },
    };

    // ── Serialize (deterministic) ────────────────────────────────────────

    let index_value = serde_json::to_value(&index)?;
    let index_json = canonical_json_bytes(&index_value)?;

    // ── Self-validate against schema (FR-09) ─────────────────────────────

    if let Err(msg) = schema::validate_against_schema(&index_json, repo_root) {
        return Err(IndexError::Schema(msg));
    }

    // ── Build meta (ephemeral) ───────────────────────────────────────────

    let build_meta = json!({
        "builtAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "indexerId": INDEXER_ID,
        "indexerVersion": indexer_version,
    });
    let build_meta_json = canonical_json_bytes(&build_meta)?;

    Ok(CompileOutput {
        index_json,
        build_meta_json,
    })
}

/// Check if the existing index.json is stale, then fail when any blocking
/// diagnostic (currently `I-105` per spec 118 §8 step 3) is present.
pub fn check(repo_root: &Path) -> Result<(), IndexError> {
    let index_path = repo_root.join("build/codebase-index/index.json");
    let raw = fs::read_to_string(&index_path)?;
    let doc: serde_json::Value = serde_json::from_str(&raw)?;

    let existing_hash = doc
        .get("build")
        .and_then(|b| b.get("contentHash"))
        .and_then(|h| h.as_str())
        .unwrap_or("")
        .to_string();

    let rust_toml_paths = manifest::discover_rust_crates(repo_root);
    let npm_json_paths = manifest::discover_npm_packages(repo_root);
    let input_files = collect_input_files(repo_root, &rust_toml_paths, &npm_json_paths);
    let current_hash =
        hash::compute_content_hash(repo_root, &input_files).map_err(IndexError::Io)?;

    if existing_hash != current_hash {
        return Err(IndexError::Stale {
            expected: current_hash,
            actual: existing_hash,
        });
    }

    // Spec 118 AC-4 / §8 step 3 — blocking diagnostics gate.
    for code in BLOCKING_DIAGNOSTIC_CODES {
        let count = doc
            .get("diagnostics")
            .and_then(|d| d.get("warnings"))
            .and_then(|w| w.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|d| d.get("code").and_then(|c| c.as_str()) == Some(*code))
                    .count()
            })
            .unwrap_or(0);
        if count > 0 {
            return Err(IndexError::Blocking {
                code: (*code).to_string(),
                count,
            });
        }
    }

    Ok(())
}

/// Diagnostic for issue #46 — print every input file's repo-relative path
/// and its normalized-content sha256 (sorted by path), so dumps from
/// different platforms can be diffed to find the first divergent line.
///
/// Output format: `<rel-path>\t<sha256-hex>\n`. Sorted lexicographically by
/// path.  Stable across runs on the same platform; expected to be stable
/// across platforms (the bug we're trying to find).
pub fn dump_inputs(repo_root: &Path) -> Result<(), IndexError> {
    use sha2::{Digest, Sha256};
    let rust_toml_paths = manifest::discover_rust_crates(repo_root);
    let npm_json_paths = manifest::discover_npm_packages(repo_root);
    let mut input_files = collect_input_files(repo_root, &rust_toml_paths, &npm_json_paths);
    input_files.sort();
    input_files.dedup();

    let mut lines: Vec<(String, String)> = Vec::new();
    for path in &input_files {
        if !path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(path)?;
        // Apply the same normalization as compute_content_hash: strip BOM,
        // CRLF/CR -> LF.
        let normalized = raw
            .strip_prefix('\u{feff}')
            .unwrap_or(&raw)
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        let digest = format!("{:x}", hasher.finalize());
        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        lines.push((rel, digest));
    }

    // Sort exactly the same way compute_content_hash sorts.
    lines.sort_by(|a, b| a.0.cmp(&b.0));

    for (rel, digest) in lines {
        println!("{rel}\t{digest}");
    }
    Ok(())
}

/// Render CODEBASE-INDEX.md from existing index.json.
pub fn render_to_file(repo_root: &Path) -> Result<(), IndexError> {
    let index_path = repo_root.join("build/codebase-index/index.json");
    let raw = fs::read_to_string(&index_path)?;
    let index: CodebaseIndex = serde_json::from_str(&raw)?;
    let markdown = render::render_markdown(&index);
    let out_path = repo_root.join("build/codebase-index/CODEBASE-INDEX.md");
    fs::write(out_path, markdown)?;
    Ok(())
}

// ── Internal helpers ────────────────────────────────────────────────────────

/// Collect all input files that contribute to the content hash.
fn collect_input_files(
    repo_root: &Path,
    rust_tomls: &[PathBuf],
    npm_jsons: &[PathBuf],
) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();

    // Cargo.toml files
    files.extend_from_slice(rust_tomls);

    // Workspace Cargo.toml
    let ws = repo_root.join("crates/Cargo.toml");
    if ws.is_file() {
        files.push(ws);
    }

    // package.json files
    files.extend_from_slice(npm_jsons);

    // pnpm-workspace.yaml
    let pnpm_ws = repo_root.join("pnpm-workspace.yaml");
    if pnpm_ws.is_file() {
        files.push(pnpm_ws);
    }

    // Spec files
    let specs_dir = repo_root.join("specs");
    if specs_dir.is_dir() {
        if let Ok(dir) = fs::read_dir(&specs_dir) {
            for ent in dir.flatten() {
                let p = ent.path();
                if p.is_dir() {
                    let spec_md = p.join("spec.md");
                    if spec_md.is_file() {
                        files.push(spec_md);
                    }
                }
            }
        }
    }

    // Factory adapter manifests
    let adapters_dir = repo_root.join("factory/adapters");
    if adapters_dir.is_dir() {
        if let Ok(dir) = fs::read_dir(&adapters_dir) {
            for ent in dir.flatten() {
                let manifest = ent.path().join("manifest.yaml");
                if manifest.is_file() {
                    files.push(manifest);
                }
            }
        }
    }

    // Factory process stages
    let stages_dir = repo_root.join("factory/process/stages");
    if stages_dir.is_dir() {
        if let Ok(dir) = fs::read_dir(&stages_dir) {
            for ent in dir.flatten() {
                let p = ent.path();
                if p.is_file() {
                    files.push(p);
                }
            }
        }
    }

    // .claude/ agents, commands, rules
    for subdir in &[".claude/agents", ".claude/commands", ".claude/rules"] {
        let dir = repo_root.join(subdir);
        if dir.is_dir() {
            collect_md_files(&dir, &mut files);
        }
    }

    // schemas/
    let schemas_dir = repo_root.join("schemas");
    if schemas_dir.is_dir() {
        if let Ok(dir) = fs::read_dir(&schemas_dir) {
            for ent in dir.flatten() {
                let p = ent.path();
                if p.is_file() && (p.extension().and_then(|e| e.to_str()) == Some("json")) {
                    files.push(p);
                }
            }
        }
    }

    // .github/workflows/ (spec 118 — header changes affect Layer 5).
    let workflows_dir = repo_root.join(".github/workflows");
    if workflows_dir.is_dir() {
        if let Ok(dir) = fs::read_dir(&workflows_dir) {
            for ent in dir.flatten() {
                let p = ent.path();
                let ext = p.extension().and_then(|e| e.to_str());
                if p.is_file() && (ext == Some("yml") || ext == Some("yaml")) {
                    files.push(p);
                }
            }
        }
    }

    // Workflow allowlist (spec 118).
    let allowlist = repo_root.join("tools/codebase-indexer/workflow-allowlist.toml");
    if allowlist.is_file() {
        files.push(allowlist);
    }

    files.sort();
    files.dedup();
    files
}

fn collect_md_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for ent in entries.flatten() {
            let p = ent.path();
            if p.is_dir() {
                collect_md_files(&p, files);
            } else if p.extension().and_then(|e| e.to_str()) == Some("md") {
                files.push(p);
            }
        }
    }
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, IndexError> {
    let sorted = sort_json_value(value.clone());
    let s = serde_json::to_string_pretty(&sorted)?;
    Ok(s.into_bytes())
}

fn sort_json_value(v: Value) -> Value {
    match v {
        Value::Object(map) => {
            let mut out: BTreeMap<String, Value> = BTreeMap::new();
            for (k, val) in map {
                out.insert(k, sort_json_value(val));
            }
            let mut m = Map::new();
            for (k, v) in out {
                m.insert(k, v);
            }
            Value::Object(m)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(sort_json_value).collect()),
        other => other,
    }
}

fn normalize_repo_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod typed_reader_tests {
    use super::*;

    fn minimal_fixture() -> String {
        // schemaVersion explicit; the rest is the minimum required by
        // CodebaseIndex's required (non-default) fields.
        format!(
            r#"{{
                "schemaVersion": "{SCHEMA_VERSION}",
                "build": {{
                    "indexerId": "codebase-indexer",
                    "indexerVersion": "0.1.0",
                    "repoRoot": "/tmp/x",
                    "contentHash": "0"
                }},
                "inventory": [],
                "traceability": {{ "mappings": [], "orphanedSpecs": [], "untracedCode": [] }},
                "factory": [],
                "infrastructure": {{ "tools": [], "agents": [], "commands": [], "rules": [], "schemas": [] }},
                "diagnostics": {{ "warnings": [], "errors": [] }}
            }}"#
        )
    }

    fn write_fixture(body: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("index.json");
        fs::write(&p, body).unwrap();
        (dir, p)
    }

    #[test]
    fn load_accepts_current_schema_version() {
        let (_d, p) = write_fixture(&minimal_fixture());
        let idx = load(&p).expect("load typed index");
        assert_eq!(idx.schema_version, SCHEMA_VERSION);
        assert!(idx.inventory.is_empty());
        assert!(idx.traceability.mappings.is_empty());
    }

    #[test]
    fn load_rejects_unknown_schema_version() {
        let body = minimal_fixture().replace(SCHEMA_VERSION, "9.9.9");
        let (_d, p) = write_fixture(&body);
        match load(&p) {
            Ok(_) => panic!("unknown schema must reject"),
            Err(IndexReaderError::UnknownSchemaVersion(_)) => {}
            Err(other) => panic!("expected UnknownSchemaVersion, got {other}"),
        }
    }

    #[test]
    fn load_decodes_traceability_mappings() {
        let body = format!(
            r#"{{
                "schemaVersion": "{SCHEMA_VERSION}",
                "build": {{ "indexerId": "x", "indexerVersion": "0", "repoRoot": ".", "contentHash": "0" }},
                "inventory": [],
                "traceability": {{
                    "mappings": [
                        {{
                            "specId": "127-spec-code-coupling-gate",
                            "implementingPaths": [
                                {{ "path": "tools/spec-code-coupling-check" }},
                                {{ "path": "Makefile", "primary": true }}
                            ]
                        }}
                    ],
                    "orphanedSpecs": [],
                    "untracedCode": []
                }},
                "factory": [],
                "infrastructure": {{ "tools": [], "agents": [], "commands": [], "rules": [], "schemas": [] }},
                "diagnostics": {{ "warnings": [], "errors": [] }}
            }}"#
        );
        let (_d, p) = write_fixture(&body);
        let idx = load(&p).unwrap();
        assert_eq!(idx.traceability.mappings.len(), 1);
        let m = &idx.traceability.mappings[0];
        assert_eq!(m.spec_id, "127-spec-code-coupling-gate");
        assert_eq!(m.implementing_paths.len(), 2);
        assert_eq!(m.implementing_paths[0].path, "tools/spec-code-coupling-check");
        assert_eq!(m.implementing_paths[1].primary, Some(true));
    }

    #[test]
    fn load_surfaces_io_errors_for_missing_file() {
        let path = std::path::PathBuf::from("/nonexistent/path/index.json");
        match load(&path) {
            Ok(_) => panic!("missing file must fail"),
            Err(IndexReaderError::Io(_)) => {}
            Err(other) => panic!("expected Io, got {other}"),
        }
    }
}
