//! `spec-code-coupling-check` binary entrypoint (spec 127 + spec 152).
//!
//! Spec 152 activation: the binary now extracts git-diff hunks via
//! `git diff -U0 <base>...<head>`, attributes each hunk to its
//! containing section(s) via [`open_agentic_spec_code_coupling_check::
//! hunk_attribution::attribute_hunks_for_file`], builds a
//! [`SectionClaimIndex`] from the spec registry's `co_authority`
//! frontmatter, and invokes [`check_coupling_section_aware`] for
//! satisfaction. Paths whose hunks cannot be attributed (file type
//! has no parser, or hunks sit outside all named sections) fall back
//! to whole-file authority — identical to the pre-152 behaviour.

use clap::Parser;
use open_agentic_spec_code_coupling_check::{
    BypassConfig, build_section_claim_index, check_coupling_section_aware, load_index, render,
};
use open_agentic_spec_code_coupling_check::hunk_attribution::{
    HunkAttributionMap, HunkSections, attribute_hunks_for_file,
};
use open_agentic_spec_registry_reader::load as load_registry;
use std::collections::BTreeSet;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

#[derive(Parser, Debug)]
#[command(
    name = "spec-code-coupling-check",
    about = "PR-time gate: every diff path claimed by a spec's `implements:` / `co_authority:` frontmatter must be accompanied by an edit to that spec's spec.md (spec 127 + spec 152).",
    version
)]
struct Cli {
    /// Repo root (defaults to current working directory).
    #[arg(long, default_value = ".")]
    repo: PathBuf,

    /// Base ref for the diff (default: origin/main).
    #[arg(long, default_value = "origin/main")]
    base: String,

    /// Head ref for the diff (default: HEAD).
    #[arg(long, default_value = "HEAD")]
    head: String,

    /// Override the diff: read newline-delimited paths from this file.
    /// When set, --base/--head are ignored for path collection (but
    /// still used for hunk extraction unless this file ALSO contains
    /// `@@`-format hunk lines, which it does not). Section attribution
    /// falls back to whole-file authority in this mode.
    #[arg(long)]
    paths_from: Option<PathBuf>,

    /// PR body (waiver source). Path to a file containing the PR body;
    /// defaults to empty if unset and $GITHUB_PR_BODY is also unset.
    #[arg(long)]
    pr_body: Option<PathBuf>,

    /// Path to the codebase index (default: build/codebase-index/index.json
    /// resolved from --repo).
    #[arg(long)]
    index: Option<PathBuf>,

    /// Path to the spec registry (default: build/spec-registry/registry.json
    /// resolved from --repo).
    #[arg(long)]
    registry: Option<PathBuf>,

