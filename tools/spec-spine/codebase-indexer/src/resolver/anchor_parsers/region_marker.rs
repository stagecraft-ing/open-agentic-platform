//! Spec 152 §2.1 — `// region: <name>` / `// endregion` (source files)
//! and `# region: <name>` / `# endregion` (shell, YAML, TOML, `.env`)
//! markers.
//!
//! Covers Rust, TypeScript, JavaScript, Shell, TOML, YAML config, and
//! any other comment-bearing source file not covered by a dedicated
//! parser. Both `//` and `#` line-comment prefixes are accepted on the
//! same opener/closer shape (spec 152 §2.1: shell scripts and TOML/YAML
//! config use `#`; source files use `//`). The closer may or may not
//! carry the same name. Unmatched openers (no closing `endregion`) are
//! reported as `Err(...)` because the file is malformed.

use super::{AnchorParser, AnchorResult};
use crate::types::LineSpan;

pub struct RegionMarkerParser;

impl AnchorParser for RegionMarkerParser {
    fn find_anchor(&self, content: &str, anchor: &str) -> AnchorResult {
        let mut start: Option<u32> = None;
        for (idx, line) in content.lines().enumerate() {
            let lineno = (idx as u32) + 1;
            if let Some(name) = parse_region_open(line) {
                if name == anchor {
                    start = Some(lineno);
                }
                continue;
            }
            if is_region_close(line) {
                if let Some(s) = start {
                    return Ok(Some(LineSpan {
                        start_line: s,
                        end_line: lineno,
                    }));
                }
            }
        }
        if start.is_some() {
            // Opener without a closer is a malformed file, not a
            // missing anchor — surface separately so the caller can
            // raise the right diagnostic.
            return Err(format!(
                "region marker {anchor:?} has no matching `endregion`"
            ));
        }
        Ok(None)
    }
}

/// Match `// region: <name>` (source-file convention) or `# region:
/// <name>` (shell/YAML/TOML/.env convention) with arbitrary leading
/// whitespace. Returns the name string (trimmed). Spec 152 §2.1: the
/// prefix follows the file kind's native single-line-comment syntax.
fn parse_region_open(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = strip_comment_prefix(trimmed)?.trim_start();
    let rest = rest.strip_prefix("region:")?.trim();
    if rest.is_empty() { None } else { Some(rest) }
}

/// Match `// endregion` or `# endregion` (with optional trailing name
/// we don't compare against) with arbitrary leading whitespace.
fn is_region_close(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(rest) = strip_comment_prefix(trimmed) else {
        return false;
    };
    let rest = rest.trim_start();
    rest.starts_with("endregion")
}

/// Strip a leading `//` (source) or `#` (shell/YAML/TOML/.env) comment
/// marker. Order matters: `//` is checked first so that a hypothetical
/// `#//` line (impossible in practice) isn't mis-stripped.
///
/// `#` matches only when followed by whitespace or end-of-line —
/// guards against Rust attribute syntax `#[...]` and shell shebang
/// `#!/...`. Both are non-region tokens and shouldn't be parsed as
/// comment openers.
fn strip_comment_prefix(s: &str) -> Option<&str> {
    if let Some(rest) = s.strip_prefix("//") {
        return Some(rest);
    }
    let rest = s.strip_prefix('#')?;
    match rest.chars().next() {
        None => Some(rest),
        Some(c) if c.is_whitespace() => Some(rest),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
fn outer() {}

// region: helpers
fn helper_a() {}
fn helper_b() {}
// endregion

fn main() {}
";

    #[test]
    fn finds_block() {
        let p = RegionMarkerParser;
        let span = p.find_anchor(SAMPLE, "helpers").unwrap().unwrap();
        assert_eq!(span.start_line, 3);
        assert_eq!(span.end_line, 6);
    }

    #[test]
    fn missing_anchor_returns_none() {
        let p = RegionMarkerParser;
        assert!(p.find_anchor(SAMPLE, "absent").unwrap().is_none());
    }

    #[test]
    fn unmatched_open_is_error() {
        let p = RegionMarkerParser;
        let result = p.find_anchor("// region: never_closes\nfn x() {}\n", "never_closes");
        assert!(result.is_err());
    }

    const SHELL_SAMPLE: &str = "\
#!/usr/bin/env bash
set -euo pipefail

# region: bootstrap
echo \"setting up\"
mkdir -p /tmp/example
# endregion

main \"$@\"
";

    #[test]
    fn finds_hash_prefixed_region() {
        let p = RegionMarkerParser;
        let span = p.find_anchor(SHELL_SAMPLE, "bootstrap").unwrap().unwrap();
        assert_eq!(span.start_line, 4);
        assert_eq!(span.end_line, 7);
    }

    #[test]
    fn shebang_is_not_region_open() {
        // `#!/usr/bin/env bash` must not be parsed as a region opener.
        let p = RegionMarkerParser;
        assert!(p.find_anchor(SHELL_SAMPLE, "/usr").unwrap().is_none());
    }

    #[test]
    fn rust_attribute_is_not_region_open() {
        // `#[derive(...)]` must not be parsed as a region opener even
        // though it starts with `#`.
        let p = RegionMarkerParser;
        let sample = "#[derive(Debug)]\nstruct X;\n";
        assert!(p.find_anchor(sample, "derive").unwrap().is_none());
    }
}
