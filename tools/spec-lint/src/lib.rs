//! Optional conformance warnings (Feature 006) — does not replace spec-compiler validation.

use open_agentic_frontmatter::split_frontmatter_optional;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Warning {
    pub code: &'static str,
    pub path: String,
    pub message: String,
}

/// Discover `specs/<NNN>-<kebab>/` directories (same shape as spec-compiler).
pub fn feature_spec_dirs(repo_root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let specs = repo_root.join("specs");
    let mut out = Vec::new();
    if !specs.is_dir() {
        return Ok(out);
    }
    for ent in fs::read_dir(&specs)? {
        let p = ent?.path();
        if !p.is_dir() {
            continue;
        }
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_feature_dir_name(name) && p.join("spec.md").is_file() {
            out.push(p);
        }
    }
    out.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    Ok(out)
}

fn is_feature_dir_name(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() >= 5 && b[..3].iter().all(|c| c.is_ascii_digit()) && b[3] == b'-'
}

fn superseded_pointer_ok(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("superseded by")
        || lower.contains("## supersession")
        || lower.contains("replacement feature")
        || Regex::new(r"`[0-9]{3}-[a-z0-9]+(-[a-z0-9]+)*`")
            .unwrap()
            .is_match(body)
}

fn retired_rationale_ok(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("## retirement")
        || lower.contains("**retired**")
        || lower.contains("rationale")
        || lower.contains("withdrawn")
        || lower.contains("retired (")
}

fn is_example_changeset(content: &str) -> bool {
    let head: String = content.chars().take(4096).collect();
    let lower = head.to_lowercase();
    lower.contains("example")
        || lower.contains("illustrates")
        || lower.contains("non-normative template")
}

fn rel(repo_root: &Path, p: &Path) -> String {
    p.strip_prefix(repo_root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Run all MVP lint rules; warnings are best-effort heuristics (see Feature 006 spec).
pub fn lint_feature_dir(repo_root: &Path, feature_dir: &Path) -> Vec<Warning> {
    let mut w = Vec::new();
    let spec_path = feature_dir.join("spec.md");
    let tasks_path = feature_dir.join("tasks.md");
    let changeset_path = feature_dir.join("execution/changeset.md");
    let verification_path = feature_dir.join("execution/verification.md");

    let spec_raw = match fs::read_to_string(&spec_path) {
        Ok(s) => s,
        Err(_) => return w,
    };

    const VALID_STATUSES: &[&str] = &["draft", "approved", "superseded", "retired"];
    const VALID_IMPLEMENTATIONS: &[&str] =
        &["pending", "in-progress", "complete", "n/a", "deferred"];

    if let Some((fm, body)) = split_frontmatter_optional(&spec_raw) {
        if let Some(status) = fm.get("status").and_then(|v| v.as_str()) {
            if !VALID_STATUSES.contains(&status) {
                w.push(Warning {
                    code: "W-006",
                    path: rel(repo_root, &spec_path),
                    message: format!(
                        "status '{}' is not in the canonical enum (draft | active | approved | superseded | retired) per Feature 000",
                        status
                    ),
                });
            }
            if status == "superseded" && !superseded_pointer_ok(&body) {
                w.push(Warning {
                    code: "W-002",
                    path: rel(repo_root, &spec_path),
                    message: "status is superseded but body lacks an obvious replacement pointer (Feature 003)".into(),
                });
            }
            if status == "retired" && !retired_rationale_ok(&body) {
                w.push(Warning {
                    code: "W-003",
                    path: rel(repo_root, &spec_path),
                    message: "status is retired but body lacks an obvious rationale section (Feature 003)".into(),
                });
            }
        }
        if let Some(impl_status) = fm.get("implementation").and_then(|v| v.as_str()) {
            if !VALID_IMPLEMENTATIONS.contains(&impl_status) {
                w.push(Warning {
                    code: "W-007",
                    path: rel(repo_root, &spec_path),
                    message: format!(
                        "implementation '{}' is not in the canonical enum (pending | in-progress | complete | n/a | deferred) per Feature 000",
                        impl_status
                    ),
                });
            }
        }
    }

    if let Ok(tasks_raw) = fs::read_to_string(&tasks_path) {
        let has_pending_tag = tasks_raw.contains("(pending)");
        for line in tasks_raw.lines() {
            let l = line.trim();
            if l.starts_with("- [x]")
                && l.to_lowercase().contains("(complete)")
                && !verification_path.is_file()
            {
                w.push(Warning {
                    code: "W-001",
                    path: rel(repo_root, &tasks_path),
                    message: "task marked (complete) but execution/verification.md is missing (Feature 005)".into(),
                });
                break;
            }
        }
        if has_pending_tag && tasks_raw.contains("### ") {
            w.push(Warning {
                code: "W-005",
                path: rel(repo_root, &tasks_path),
                message: "mixed task-state notation: (pending) tags and ### section headings in one tasks.md (Feature 004)".into(),
            });
        }
    }

    if changeset_path.is_file() {
        if let Ok(cs) = fs::read_to_string(&changeset_path) {
            if !is_example_changeset(&cs) && !verification_path.is_file() {
                w.push(Warning {
                    code: "W-004",
                    path: rel(repo_root, &changeset_path),
                    message: "execution/changeset.md exists but execution/verification.md is missing (Feature 005)".into(),
                });
            }
        }
    }

    w
}

pub fn lint_repo(repo_root: &Path) -> Vec<Warning> {
    let mut all = Vec::new();
    let dirs = feature_spec_dirs(repo_root).unwrap_or_default();
    for d in dirs {
        all.extend(lint_feature_dir(repo_root, &d));
    }
    all
}
