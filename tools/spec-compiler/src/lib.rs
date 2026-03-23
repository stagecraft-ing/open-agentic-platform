//! Library for compiling `specs/*/spec.md` into Feature 000 registry JSON.

use serde::Serialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const COMPILER_ID: &str = "open-agentic-spec-compiler";
const SPEC_VERSION: &str = "1.0.0";

/// Known frontmatter keys consumed into normalized fields (remainder → extraFrontmatter).
const KNOWN_KEYS: &[&str] = &[
    "id",
    "title",
    "status",
    "created",
    "summary",
    "authors",
    "kind",
    "feature_branch",
];

#[derive(Debug)]
pub enum CompileError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
    Json(serde_json::Error),
    MissingFrontmatter { path: PathBuf },
    InvalidFrontmatter { path: PathBuf, msg: String },
}

impl From<std::io::Error> for CompileError {
    fn from(e: std::io::Error) -> Self {
        CompileError::Io(e)
    }
}

impl From<serde_yaml::Error> for CompileError {
    fn from(e: serde_yaml::Error) -> Self {
        CompileError::Yaml(e)
    }
}

impl From<serde_json::Error> for CompileError {
    fn from(e: serde_json::Error) -> Self {
        CompileError::Json(e)
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::Io(e) => write!(f, "{e}"),
            CompileError::Yaml(e) => write!(f, "{e}"),
            CompileError::Json(e) => write!(f, "{e}"),
            CompileError::MissingFrontmatter { path } => {
                write!(f, "missing YAML frontmatter: {}", path.display())
            }
            CompileError::InvalidFrontmatter { path, msg } => {
                write!(f, "{}: {msg}", path.display())
            }
        }
    }
}

impl std::error::Error for CompileError {}

/// Result of a compile: registry JSON bytes (deterministic) + build-meta JSON bytes (ephemeral).
pub struct CompileOutput {
    pub registry_json: Vec<u8>,
    pub build_meta_json: Vec<u8>,
    pub validation_passed: bool,
}

/// Run compilation from `repo_root` (must be the repository root). Writes to `build/spec-registry/`.
pub fn compile_and_write(repo_root: &Path) -> Result<CompileOutput, CompileError> {
    let out = compile(repo_root)?;
    let out_dir = repo_root.join("build/spec-registry");
    fs::create_dir_all(&out_dir)?;
    fs::write(out_dir.join("registry.json"), &out.registry_json)?;
    fs::write(out_dir.join("build-meta.json"), &out.build_meta_json)?;
    Ok(out)
}

