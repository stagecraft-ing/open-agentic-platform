//! File-level spec annotation scanner (spec 129).
//!
//! Walks Rust source files inside discovered packages and extracts
//! `// Spec: specs/NNN-slug/spec.md` headers from the leading comment
//! block of each file. Returns a per-file mapping that the cross-reference
//! engine merges into the traceability layer with a `comment-header`
//! source variant.
//!
//! Recognition rules:
//! - The comment must appear in the leading comment block (before any
//!   non-comment, non-blank line).
//! - Both `// Spec:` and `//! Spec:` (doc comments) are accepted; the
//!   keyword is case-sensitive on `Spec:` to match the existing convention
//!   used in 50+ source files.
//! - Path-style references (`specs/NNN-slug/spec.md`) and short-form
//!   references (`NNN-slug` directly after `Spec:`) are both extracted.
//!   The first match per file wins; subsequent `Spec:` lines are ignored.
//! - Trailing `— FR-001`, `— §3`, `— T012` qualifiers are tolerated and
//!   discarded — only the `NNN-slug` is captured.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// One file → one spec ID, when a header is present.
pub type CommentHeaderMap = BTreeMap<String, String>; // repo-relative path → spec id

/// Scan every `.rs` file under `package_paths` (each a repo-relative
/// directory) for a Spec comment header. `repo_root` is used to resolve
/// the absolute paths and to render results back to repo-relative form.
pub fn scan_packages(repo_root: &Path, package_paths: &[String]) -> CommentHeaderMap {
    let mut out = CommentHeaderMap::new();
    for pkg in package_paths {
        let abs = repo_root.join(pkg);
        if !abs.is_dir() {
            continue;
        }
        scan_dir(&abs, repo_root, &mut out);
    }
    out
}

fn scan_dir(dir: &Path, repo_root: &Path, out: &mut CommentHeaderMap) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip target/, node_modules/, .git/ defensively.
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "target" | "node_modules" | ".git" | "tests" | "benches") {
                continue;
            }
            scan_dir(&path, repo_root, out);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if let Some(spec_id) = extract_header(&path) {
            let rel = path
                .strip_prefix(repo_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(rel, spec_id);
        }
    }
}

fn extract_header(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    parse_leading_block(&raw)
}

/// Parse the leading comment block of a Rust source for a `Spec:` line.
/// Stops scanning at the first non-comment, non-blank line.
pub fn parse_leading_block(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        // `//`, `///`, or `//!` are all comment lines.
        if !trimmed.starts_with("//") {
            // Hit a non-comment line — leading block is over.
            return None;
        }
        // Strip the comment prefix and look for a Spec: claim.
        let body = trimmed
            .trim_start_matches("//")
            .trim_start_matches('!')
            .trim_start_matches('/')
            .trim_start();
        if let Some(rest) = body.strip_prefix("Spec:") {
            return parse_spec_id(rest);
        }
    }
    None
}

/// Extract `NNN-slug` from a `Spec:` payload. Accepts:
/// - `specs/NNN-slug/spec.md` (canonical long form)
/// - `specs/NNN-slug/` (trailing slash)
/// - `NNN-slug` (short form)
/// Returns `None` if no `\d{3}-[a-z][a-z0-9-]*` token is present.
fn parse_spec_id(payload: &str) -> Option<String> {
    let trimmed = payload.trim();
    // Try long form: split on whitespace, take first token, then peel.
    let first = trimmed.split_whitespace().next()?;
    // Strip leading `specs/` if present, trailing `/spec.md` or `/`.
    let mid = first
        .strip_prefix("specs/")
        .unwrap_or(first)
        .trim_end_matches("/spec.md")
        .trim_end_matches('/');
    if is_spec_id(mid) {
        return Some(mid.to_string());
    }
    None
}

