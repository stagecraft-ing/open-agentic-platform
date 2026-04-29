//! Workflow-to-spec traceability scanner per spec 118.
//!
//! Scans `.github/workflows/**/*.yml` for the leading-comment convention
//! `# Spec: NNN-kebab-case-slug` and emits a [`WorkflowTrace`] per file.
//! Files without a header AND without an entry in
//! `tools/codebase-indexer/workflow-allowlist.toml` produce diagnostic I-105.

use crate::types::{Diagnostic, WorkflowTrace, WorkflowTraceSource};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

const HEADER_SCAN_LINES: usize = 20;

#[derive(Deserialize, Default)]
struct AllowlistDoc {
    #[serde(default)]
    allowlist: Vec<AllowlistEntry>,
}

#[derive(Deserialize)]
struct AllowlistEntry {
    path: String,
    #[allow(dead_code)] // surfaced for human inspection only
    reason: String,
    #[serde(default)]
    spec: Option<String>,
}

/// Result of a scan: the trace records (sorted by path) plus diagnostics.
pub struct ScanResult {
    pub traces: Vec<WorkflowTrace>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn scan_workflows(repo_root: &Path) -> ScanResult {
    let workflows_dir = repo_root.join(".github/workflows");
    let mut traces: Vec<WorkflowTrace> = Vec::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    let allowlist = load_allowlist(repo_root);

    if !workflows_dir.is_dir() {
        return ScanResult {
            traces,
            diagnostics,
        };
    }

    let mut workflow_paths: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&workflows_dir) {
        for ent in entries.flatten() {
            let p = ent.path();
            if !p.is_file() {
                continue;
            }
            let ext = p.extension().and_then(|e| e.to_str());
            if ext != Some("yml") && ext != Some("yaml") {
                continue;
            }
            workflow_paths.push(p);
        }
    }
    workflow_paths.sort();

    for path in workflow_paths {
        let rel = relative_to_repo(&path, repo_root);

        // 1) Header scan.
        let header_specs = parse_header_specs(&path);

        // 2) Allowlist lookup.
        let allowlist_hit = allowlist.iter().find(|a| a.path == rel);

        let (specs, source) = if !header_specs.is_empty() {
            (header_specs, WorkflowTraceSource::Header)
        } else if let Some(entry) = allowlist_hit {
            let allow_specs: Vec<String> = entry.spec.iter().cloned().collect();
            (allow_specs, WorkflowTraceSource::Allowlist)
        } else {
            // No header, no allowlist — diagnostic I-105 (warning, not error).
            diagnostics.push(Diagnostic {
                code: "I-105".into(),
                message: format!(
                    "workflow {rel} declares no `# Spec: NNN-slug` header and is not listed in workflow-allowlist.toml"
                ),
                path: Some(rel.clone()),
            });
            (Vec::new(), WorkflowTraceSource::Unmapped)
        };

        traces.push(WorkflowTrace {
            path: rel,
            specs,
            source,
        });
    }

    ScanResult {
        traces,
        diagnostics,
    }
}

fn parse_header_specs(path: &Path) -> Vec<String> {
    let raw = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        if idx >= HEADER_SCAN_LINES {
            break;
        }
        let trimmed = line.trim_start();
        if !trimmed.starts_with('#') {
            continue;
        }
        // Accept "# Spec: NNN-slug" — case-sensitive on the keyword to keep
        // the convention strict; loose match on whitespace.
        let after_hash = trimmed.trim_start_matches('#').trim_start();
        if let Some(rest) = after_hash.strip_prefix("Spec:") {
            let slug = rest.trim();
            if !slug.is_empty() {
                out.push(slug.to_string());
            }
        }
    }
    out
}

fn load_allowlist(repo_root: &Path) -> Vec<AllowlistEntry> {
    let path = repo_root.join("tools/codebase-indexer/workflow-allowlist.toml");
    let raw = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let doc: AllowlistDoc = match toml::from_str(&raw) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    doc.allowlist
}

fn relative_to_repo(path: &Path, repo_root: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(p: &Path, s: &str) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, s).unwrap();
    }

    #[test]
    fn parses_single_spec_header() {
        let td = TempDir::new().unwrap();
        let wf = td.path().join(".github/workflows/example.yml");
        write(
            &wf,
            "# Spec: 042-multi-provider-agent-registry\n\nname: Example\n",
        );
        let r = scan_workflows(td.path());
        assert_eq!(r.traces.len(), 1);
        assert_eq!(r.traces[0].specs, vec!["042-multi-provider-agent-registry"]);
        assert_eq!(r.traces[0].source, WorkflowTraceSource::Header);
        assert!(r.diagnostics.is_empty());
    }

    #[test]
    fn parses_multi_spec_header() {
        let td = TempDir::new().unwrap();
        let wf = td.path().join(".github/workflows/example.yml");
        write(
            &wf,
            "# Spec: 037-cross-platform-axiomregent\n# Spec: 117-release-artifact-attestations\n\nname: Example\n",
        );
        let r = scan_workflows(td.path());
        assert_eq!(r.traces.len(), 1);
        assert_eq!(
            r.traces[0].specs,
            vec![
                "037-cross-platform-axiomregent",
                "117-release-artifact-attestations"
            ]
        );
    }

    #[test]
    fn unmapped_workflow_emits_i105() {
        let td = TempDir::new().unwrap();
        let wf = td.path().join(".github/workflows/example.yml");
        write(&wf, "name: Example\n");
        let r = scan_workflows(td.path());
        assert_eq!(r.traces.len(), 1);
        assert_eq!(r.traces[0].source, WorkflowTraceSource::Unmapped);
        assert!(r.traces[0].specs.is_empty());
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].code, "I-105");
    }

    #[test]
    fn allowlist_overrides_missing_header() {
        let td = TempDir::new().unwrap();
        let wf = td.path().join(".github/workflows/ops.yml");
        write(&wf, "name: ops\n");
        let allow = td
            .path()
            .join("tools/codebase-indexer/workflow-allowlist.toml");
        write(
            &allow,
            "[[allowlist]]\npath = \".github/workflows/ops.yml\"\nreason = \"pure ops, no spec\"\nspec = \"085-remote-control-cli\"\n",
        );
        let r = scan_workflows(td.path());
        assert_eq!(r.traces[0].source, WorkflowTraceSource::Allowlist);
        assert_eq!(r.traces[0].specs, vec!["085-remote-control-cli"]);
        assert!(r.diagnostics.is_empty());
    }

    #[test]
    fn ignores_lines_past_header_window() {
        let td = TempDir::new().unwrap();
        let wf = td.path().join(".github/workflows/example.yml");
        let mut body = String::new();
        for _ in 0..25 {
            body.push_str("# filler\n");
        }
        body.push_str("# Spec: 999-too-late\n");
        write(&wf, &body);
        let r = scan_workflows(td.path());
        assert_eq!(r.traces[0].source, WorkflowTraceSource::Unmapped);
        assert!(r.traces[0].specs.is_empty());
    }
}
