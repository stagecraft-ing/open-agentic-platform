//! Cargo.toml and package.json parsers (Layer 1).

use crate::types::{PackageKind, PackageRecord};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Errors specific to manifest parsing.
#[derive(Debug)]
pub enum ManifestError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    Json(serde_json::Error),
    Yaml(serde_yaml::Error),
    Missing { path: PathBuf, field: String },
}

impl From<std::io::Error> for ManifestError {
    fn from(e: std::io::Error) -> Self {
        ManifestError::Io(e)
    }
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManifestError::Io(e) => write!(f, "IO: {e}"),
            ManifestError::Toml(e) => write!(f, "TOML: {e}"),
            ManifestError::Json(e) => write!(f, "JSON: {e}"),
            ManifestError::Yaml(e) => write!(f, "YAML: {e}"),
            ManifestError::Missing { path, field } => {
                write!(f, "{}: missing field {field:?}", path.display())
            }
        }
    }
}

/// Parse a single Cargo.toml into a PackageRecord.
pub fn parse_cargo_toml(path: &Path, repo_root: &Path) -> Result<PackageRecord, ManifestError> {
    let raw = fs::read_to_string(path)?;
    let doc: toml::Value = raw.parse().map_err(ManifestError::Toml)?;

    let pkg = doc.get("package").and_then(|v| v.as_table());
    let name = pkg
        .and_then(|t| t.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| ManifestError::Missing {
            path: path.to_path_buf(),
            field: "package.name".into(),
        })?
        .to_string();

    let version = pkg
        .and_then(|t| t.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let edition = pkg
        .and_then(|t| t.get("edition"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Determine kind from [lib] and [[bin]] presence
    let dir = path.parent().unwrap_or(Path::new("."));
    let has_lib = doc.get("lib").is_some() || dir.join("src/lib.rs").is_file();
    let bin_targets: Vec<String> = if let Some(bins) = doc.get("bin").and_then(|v| v.as_array()) {
        bins.iter()
            .filter_map(|b| {
                b.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .collect()
    } else if dir.join("src/main.rs").is_file() {
        vec![name.clone()]
    } else {
        vec![]
    };
    let has_bin = !bin_targets.is_empty();

    let kind = match (has_lib, has_bin) {
        (true, true) => PackageKind::RustLibBin,
        (true, false) => PackageKind::RustLib,
        (false, true) => PackageKind::RustBin,
        (false, false) => PackageKind::RustLib, // default to lib
    };

    // Entry points
    let mut entry_points = Vec::new();
    if has_lib {
        let lib_path = doc
            .get("lib")
            .and_then(|v| v.get("path"))
            .and_then(|v| v.as_str())
            .unwrap_or("src/lib.rs");
        entry_points.push(lib_path.to_string());
    }
    if let Some(bins) = doc.get("bin").and_then(|v| v.as_array()) {
        for b in bins {
            if let Some(p) = b.get("path").and_then(|v| v.as_str()) {
                entry_points.push(p.to_string());
            }
        }
    } else if has_bin {
        entry_points.push("src/main.rs".to_string());
    }

    // [package.metadata.oap].spec
    let spec_ref = pkg
        .and_then(|t| t.get("metadata"))
        .and_then(|v| v.get("oap"))
        .and_then(|v| v.get("spec"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let rel_path = normalize_path(repo_root, dir);

    Ok(PackageRecord {
        name,
        path: rel_path,
        kind,
        version,
        edition,
        entry_points: if entry_points.is_empty() {
            None
        } else {
            Some(entry_points)
        },
        internal_deps: None, // resolved later by resolve_internal_deps
        external_deps: None, // resolved later
        spec_ref,
        // store raw dep names temporarily for resolution
    })
}

/// Extract dependency names from a Cargo.toml section.
fn extract_dep_names(doc: &toml::Value, section: &str) -> Vec<String> {
    let Some(deps) = doc.get(section).and_then(|v| v.as_table()) else {
        return vec![];
    };
    let mut names: Vec<String> = Vec::new();
    for (key, val) in deps {
        // The dep name is the key, unless there's a `package` rename
        let actual_name = val
            .get("package")
            .and_then(|v| v.as_str())
            .unwrap_or(key.as_str());
        names.push(actual_name.to_string());
    }
    names.sort();
    names
}

/// Get all dependency names for a Cargo.toml (for resolution).
pub fn get_cargo_dep_names(path: &Path) -> Vec<String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return vec![];
    };
    let Ok(doc) = raw.parse::<toml::Value>() else {
        return vec![];
    };
    extract_dep_names(&doc, "dependencies")
}

/// Parse a workspace Cargo.toml to get member list.
pub fn parse_workspace_members(path: &Path) -> Result<Vec<String>, ManifestError> {
    let raw = fs::read_to_string(path)?;
    let doc: toml::Value = raw.parse().map_err(ManifestError::Toml)?;
    let members = doc
        .get("workspace")
        .and_then(|v| v.get("members"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(members)
}

/// Parse a single package.json into a PackageRecord.
pub fn parse_package_json(path: &Path, repo_root: &Path) -> Result<PackageRecord, ManifestError> {
    let raw = fs::read_to_string(path)?;
    let doc: serde_json::Value = serde_json::from_str(&raw).map_err(ManifestError::Json)?;

    let name = doc
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let version = doc
        .get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let has_workspaces = doc.get("workspaces").is_some();
    let kind = if has_workspaces {
        PackageKind::NpmWorkspace
    } else {
        PackageKind::NpmPackage
    };

    // Entry points
    let mut entry_points = Vec::new();
    for key in &["main", "module"] {
        if let Some(v) = doc.get(*key).and_then(|v| v.as_str()) {
            entry_points.push(v.to_string());
        }
    }

    let dir = path.parent().unwrap_or(Path::new("."));
    let rel_path = normalize_path(repo_root, dir);

    // [oap].spec
    let spec_ref = doc
        .get("oap")
        .and_then(|v| v.get("spec"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(PackageRecord {
        name,
        path: rel_path,
        kind,
        version,
        edition: None,
        entry_points: if entry_points.is_empty() {
            None
        } else {
            Some(entry_points)
        },
        internal_deps: None,
        external_deps: None,
        spec_ref,
    })
}

fn extract_json_dep_names(doc: &serde_json::Value, section: &str) -> Vec<String> {
    let Some(deps) = doc.get(section).and_then(|v| v.as_object()) else {
        return vec![];
    };
    let mut names: Vec<String> = deps.keys().cloned().collect();
    names.sort();
    names
}

/// Get all dependency names for a package.json (for resolution).
pub fn get_npm_dep_names(path: &Path) -> Vec<String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return vec![];
    };
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return vec![];
    };
    let mut names = extract_json_dep_names(&doc, "dependencies");
    names.extend(extract_json_dep_names(&doc, "devDependencies"));
    names.sort();
    names.dedup();
    names
}

/// Resolve internal vs external deps for all packages.
pub fn resolve_internal_deps(packages: &mut [PackageRecord], dep_map: &[(String, Vec<String>)]) {
    let known_names: BTreeSet<String> = packages.iter().map(|p| p.name.clone()).collect();

    for pkg in packages.iter_mut() {
        let Some(deps) = dep_map.iter().find(|(path, _)| path == &pkg.path) else {
            continue;
        };
        let mut internal = Vec::new();
        let mut external = Vec::new();
        for dep in &deps.1 {
            if known_names.contains(dep) {
                internal.push(dep.clone());
            } else {
                external.push(dep.clone());
            }
        }
        internal.sort();
        external.sort();
        pkg.internal_deps = if internal.is_empty() {
            None
        } else {
            Some(internal)
        };
        pkg.external_deps = if external.is_empty() {
            None
        } else {
            Some(external)
        };
    }
}

/// Discover all Rust crate Cargo.toml files in the repo.
pub fn discover_rust_crates(repo_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // crates/ workspace members
    let workspace_toml = repo_root.join("crates/Cargo.toml");
    if workspace_toml.is_file() {
        if let Ok(members) = parse_workspace_members(&workspace_toml) {
            for m in members {
                let p = repo_root.join("crates").join(&m).join("Cargo.toml");
                if p.is_file() {
                    paths.push(p);
                }
            }
        }
    }

    // tools/*/ (each independent, skip tools/shared/)
    if let Ok(entries) = fs::read_dir(repo_root.join("tools")) {
        for ent in entries.flatten() {
            let p = ent.path();
            if !p.is_dir() {
                continue;
            }
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "shared" {
                continue;
            }
            let toml_path = p.join("Cargo.toml");
            if toml_path.is_file() {
                paths.push(toml_path);
            }
        }
    }

    // tools/shared/*/
    let shared = repo_root.join("tools/shared");
    if shared.is_dir() {
        if let Ok(entries) = fs::read_dir(&shared) {
            for ent in entries.flatten() {
                let p = ent.path();
                if p.is_dir() {
                    let toml_path = p.join("Cargo.toml");
                    if toml_path.is_file() {
                        paths.push(toml_path);
                    }
                }
            }
        }
    }

    // grammars/*/
    let grammars = repo_root.join("grammars");
    if grammars.is_dir() {
        if let Ok(entries) = fs::read_dir(&grammars) {
            for ent in entries.flatten() {
                let p = ent.path();
                if p.is_dir() {
                    let toml_path = p.join("Cargo.toml");
                    if toml_path.is_file() {
                        paths.push(toml_path);
                    }
                }
            }
        }
    }

    // platform/services/deployd-api-rs/
    let deployd = repo_root.join("platform/services/deployd-api-rs/Cargo.toml");
    if deployd.is_file() {
        paths.push(deployd);
    }

    paths.sort();
    paths
}

/// Discover all npm package.json files in the repo.
pub fn discover_npm_packages(repo_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Read pnpm-workspace.yaml for workspace globs
    let ws_yaml = repo_root.join("pnpm-workspace.yaml");
    if ws_yaml.is_file() {
        if let Ok(raw) = fs::read_to_string(&ws_yaml) {
            if let Ok(doc) = serde_yaml::from_str::<serde_yaml::Value>(&raw) {
                if let Some(pkgs) = doc.get("packages").and_then(|v| v.as_sequence()) {
                    for glob in pkgs {
                        if let Some(pattern) = glob.as_str() {
                            // Pattern is like "apps/*" or "packages/*"
                            let base = pattern.trim_end_matches("/*").trim_end_matches("/**");
                            let dir = repo_root.join(base);
                            if dir.is_dir() {
                                if let Ok(entries) = fs::read_dir(&dir) {
                                    for ent in entries.flatten() {
                                        let p = ent.path();
                                        if p.is_dir() {
                                            let pkg = p.join("package.json");
                                            if pkg.is_file() {
                                                paths.push(pkg);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Platform services (non-workspace npm packages)
    for svc in ["stagecraft", "tenant-hello"] {
        let p = repo_root
            .join("platform/services")
            .join(svc)
            .join("package.json");
        if p.is_file() {
            paths.push(p);
        }
    }

    paths.sort();
    paths.dedup();
    paths
}

fn normalize_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_dep_names_empty() {
        let doc: toml::Value = "[package]\nname = \"test\"".parse().unwrap();
        assert!(extract_dep_names(&doc, "dependencies").is_empty());
    }

    #[test]
    fn extract_dep_names_basic() {
        let doc: toml::Value = r#"
            [dependencies]
            serde = "1"
            clap = { version = "4" }
        "#
        .parse()
        .unwrap();
        let names = extract_dep_names(&doc, "dependencies");
        assert_eq!(names, vec!["clap", "serde"]);
    }

    #[test]
    fn extract_dep_names_with_rename() {
        let doc: toml::Value = r#"
            [dependencies]
            my_dep = { package = "actual-name", version = "1" }
        "#
        .parse()
        .unwrap();
        let names = extract_dep_names(&doc, "dependencies");
        assert_eq!(names, vec!["actual-name"]);
    }
}