    /// Path to a newline-delimited bypass-prefix file (Cut D W-08).
    /// Lines starting with `#` are comments; blanks are ignored.
    /// Without this flag, the gate operates fail-closed: every diff
    /// path must be claimed by some spec's implements: list (or its
    /// `co_authority:` section authority must be edited).
    #[arg(long = "bypass-prefix-file")]
    bypass_prefix_file: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let index_path = cli.index.clone().unwrap_or_else(|| {
        cli.repo.join("build/codebase-index/index.json")
    });
    let index = match load_index(&index_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("spec-code-coupling-check: {e}");
            return ExitCode::from(2);
        }
    };

    // The registry feeds section-aware authority (spec 152). When the
    // path is unset and the default location is absent, the gate
    // degrades to whole-file authority — the same behaviour as the
    // pre-152 gate. This keeps integration tests that synthesise an
    // index without a registry working, and gives CI a soft-fail path
    // when the registry hasn't been compiled yet.
    let default_registry_path = cli.repo.join("build/spec-registry/registry.json");
    let registry_path_opt: Option<PathBuf> = match &cli.registry {
        Some(p) => Some(p.clone()),
        None => {
            if default_registry_path.exists() {
                Some(default_registry_path.clone())
            } else {
                None
            }
        }
    };
    let section_claims = match registry_path_opt {
        Some(path) => match load_registry(&path) {
            Ok(r) => build_section_claim_index(&r),
            Err(e) => {
                eprintln!(
                    "spec-code-coupling-check: load registry {}: {e}",
                    path.display()
                );
                return ExitCode::from(2);
            }
        },
        None => {
            // No registry available → empty section claims; whole-file fallback applies.
            open_agentic_spec_code_coupling_check::SectionClaimIndex::new()
        }
    };

    let (diff_paths, hunk_attribution) = match collect_diff(&cli) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("spec-code-coupling-check: {e}");
            return ExitCode::from(2);
        }
    };

    let pr_body = match read_pr_body(&cli) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("spec-code-coupling-check: {e}");
            return ExitCode::from(2);
        }
    };

    let bypass = match &cli.bypass_prefix_file {
        Some(path) => match BypassConfig::from_file(path) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!(
                    "spec-code-coupling-check: read --bypass-prefix-file {}: {e}",
                    path.display()
                );
                return ExitCode::from(2);
            }
        },
        None => BypassConfig::default(),
    };

    let outcome = check_coupling_section_aware(
        &index,
        &diff_paths,
        &hunk_attribution,
        &section_claims,
        &pr_body,
        &bypass,
    );
    let rendered = render(&outcome);
    if !rendered.is_empty() {
        // Stdout for clean run summaries; stderr for failure blocks so
        // the violation header lands in the GitHub Actions step error pane.
        if outcome.exit_code() == 0 {
            println!("{rendered}");
        } else {
            eprintln!("{rendered}");
        }
    }

    let code = outcome.exit_code();
    if code == 0 {
        if outcome.violations.is_empty() {
            println!(
                "spec-code-coupling-check: OK — {} diff path(s) checked.",
                diff_paths.len()
            );
        }
        ExitCode::SUCCESS
    } else {
        ExitCode::from(code as u8)
    }
}

/// Collect both the flat path set (legacy interface) and the per-path
/// section attribution (spec 152 §2.2).
fn collect_diff(cli: &Cli) -> Result<(BTreeSet<String>, HunkAttributionMap), String> {
    if let Some(path) = &cli.paths_from {
        // Path-only mode: no hunk data → whole-file fallback for all
        // paths (the section_aware library handles missing entries).
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("read --paths-from {}: {e}", path.display()))?;
        let paths: BTreeSet<String> = text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        return Ok((paths, HunkAttributionMap::new()));
    }

    let raw = run_git_diff_unified(&cli.repo, &cli.base, &cli.head)?;
    let per_file_hunks = parse_unified_diff(&raw);

    let mut diff_paths: BTreeSet<String> = BTreeSet::new();
    let mut attribution: HunkAttributionMap = HunkAttributionMap::new();

    for (path, hunks) in per_file_hunks {
        diff_paths.insert(path.clone());

        // Read the file's current content for section parsing. A
        // missing file (deleted in the diff) yields whole-file
        // fallback for that path — section attribution skipped.
        let full_path = cli.repo.join(&path);
        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let Some(sections) = attribute_hunks_for_file(&path, &content, &hunks) {
            if !sections.is_empty() {
                attribution.insert(path, sections);
            }
            // Empty section set → fall through to whole-file fallback
            // (don't insert; the section_aware library treats missing
            // entries as "no attribution" → whole-file authority).
        }
    }

    Ok((diff_paths, attribution))
}

