//! Adapter scopes compiler (spec 105 Phase 1).
//!
//! Reads every `factory/adapters/*/manifest.yaml`, extracts the adapter's
//! effective `file_write_scope` (top-level directories referenced in
//! `directory_conventions:`) and `allowed_commands` (unique binary names
//! from every command string under `commands:`), and emits a compiled JSON.
//!
//! Replaces `scripts/compile-adapter-scopes.js`. Output is deterministic —
//! no timestamp — so the committed artifact is stable across regenerations.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Keys under `commands:` whose scalar value is a single executable command.
/// Mirrors the JS COMMAND_KEYS set exactly.
const TOP_LEVEL_COMMAND_KEYS: &[&str] = &[
    "install",
    "compile",
    "test",
    "lint",
    "dev",
    "format_check",
    "format",
    "type_check",
    "seed",
    "migrate",
    "gen_client",
];

#[derive(Debug, Deserialize)]
struct Manifest {
    adapter: AdapterSection,
    #[serde(default)]
    commands: serde_yaml::Value,
    #[serde(default)]
    directory_conventions: serde_yaml::Value,
}

#[derive(Debug, Deserialize)]
struct AdapterSection {
    name: String,
}

#[derive(Debug, Serialize)]
pub struct AdapterScope {
    pub file_write_scope: Vec<String>,
    pub allowed_commands: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CompiledOutput {
    pub adapters: BTreeMap<String, AdapterScope>,
}

/// Discover every adapter directory that contains a `manifest.yaml`, compile
/// each, and return the combined output with adapters keyed by name.
pub fn compile_from_adapters_dir(adapters_dir: &Path) -> Result<CompiledOutput, String> {
    let mut entries: Vec<_> = fs::read_dir(adapters_dir)
        .map_err(|e| format!("reading {}: {e}", adapters_dir.display()))?
        .filter_map(Result::ok)
        .filter(|e| e.path().join("manifest.yaml").is_file())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        return Err(format!(
            "no adapter manifests under {}",
            adapters_dir.display()
        ));
    }

    let mut adapters = BTreeMap::new();
    for entry in entries {
        let manifest_path = entry.path().join("manifest.yaml");
        let text = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("reading {}: {e}", manifest_path.display()))?;
        let manifest: Manifest = serde_yaml::from_str(&text)
            .map_err(|e| format!("parsing {}: {e}", manifest_path.display()))?;

        let scope = compile_adapter(&manifest);
        adapters.insert(manifest.adapter.name, scope);
    }

    Ok(CompiledOutput { adapters })
}

fn compile_adapter(m: &Manifest) -> AdapterScope {
    let file_write_scope: Vec<String> = extract_directory_dirs(&m.directory_conventions)
        .into_iter()
        .collect();

    let mut binaries: BTreeSet<String> = BTreeSet::new();
    for cmd in extract_commands(&m.commands) {
        if let Some(bin) = first_word(&cmd) {
            binaries.insert(bin.to_string());
        }
    }

    AdapterScope {
        file_write_scope,
        allowed_commands: binaries.into_iter().collect(),
    }
}

/// Walk `commands:` and collect every executable command string.
/// - Top-level scalar values: only if the key is in TOP_LEVEL_COMMAND_KEYS.
/// - List values: every scalar item, or `command:` field from mapping items.
fn extract_commands(value: &serde_yaml::Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(mapping) = value.as_mapping() else {
        return out;
    };
    for (k, v) in mapping {
        let Some(key) = k.as_str() else { continue };
        match v {
            serde_yaml::Value::String(s) => {
                if TOP_LEVEL_COMMAND_KEYS.contains(&key) {
                    out.push(s.trim().to_string());
                }
            }
            serde_yaml::Value::Sequence(seq) => {
                for item in seq {
                    match item {
                        serde_yaml::Value::String(s) => out.push(s.trim().to_string()),
                        serde_yaml::Value::Mapping(m) => {
                            if let Some(cmd) =
                                m.get(serde_yaml::Value::String("command".into()))
                                    .and_then(|v| v.as_str())
                            {
                                out.push(cmd.trim().to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Recursively walk `directory_conventions:` and collect the set of unique
/// top-level directories (first path component + `/`), skipping values that
/// are null, empty, dotfiles, or have no `/` separator.
fn extract_directory_dirs(value: &serde_yaml::Value) -> BTreeSet<String> {
    let mut dirs = BTreeSet::new();
    collect_dirs_from(value, &mut dirs);
    dirs
}

fn collect_dirs_from(value: &serde_yaml::Value, dirs: &mut BTreeSet<String>) {
    match value {
        serde_yaml::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return;
            }
            if !trimmed.contains('/') {
                return;
            }
            let top = trimmed.split('/').next().unwrap_or("");
            if top.is_empty() || top.starts_with('.') {
                return;
            }
            dirs.insert(format!("{top}/"));
        }
        serde_yaml::Value::Mapping(m) => {
            for (_k, v) in m {
                collect_dirs_from(v, dirs);
            }
        }
        serde_yaml::Value::Sequence(seq) => {
            for v in seq {
                collect_dirs_from(v, dirs);
            }
        }
        _ => {}
    }
}

fn first_word(s: &str) -> Option<&str> {
    s.split_whitespace().next()
}

/// Serialize to JSON with 2-space indentation and a trailing newline,
/// matching the JavaScript `JSON.stringify(v, null, 2) + "\n"` shape.
pub fn serialize_to_string(output: &CompiledOutput) -> Result<String, String> {
    let mut s = serde_json::to_string_pretty(output)
        .map_err(|e| format!("serialize: {e}"))?;
    s.push('\n');
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_word_basic() {
        assert_eq!(first_word("npm install"), Some("npm"));
        assert_eq!(first_word("  npx tsc --noEmit"), Some("npx"));
        assert_eq!(first_word("cargo"), Some("cargo"));
        assert_eq!(first_word(""), None);
    }

    #[test]
    fn collect_dirs_scalar_rules() {
        let mut out = BTreeSet::new();
        collect_dirs_from(
            &serde_yaml::Value::String("apps/web/src/routes.ts".into()),
            &mut out,
        );
        assert!(out.contains("apps/"));

        out.clear();
        collect_dirs_from(&serde_yaml::Value::String(".env.example".into()), &mut out);
        assert!(out.is_empty(), "dotfiles are skipped");

        out.clear();
        collect_dirs_from(&serde_yaml::Value::String("noslash".into()), &mut out);
        assert!(out.is_empty(), "values without `/` are skipped");
    }

    #[test]
    fn extract_commands_top_level_filters_by_keys() {
        let y: serde_yaml::Value = serde_yaml::from_str(
            r#"
install: "npm install"
timeout_ms: 30000
test: "npm test"
custom_unknown: "should not appear"
"#,
        )
        .unwrap();
        let cmds = extract_commands(&y);
        assert!(cmds.contains(&"npm install".to_string()));
        assert!(cmds.contains(&"npm test".to_string()));
        assert!(!cmds.contains(&"should not appear".to_string()));
    }

    #[test]
    fn extract_commands_list_items() {
        let y: serde_yaml::Value = serde_yaml::from_str(
            r#"
feature_verify:
  - "npm run build"
  - "npm test"
pre_verify:
  - command: "npx tsc --noEmit"
    working_dir: "."
    timeout_ms: 30000
"#,
        )
        .unwrap();
        let cmds = extract_commands(&y);
        assert!(cmds.contains(&"npm run build".to_string()));
        assert!(cmds.contains(&"npm test".to_string()));
        assert!(cmds.contains(&"npx tsc --noEmit".to_string()));
    }
}
