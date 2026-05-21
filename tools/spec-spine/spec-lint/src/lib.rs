//! Optional conformance warnings (Feature 006) — does not replace spec-compiler validation.

use open_agentic_spec_types::{CONVENTIONAL_CATEGORIES, SHAPE_TABLE, split_frontmatter_optional};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Warning {
    pub code: &'static str,
    /// Spec 128 §7.1 (amended by spec 147) — severity tier registered at
    /// the W-code's site. `"warning"` participates in `--fail-on-warn`
    /// gating; `"info"` is informational only and is exempt from
    /// fail-on-warn. A future `--fail-on-info` flag may gate info-tier
    /// diagnostics independently.
    pub severity: &'static str,
    pub path: String,
    pub message: String,
}

fn shape_table_has_kind(kind: &str) -> bool {
    SHAPE_TABLE.iter().any(|(k, _)| *k == kind)
}

fn shape_table_allows(kind: &str, shape: &str) -> bool {
    SHAPE_TABLE
        .iter()
        .any(|(k, shapes)| *k == kind && shapes.contains(&shape))
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

    if let Some((fm, _body)) = split_frontmatter_optional(&spec_raw) {
        if let Some(status) = fm.get("status").and_then(|v| v.as_str()) {
            if !VALID_STATUSES.contains(&status) {
                w.push(Warning {
                    code: "W-006",
                    severity: "warning",
                    path: rel(repo_root, &spec_path),
                    message: format!(
                        "status '{}' is not in the canonical enum (draft | active | approved | superseded | retired) per Feature 000",
                        status
                    ),
                });
            }
            // W-002 / W-003 — spec 147 Phase 4 rewired these from prose
            // scans on the body to frontmatter-presence checks. The
            // governance-lifecycle fields (`superseded_by`,
            // `retirement_rationale`) are now KNOWN_KEYS in the spec
            // compiler and carry typed authority; the lint surface
            // checks that authors actually filled them in.
            if status == "superseded" && fm.get("superseded_by").is_none() {
                w.push(Warning {
                    code: "W-002",
                    severity: "warning",
                    path: rel(repo_root, &spec_path),
                    message: "status is superseded but frontmatter is missing `superseded_by:` (spec 147 governance-lifecycle fields)".into(),
                });
            }
            if status == "retired" && fm.get("retirement_rationale").is_none() {
                w.push(Warning {
                    code: "W-003",
                    severity: "warning",
                    path: rel(repo_root, &spec_path),
                    message: "status is retired but frontmatter is missing `retirement_rationale:` (spec 147 governance-lifecycle fields)".into(),
                });
            }
        }
        if let Some(impl_status) = fm.get("implementation").and_then(|v| v.as_str()) {
            if !VALID_IMPLEMENTATIONS.contains(&impl_status) {
                w.push(Warning {
                    code: "W-007",
                    severity: "warning",
                    path: rel(repo_root, &spec_path),
                    message: format!(
                        "implementation '{}' is not in the canonical enum (pending | in-progress | complete | n/a | deferred) per Feature 000",
                        impl_status
                    ),
                });
            }
        }
        // ── Spec 147 — W-130: category value not in conventional vocabulary (info severity) ──
        if let Some(seq) = fm.get("category").and_then(|v| v.as_sequence()) {
            for item in seq {
                let Some(tag) = item.as_str() else {
                    continue;
                };
                if !CONVENTIONAL_CATEGORIES.contains(&tag) {
                    w.push(Warning {
                        code: "W-130",
                        severity: "info",
                        path: rel(repo_root, &spec_path),
                        message: format!(
                            "category value {tag:?} is not in the conventional vocabulary; conventional values: {}",
                            CONVENTIONAL_CATEGORIES.join(", ")
                        ),
                    });
                }
            }
        }
        // ── Spec 147 — W-131: shape value outside the declared (kind, shape) table (warning severity) ──
        if let (Some(kind), Some(shape)) = (
            fm.get("kind").and_then(|v| v.as_str()),
            fm.get("shape").and_then(|v| v.as_str()),
        ) {
            if shape_table_has_kind(kind) && !shape_table_allows(kind, shape) {
                w.push(Warning {
                    code: "W-131",
                    severity: "warning",
                    path: rel(repo_root, &spec_path),
                    message: format!(
                        "shape value {shape:?} is not in the declared (kind, shape) table for kind={kind:?}; novel shape values must trigger an explicit table update per spec 147 §`shape:`"
                    ),
                });
            }
        }

        // ── Spec 130 — V-020: spec lacks relationship fields ──
        //
        // Fires when a spec declares no relationship to code or other specs
        // (none of `establishes`, `extends`, `refines`, `supersedes`,
        // `amends`, `co_authority`, `constrains`) and does not carry the
        // bootstrap marker `origin: retroactive: true`. The eight
        // relationship fields are the corpus's machine-readable governance
        // model (spec 130). V-020 prevents new specs from accreting
        // without declaring their relationships.
        let has_relationship_field = [
            "establishes",
            "extends",
            "refines",
            "supersedes",
            "amends",
            "co_authority",
            "constrains",
        ]
        .iter()
        .any(|k| fm.get(*k).is_some());

        let is_retroactive_bootstrap = fm
            .get("origin")
            .and_then(|v| v.as_mapping())
            .and_then(|m| m.get("retroactive"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !has_relationship_field && !is_retroactive_bootstrap {
            w.push(Warning {
                code: "V-020",
                severity: "warning",
                path: rel(repo_root, &spec_path),
                message: "spec carries no relationship fields (establishes / extends / refines / supersedes / amends / co_authority / constrains) and is not marked `origin: retroactive: true`; declare an honest relationship per spec 130".into(),
            });
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
                    severity: "warning",
                    path: rel(repo_root, &tasks_path),
                    message: "task marked (complete) but execution/verification.md is missing (Feature 005)".into(),
                });
                break;
            }
        }
        if has_pending_tag && tasks_raw.contains("### ") {
            w.push(Warning {
                code: "W-005",
                severity: "warning",
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
                    severity: "warning",
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
    for d in &dirs {
        all.extend(lint_feature_dir(repo_root, d));
    }
    all.extend(corpus_lint_pass(repo_root, &dirs));
    all.extend(spec154_l005_pass(repo_root, &dirs));
    all
}

// ─────────────────────────────────────────────────────────────────────
// Spec 154 — L-005 advisory soft lint
// ─────────────────────────────────────────────────────────────────────

/// Discover workspace-member directory paths from the root Cargo.toml.
/// Returns a sorted set of relative directory paths (e.g.
/// `"crates/canonical-json"`, `"tools/spec-spine/spec-compiler"`).
/// Empty when the manifest is absent or unparseable.
fn workspace_member_dirs(repo_root: &Path) -> Vec<String> {
    let manifest = repo_root.join("Cargo.toml");
    let Ok(raw) = fs::read_to_string(&manifest) else {
        return Vec::new();
    };
    let Ok(parsed) = raw.parse::<toml::Value>() else {
        return Vec::new();
    };
    let Some(members) = parsed
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
    else {
        return Vec::new();
    };
    let mut out: Vec<String> = members
        .iter()
        .filter_map(|m| m.as_str().map(String::from))
        .collect();
    out.sort();
    out
}

/// L-005 — advisory soft lint (info severity) emitted when a legacy
/// bare-string path inside any relationship field falls inside a
/// workspace-member directory and could be expressed as a `crate:`
/// unit (spec 154 §3.1). Lint, not error: corpus migration is
/// Tier 2 Segment 5; this surface lets reviewers steer authors in
/// the meantime.
///
/// Does not fire on:
///   * explicit `unit: {kind: ...}` declarations (already typed);
///   * paths outside any workspace member directory (legitimate
///     `file:` cases — `Makefile`, `deny.toml`, `standards/...`).
fn spec154_l005_pass(repo_root: &Path, feature_dirs: &[PathBuf]) -> Vec<Warning> {
    let members = workspace_member_dirs(repo_root);
    if members.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<Warning> = Vec::new();
    for d in feature_dirs {
        let spec_path = d.join("spec.md");
        let Ok(raw) = fs::read_to_string(&spec_path) else {
            continue;
        };
        let Some((fm, _)) = split_frontmatter_optional(&raw) else {
            continue;
        };
        let mapping = match fm.as_mapping() {
            Some(m) => m,
            None => continue,
        };
        let mut seen_paths: std::collections::BTreeSet<String> =
            std::collections::BTreeSet::new();
        collect_legacy_paths(mapping, &mut seen_paths);
        for path in &seen_paths {
            if let Some(member) = match_workspace_member(path, &members) {
                out.push(Warning {
                    code: "L-005",
                    severity: "info",
                    path: rel(repo_root, &spec_path),
                    message: format!(
                        "legacy bare-string path {path:?} sits inside workspace member {member:?}; consider migrating to `unit: {{ kind: crate, id: <manifest-name> }}` per spec 154 §3.1 (Tier 2 Segment 5 corpus migration)",
                    ),
                });
            }
        }
    }
    out
}

/// Walk a frontmatter mapping, collecting every bare-string path that
/// appears in a relationship field (`establishes`, `extends.paths`,
/// `refines.paths`, `co_authority.paths`, `constrains.paths`,
/// `references`). Strings inside explicit `unit:` fields are not
/// collected — they are already typed.
fn collect_legacy_paths(
    fm: &serde_yaml::Mapping,
    out: &mut std::collections::BTreeSet<String>,
) {
    // `establishes:` — array, items are strings or `{unit:...}`/`{kind:...}`.
    if let Some(seq) = fm.get("establishes").and_then(|v| v.as_sequence()) {
        for item in seq {
            if let Some(s) = item.as_str() {
                out.insert(s.to_string());
            }
        }
    }
    // Item-based relationships with legacy `paths:` plural form.
    for key in &["extends", "refines", "supersedes", "co_authority", "constrains"] {
        let Some(seq) = fm.get(*key).and_then(|v| v.as_sequence()) else {
            continue;
        };
        for item in seq {
            let Some(map) = item.as_mapping() else {
                continue;
            };
            if let Some(paths) = map.get("paths").and_then(|v| v.as_sequence()) {
                for p in paths {
                    if let Some(s) = p.as_str() {
                        out.insert(s.to_string());
                    }
                }
            }
        }
    }
    // `references:` — bare strings (legacy / shorthand form).
    if let Some(seq) = fm.get("references").and_then(|v| v.as_sequence()) {
        for item in seq {
            if let Some(s) = item.as_str() {
                out.insert(s.to_string());
            }
        }
    }
}

/// Return the workspace-member directory that contains `path`, if
/// any. `path` is a repo-relative posix path (e.g.
/// `"crates/foo/src/lib.rs"` or `"crates/foo/"`); `members` is the
/// sorted list of workspace-member directory paths.
fn match_workspace_member(path: &str, members: &[String]) -> Option<String> {
    let normalised = path.trim_end_matches('/');
    for m in members {
        if normalised == m {
            return Some(m.clone());
        }
        let prefix = format!("{m}/");
        if normalised.starts_with(&prefix) || path.starts_with(&prefix) {
            return Some(m.clone());
        }
    }
    None
}

/// Spec 147 — corpus-level W-codes that need to see every spec at once.
/// Today this is W-132 (orphan capability surface); future corpus-wide
/// info diagnostics slot in here.
fn corpus_lint_pass(repo_root: &Path, feature_dirs: &[PathBuf]) -> Vec<Warning> {
    let mut out: Vec<Warning> = Vec::new();
    // Collect (spec-id, kind, frontmatter, path) for every spec.
    #[derive(Clone)]
    struct SpecView {
        id: String,
        kind: Option<String>,
        selectable_by: Option<String>,
        selects: Vec<String>, // capability ids selected by a profile, if any
        path: String,
    }
    let mut views: Vec<SpecView> = Vec::new();
    for d in feature_dirs {
        let spec_path = d.join("spec.md");
        let Ok(raw) = fs::read_to_string(&spec_path) else {
            continue;
        };
        let Some((fm, _)) = split_frontmatter_optional(&raw) else {
            continue;
        };
        let id = fm
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let kind = fm.get("kind").and_then(|v| v.as_str()).map(|s| s.to_string());
        let selectable_by = fm
            .get("selectable_by")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let selects: Vec<String> = fm
            .get("selects")
            .and_then(|v| v.as_mapping())
            .map(|m| {
                m.values()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        views.push(SpecView {
            id,
            kind,
            selectable_by,
            selects,
            path: rel(repo_root, &spec_path),
        });
    }

    // W-132 — capability declares `selectable_by:` but no profile spec
    // selects this capability. Surfaces orphan capabilities. Info
    // severity (spec 128 §7.3): not a contract violation.
    let selected_caps: std::collections::BTreeSet<String> = views
        .iter()
        .filter(|s| s.kind.as_deref() == Some("profile"))
        .flat_map(|s| s.selects.iter().cloned())
        .collect();
    for s in &views {
        if s.kind.as_deref() != Some("capability") {
            continue;
        }
        if s.selectable_by.is_none() {
            continue;
        }
        let id_prefix = s.id.split_once('-').map(|(p, _)| p).unwrap_or(s.id.as_str());
        let referenced = selected_caps.iter().any(|c| {
            c == &s.id
                || c == id_prefix
                || c.split_once('-').map(|(p, _)| p).unwrap_or(c.as_str()) == id_prefix
        });
        if !referenced {
            out.push(Warning {
                code: "W-132",
                severity: "info",
                path: s.path.clone(),
                message: format!(
                    "capability {id:?} declares `selectable_by:` but no profile spec selects it; orphan capability (advisory, info-tier)",
                    id = s.id
                ),
            });
        }
    }

    out
}