/// Build registry + build-meta without writing (for tests).
pub fn compile(repo_root: &Path) -> Result<CompileOutput, CompileError> {
    let compiler_version = env!("CARGO_PKG_VERSION").to_string();
    let mut violations: Vec<Violation> = Vec::new();

    let spec_paths = discover_spec_paths(repo_root)?;
    for dir in missing_spec_md_dirs(repo_root)? {
        violations.push(Violation {
            code: "V-001".to_string(),
            severity: "error".to_string(),
            message: "spec.md missing for feature directory".to_string(),
            path: Some(normalize_repo_path(repo_root, &dir)),
        });
    }

    yaml_violations(repo_root, &mut violations);

    let mut features: Vec<FeatureRecord> = Vec::new();
    let mut seen_ids: BTreeMap<String, PathBuf> = BTreeMap::new();

    for spec_path in &spec_paths {
        let raw = fs::read_to_string(spec_path)?;
        let (yaml_val, body): (serde_yaml::Value, String) = split_frontmatter(&raw, spec_path)?;

        let fm = yaml_val
            .as_mapping()
            .ok_or_else(|| CompileError::InvalidFrontmatter {
                path: spec_path.clone(),
                msg: "frontmatter must be a mapping".into(),
            })?;

        let id = required_str(fm, "id", spec_path)?;
        let title = required_str(fm, "title", spec_path)?;
        let status = required_str(fm, "status", spec_path)?;
        let created = required_str(fm, "created", spec_path)?;
        let summary = required_str(fm, "summary", spec_path)?;

        if let Some(prev) = seen_ids.get(&id) {
            violations.push(Violation {
                code: "V-003".to_string(),
                severity: "error".to_string(),
                message: format!("duplicate feature id {id:?}"),
                path: Some(normalize_repo_path(repo_root, spec_path)),
            });
            violations.push(Violation {
                code: "V-003".to_string(),
                severity: "error".to_string(),
                message: format!("duplicate feature id {id:?} (first occurrence)"),
                path: Some(normalize_repo_path(repo_root, prev)),
            });
            continue;
        }
        seen_ids.insert(id.clone(), spec_path.clone());

        let rel = normalize_repo_path(repo_root, spec_path);
        let authors = optional_string_list(fm, "authors");
        let kind = optional_str(fm, "kind");
        let feature_branch = optional_str(fm, "feature_branch");
        let extra = extra_frontmatter(repo_root, fm, spec_path, &mut violations)?;

        let headings = extract_headings(&body, &title);

        features.push(FeatureRecord {
            id,
            title,
            status,
            created,
            summary,
            spec_path: rel,
            section_headings: headings,
            authors,
            kind,
            feature_branch,
            extra_frontmatter: extra,
        });
    }

    // V-002: required keys checked above; extra invalid types add violations in extra_frontmatter

    features.sort_by(|a, b| a.id.cmp(&b.id));

    let passed = !violations.iter().any(|v| v.severity == "error");

    let content_hash = compute_content_hash(repo_root, &spec_paths)?;

    let registry_value = build_registry_value(
        &compiler_version,
        content_hash,
        &features,
        passed,
        &violations,
    )?;

    let registry_json = canonical_json_bytes(&registry_value)?;

    let built_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let build_meta = json!({
        "builtAt": built_at,
        "compilerId": COMPILER_ID,
        "compilerVersion": compiler_version,
    });
    let build_meta_json = canonical_json_bytes(&build_meta)?;

    Ok(CompileOutput {
        registry_json,
        build_meta_json,
        validation_passed: passed,
    })
}

#[derive(Serialize)]
struct FeatureRecord {
    id: String,
    title: String,
    status: String,
    created: String,
    summary: String,
    #[serde(rename = "specPath")]
    spec_path: String,
    #[serde(rename = "sectionHeadings")]
    section_headings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(rename = "featureBranch", skip_serializing_if = "Option::is_none")]
    feature_branch: Option<String>,
    #[serde(rename = "extraFrontmatter", skip_serializing_if = "Option::is_none")]
    extra_frontmatter: Option<Map<String, Value>>,
}