fn is_spec_id(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 5 {
        return false;
    }
    // First 3 chars: digits.
    if !bytes[..3].iter().all(|b| b.is_ascii_digit()) {
        return false;
    }
    // 4th char: hyphen.
    if bytes[3] != b'-' {
        return false;
    }
    // Remainder: ASCII lowercase + digits + hyphens only, must start with letter.
    let tail = &s[4..];
    let first = tail.chars().next().unwrap_or('-');
    if !first.is_ascii_lowercase() {
        return false;
    }
    tail.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_rs(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(body.as_bytes()).unwrap();
        p
    }

    #[test]
    fn parse_long_form_path() {
        let body = "// SPDX-License-Identifier: AGPL-3.0-or-later\n\
                    // Spec: specs/044-multi-agent-orchestration/spec.md\n\
                    pub fn foo() {}\n";
        assert_eq!(
            parse_leading_block(body).as_deref(),
            Some("044-multi-agent-orchestration")
        );
    }

    #[test]
    fn parse_short_form() {
        let body = "// Spec: 067-tool-definition-registry\n";
        assert_eq!(
            parse_leading_block(body).as_deref(),
            Some("067-tool-definition-registry")
        );
    }

    #[test]
    fn parse_long_form_with_qualifier() {
        let body = "// SPDX-License-Identifier: AGPL-3.0-or-later\n\
                    // Spec: specs/102-governed-excellence/spec.md — FR-002 through FR-010\n\
                    use std::fs;\n";
        assert_eq!(
            parse_leading_block(body).as_deref(),
            Some("102-governed-excellence")
        );
    }

    #[test]
    fn parse_doc_comment_form() {
        let body = "//! Spec: specs/041-checkpoint-restore-ui/spec.md\n";
        assert_eq!(
            parse_leading_block(body).as_deref(),
            Some("041-checkpoint-restore-ui")
        );
    }

    #[test]
    fn ignores_spec_after_code() {
        let body = "use std::fs;\n// Spec: specs/044-multi-agent-orchestration/spec.md\n";
        assert!(parse_leading_block(body).is_none());
    }

    #[test]
    fn ignores_when_first_token_isnt_spec_keyword() {
        let body = "// Feature 079: scheduling notes\n";
        assert!(parse_leading_block(body).is_none());
    }

    #[test]
    fn rejects_malformed_spec_ids() {
        // No leading digits.
        assert!(parse_leading_block("// Spec: my-spec\n").is_none());
        // Wrong digit count.
        assert!(parse_leading_block("// Spec: 12-foo\n").is_none());
        // Empty body.
        assert!(parse_leading_block("// Spec:\n").is_none());
        // Capitalised slug.
        assert!(parse_leading_block("// Spec: 044-MultiAgent\n").is_none());
    }

    #[test]
    fn scan_walks_recursive_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg = tmp.path().join("crates/foo");
        fs::create_dir_all(pkg.join("src/sub")).unwrap();
        // Crate Cargo.toml so this looks like a real package layout.
        fs::write(pkg.join("Cargo.toml"), "[package]\nname = \"foo\"\n").unwrap();
        write_rs(
            &pkg.join("src"),
            "lib.rs",
            "// Spec: specs/044-multi-agent-orchestration/spec.md\n",
        );
        write_rs(
            &pkg.join("src/sub"),
            "mod.rs",
            "//! Spec: specs/067-tool-definition-registry/spec.md\n",
        );
        // Non-rs file should be ignored.
        fs::write(pkg.join("src/notes.txt"), "// Spec: 044-foo\n").unwrap();
        // target/ subdir should be skipped.
        fs::create_dir_all(pkg.join("target/debug")).unwrap();
        write_rs(
            &pkg.join("target/debug"),
            "ignored.rs",
            "// Spec: specs/999-fake/spec.md\n",
        );

        let map = scan_packages(tmp.path(), &["crates/foo".to_string()]);
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("crates/foo/src/lib.rs").map(String::as_str),
            Some("044-multi-agent-orchestration")
        );
        assert_eq!(
            map.get("crates/foo/src/sub/mod.rs").map(String::as_str),
            Some("067-tool-definition-registry")
        );
    }
}
