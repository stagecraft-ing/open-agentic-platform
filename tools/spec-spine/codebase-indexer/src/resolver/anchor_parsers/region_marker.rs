//! Spec 152 §2.1 — `// region: <name>` / `// endregion` markers.
//!
//! Covers Rust, TypeScript, JavaScript, Shell, TOML, and any other
//! source file not covered by a dedicated parser. The opener
//! `// region: <name>` is matched literally; the closer `// endregion`
//! may or may not carry the same name. Unmatched openers (no closing
//! `// endregion`) are reported as `Err(...)` because the file is
//! malformed.

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
                "region marker {anchor:?} has no matching `// endregion`"
            ));
        }
        Ok(None)
    }
}

/// Match `// region: <name>` with arbitrary leading whitespace.
/// Returns the name string (trimmed).
fn parse_region_open(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("//")?.trim_start();
    let rest = rest.strip_prefix("region:")?.trim();
    if rest.is_empty() { None } else { Some(rest) }
}

/// Match `// endregion` (with optional trailing name we don't compare
/// against) with arbitrary leading whitespace.
fn is_region_close(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed.strip_prefix("//") else {
        return false;
    };
    let rest = rest.trim_start();
    rest.starts_with("endregion")
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
}
