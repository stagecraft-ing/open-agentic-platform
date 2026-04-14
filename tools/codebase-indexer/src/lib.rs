//! Library for compiling the codebase index per specs/101-codebase-index-mvp.

pub mod factory;
pub mod hash;
pub mod infra;
pub mod manifest;
pub mod render;
pub mod schema;
pub mod spec_scanner;
pub mod types;
pub mod xref;

use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use types::{
    BuildInfo, CodebaseIndex, Diagnostic, Diagnostics, INDEXER_ID, SCHEMA_VERSION,
};

// ── Error type ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum IndexError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Schema(String),
    Stale { expected: String, actual: String },
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
        }
    }
}

impl std::error::Error for IndexError {}

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

    // ── Layer 2: Spec scanning + traceability ────────────────────────────

    let specs = spec_scanner::scan_specs(repo_root);
    let (traceability, xref_diags) = xref::build_traceability(&specs, &packages);
    all_diagnostics.extend(xref_diags);

    // ── Layer 3: Factory adapters ────────────────────────────────────────

    let (factory_adapters, factory_diags) = factory::scan_adapters(repo_root);
    all_diagnostics.extend(factory_diags);

    // ── Layer 4: Infrastructure ──────────────────────────────────────────

    let infrastructure = infra::scan_infrastructure(repo_root);

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

/// Check if the existing index.json is stale.
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
                if p.is_file()
                    && p.extension()
                        .and_then(|e| e.to_str())
                        .map_or(false, |e| e == "json")
                {
                    files.push(p);
                }
            }
        }
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
            } else if p
                .extension()
                .and_then(|e| e.to_str())
                .map_or(false, |e| e == "md")
            {
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