fn run_git_diff_unified(repo: &Path, base: &str, head: &str) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["diff", "--no-color", "-U0"])
        .arg(format!("{base}...{head}"))
        .output()
        .map_err(|e| format!("spawn git diff: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("git diff exited {:?}: {stderr}", out.status.code()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Parse `git diff --no-color -U0` output into per-file new-side hunk
/// ranges. Each hunk's range is half-open `[new_start, new_start + new_count)`
/// in 1-based line numbers; pure-deletion hunks (new_count == 0) are
/// represented as a single-line range at `new_start` so they participate
/// in section attribution against the surrounding context.
fn parse_unified_diff(diff_text: &str) -> std::collections::BTreeMap<String, Vec<Range<usize>>> {
    let mut out: std::collections::BTreeMap<String, Vec<Range<usize>>> =
        std::collections::BTreeMap::new();
    let mut current_path: Option<String> = None;

    for line in diff_text.lines() {
        if let Some(rest) = line.strip_prefix("+++ ") {
            // `+++ b/<path>` or `+++ /dev/null`
            let p = rest.trim();
            if p == "/dev/null" {
                current_path = None;
            } else {
                let trimmed = p
                    .strip_prefix("b/")
                    .or_else(|| p.strip_prefix("a/"))
                    .unwrap_or(p);
                current_path = Some(trimmed.to_string());
                out.entry(trimmed.to_string()).or_default();
            }
        } else if line.starts_with("@@") {
            let path = match &current_path {
                Some(p) => p.clone(),
                None => continue,
            };
            if let Some(range) = parse_hunk_header(line) {
                out.entry(path).or_default().push(range);
            }
        }
    }
    out
}

/// Parse a single `@@ -<old> +<new> @@` header into the new-side range.
fn parse_hunk_header(line: &str) -> Option<Range<usize>> {
    // Find the `+` token after `@@ -...`.
    let after_at = line.strip_prefix("@@")?.trim_start();
    let rest = after_at.strip_prefix('-')?;
    // Skip the old range.
    let plus_pos = rest.find('+')?;
    let new_part = rest[plus_pos + 1..].trim_start();
    // The new range is `<start>[,<count>] @@ ...`. Cut at whitespace.
    let new_range_str = new_part.split_whitespace().next()?;
    let (start_s, count_s) = match new_range_str.split_once(',') {
        Some((a, b)) => (a, b),
        None => (new_range_str, "1"),
    };
    let start: usize = start_s.parse().ok()?;
    let count: usize = count_s.parse().ok()?;
    // A pure-deletion hunk reports `new_start,0`. Treat as a 1-line
    // range at start so section attribution against the surrounding
    // context still fires (start is the line BEFORE the deletion in
    // the new file, but git uses the line where the deletion lands).
    let effective_count = if count == 0 { 1 } else { count };
    Some(start..start + effective_count)
}

fn read_pr_body(cli: &Cli) -> Result<String, String> {
    if let Some(path) = &cli.pr_body {
        std::fs::read_to_string(path)
            .map_err(|e| format!("read --pr-body {}: {e}", path.display()))
    } else if let Ok(s) = std::env::var("GITHUB_PR_BODY") {
        Ok(s)
    } else {
        Ok(String::new())
    }
}

// `HunkSections` is used in the binary only for type-naming inside
// the path-collection routine; the public re-export keeps the symbol
// addressable for tests that link against the binary's behaviour.
#[allow(dead_code)]
fn _hunk_sections_typename() -> HunkSections {
    Default::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hunk_header_with_count() {
        let r = parse_hunk_header("@@ -10,5 +12,7 @@ context").unwrap();
        assert_eq!(r, 12..19);
    }

    #[test]
    fn parse_hunk_header_default_count() {
        let r = parse_hunk_header("@@ -10 +12 @@").unwrap();
        assert_eq!(r, 12..13);
    }

    #[test]
    fn parse_hunk_header_pure_deletion() {
        // new_count == 0 → effective 1-line range so attribution still fires.
        let r = parse_hunk_header("@@ -10,5 +12,0 @@").unwrap();
        assert_eq!(r, 12..13);
    }

    #[test]
    fn parse_unified_diff_single_file() {
        let diff = "diff --git a/Makefile b/Makefile\n\
                    index abc..def 100644\n\
                    --- a/Makefile\n\
                    +++ b/Makefile\n\
                    @@ -10,2 +10,3 @@ context\n\
                    @@ -50 +51,5 @@\n";
        let per_file = parse_unified_diff(diff);
        let ranges = per_file.get("Makefile").expect("Makefile present");
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0], 10..13);
        assert_eq!(ranges[1], 51..56);
    }

    #[test]
    fn parse_unified_diff_deleted_file_skipped() {
        let diff = "diff --git a/gone.txt b/gone.txt\n\
                    deleted file mode 100644\n\
                    --- a/gone.txt\n\
                    +++ /dev/null\n\
                    @@ -1,5 +0,0 @@\n";
        let per_file = parse_unified_diff(diff);
        assert!(per_file.is_empty(), "deleted file should not register: {per_file:?}");
    }
}