#[derive(Clone, Serialize)]
struct Violation {
    code: String,
    severity: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

fn build_registry_value(
    compiler_version: &str,
    content_hash: String,
    features: &[FeatureRecord],
    passed: bool,
    violations: &[Violation],
) -> Result<Value, CompileError> {
    let mut viol: Vec<Violation> = violations.to_vec();
    viol.sort_by(|a, b| {
        a.code
            .cmp(&b.code)
            .then_with(|| a.message.cmp(&b.message))
    });

    let features_val = serde_json::to_value(features)?;
    let viol_val = serde_json::to_value(&viol)?;

    Ok(json!({
        "specVersion": SPEC_VERSION,
        "build": {
            "compilerId": COMPILER_ID,
            "compilerVersion": compiler_version,
            "inputRoot": ".",
            "contentHash": content_hash,
        },
        "features": features_val,
        "validation": {
            "passed": passed,
            "violations": viol_val,
        }
    }))
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, CompileError> {
    let sorted = sort_json_value(value.clone());
    let s = serde_json::to_string(&sorted)?;
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

fn compute_content_hash(repo_root: &Path, spec_paths: &[PathBuf]) -> Result<String, CompileError> {
    let mut pieces: Vec<(String, Vec<u8>)> = Vec::new();
    for p in spec_paths {
        let raw = fs::read_to_string(p)?;
        let normalized = normalize_text(&raw);
        let rel = normalize_repo_path(repo_root, p);
        let mut buf = rel.as_bytes().to_vec();
        buf.push(0);
        buf.extend_from_slice(&normalized);
        pieces.push((rel, buf));
    }
    pieces.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (_, buf) in pieces {
        hasher.update(&buf);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn normalize_text(s: &str) -> Vec<u8> {
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let s = s.replace("\r\n", "\n").replace('\r', "\n");
    s.into_bytes()
}

fn normalize_repo_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn discover_spec_paths(repo_root: &Path) -> Result<Vec<PathBuf>, CompileError> {
    let specs = repo_root.join("specs");
    if !specs.is_dir() {
        return Ok(vec![]);
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    for ent in fs::read_dir(&specs)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_dir() {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name
            .chars()
            .take(3)
            .all(|c| c.is_ascii_digit())
        {
            continue;
        }
        let spec_md = p.join("spec.md");
        if spec_md.is_file() {
            paths.push(spec_md);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Directories under specs/NNN-* that exist but lack spec.md (V-001).
fn missing_spec_md_dirs(repo_root: &Path) -> Result<Vec<PathBuf>, CompileError> {
    let specs = repo_root.join("specs");
    if !specs.is_dir() {
        return Ok(vec![]);
    }
    let mut missing = Vec::new();
    for ent in fs::read_dir(&specs)? {
        let p = ent?.path();
        if !p.is_dir() {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.len() < 5 || !name.chars().take(3).all(|c| c.is_ascii_digit()) {
            continue;
        }
        if !p.join("spec.md").is_file() {
            missing.push(p);
        }
    }
    missing.sort();
    Ok(missing)
}

/// Standalone `.yaml` / `.yml` under the repo are rejected (V-004). Skipped path
/// components match Feature 001 research; a future spec may add an explicit allowlist
/// for vendored or fixture YAML outside these directories.
fn yaml_violations(repo_root: &Path, violations: &mut Vec<Violation>) {
    let skip = |p: &Path| {
        p.components().any(|c| {
            matches!(
                c.as_os_str().to_str(),
                Some(
                    ".git" | "build" | "node_modules" | "vendor" | "target" | ".idea"
                )
            )
        })
    };
    for ent in WalkDir::new(repo_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        if skip(p) {
            continue;
        }
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "yaml" || ext == "yml" {
            violations.push(Violation {
                code: "V-004".to_string(),
                severity: "error".to_string(),
                message: "standalone authored YAML file is forbidden".to_string(),
                path: Some(normalize_repo_path(repo_root, p)),
            });
        }
    }
}

fn split_frontmatter(raw: &str, path: &Path) -> Result<(serde_yaml::Value, String), CompileError> {
    let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
    let rest = raw
        .strip_prefix("---")
        .ok_or_else(|| CompileError::MissingFrontmatter {
            path: path.to_path_buf(),
        })?;
    let rest = rest
        .strip_prefix('\n')
        .or_else(|| rest.strip_prefix("\r\n"))
        .ok_or_else(|| CompileError::MissingFrontmatter {
            path: path.to_path_buf(),
        })?;

    let (yaml_str, body) = if let Some(i) = rest.find("\n---\n") {
        (&rest[..i], rest[i + 5..].to_string())
    } else if let Some(i) = rest.find("\r\n---\r\n") {
        (&rest[..i], rest[i + 7..].to_string())
    } else {
        return Err(CompileError::MissingFrontmatter {
            path: path.to_path_buf(),
        });
    };

    let v: serde_yaml::Value = serde_yaml::from_str(yaml_str)?;
    Ok((v, body))
}

fn required_str(m: &serde_yaml::Mapping, key: &str, path: &Path) -> Result<String, CompileError> {
    let v = m.get(key).ok_or_else(|| CompileError::InvalidFrontmatter {
        path: path.to_path_buf(),
        msg: format!("missing required key {key:?}"),
    })?;
    v.as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| CompileError::InvalidFrontmatter {
            path: path.to_path_buf(),
            msg: format!("key {key:?} must be a string"),
        })
}

fn optional_str(m: &serde_yaml::Mapping, key: &str) -> Option<String> {
    m.get(key)?.as_str().map(|s| s.to_string())
}

fn optional_string_list(m: &serde_yaml::Mapping, key: &str) -> Option<Vec<String>> {
    let v = m.get(key)?;
    let arr = v.as_sequence()?;
    let mut out = Vec::new();
    for x in arr {
        out.push(x.as_str()?.to_string());
    }
    Some(out)
}

fn extra_frontmatter(
    repo_root: &Path,
    m: &serde_yaml::Mapping,
    path: &Path,
    violations: &mut Vec<Violation>,
) -> Result<Option<Map<String, Value>>, CompileError> {
    let mut extra = Map::new();
    for (k, v) in m.iter() {
        let key = k.as_str().ok_or_else(|| CompileError::InvalidFrontmatter {
            path: path.to_path_buf(),
            msg: "frontmatter keys must be strings".into(),
        })?;
        if KNOWN_KEYS.contains(&key) {
            continue;
        }
        match yaml_scalar_to_json(v) {
            Some(j) => {
                extra.insert(key.to_string(), j);
            }
            None => {
                violations.push(Violation {
                    code: "V-002".to_string(),
                    severity: "error".to_string(),
                    message: format!(
                        "frontmatter key {key:?} has a value that cannot be represented in extraFrontmatter"
                    ),
                    path: Some(normalize_repo_path(repo_root, path)),
                });
            }
        }
    }
    if extra.len() > 8 {
        violations.push(Violation {
            code: "V-002".to_string(),
            severity: "error".to_string(),
            message: "extraFrontmatter exceeds maxProperties (8)".into(),
            path: Some(normalize_repo_path(repo_root, path)),
        });
    }
    if extra.is_empty() {
        Ok(None)
    } else {
        Ok(Some(extra))
    }
}

fn yaml_scalar_to_json(v: &serde_yaml::Value) -> Option<Value> {
    match v {
        serde_yaml::Value::String(s) => Some(Value::String(s.clone())),
        serde_yaml::Value::Bool(b) => Some(Value::Bool(*b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Some(Value::Number(i.into()));
            }
            let f = n.as_f64()?;
            Some(Value::Number(serde_json::Number::from_f64(f)?))
        }
        serde_yaml::Value::Null => Some(Value::Null),
        serde_yaml::Value::Sequence(seq) => {
            let mut arr = Vec::new();
            for x in seq {
                arr.push(x.as_str()?.to_string());
            }
            if arr.len() > 64 {
                return None;
            }
            Some(Value::Array(arr.into_iter().map(Value::String).collect()))
        }
        serde_yaml::Value::Mapping(_) | serde_yaml::Value::Tagged(_) => None,
    }
}

/// ATX `#` / `##` headings only; first heading equal to `title` is dropped (see README).
pub fn extract_headings(body: &str, title: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        let t = line.trim_start();
        if let Some(h) = atx_h2(t) {
            out.push(h.to_string());
            continue;
        }
        if let Some(h) = atx_h1(t) {
            out.push(h.to_string());
        }
    }
    if let Some(first) = out.first() {
        if first.trim() == title.trim() {
            out.remove(0);
        }
    }
    out
}

fn atx_h1(line: &str) -> Option<&str> {
    if !line.starts_with('#') {
        return None;
    }
    if line.starts_with("##") {
        return None;
    }
    line.strip_prefix("# ").map(str::trim_end)
}

fn atx_h2(line: &str) -> Option<&str> {
    if !line.starts_with("##") {
        return None;
    }
    if line.starts_with("###") {
        return None;
    }
    line.strip_prefix("## ").map(str::trim_end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_skip_title_duplicate() {
        let body = "# Feature X\n\n## A\n## B\n";
        let h = extract_headings(body, "Feature X");
        assert_eq!(h, vec!["A", "B"]);
    }
}
