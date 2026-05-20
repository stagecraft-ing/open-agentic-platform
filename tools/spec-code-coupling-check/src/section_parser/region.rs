//! Comment-region section parser (spec 152 §2.1).
//!
//! Recognises `region: <name>` / `endregion` markers in source files,
//! using a per-language comment-prefix convention:
//!
//! - `//` for Rust, TypeScript, JavaScript (`.rs`, `.ts`, `.tsx`, `.js`, `.jsx`).
//! - `#` for shell, env files, YAML, TOML, Python, Bash, and any file
//!   without a recognised extension that nonetheless carries `#` comment
//!   markers (`.sh`, `.bash`, `.env*`, `.yaml`, `.yml`, `.toml`, etc.).
//!
//! Two anchor forms per language:
//!
//! - **`<comment-prefix> region: <name>`** — opens a section named `<name>`.
//!   The section extends through the matching `endregion` line inclusive,
//!   or to EOF if unclosed.
//! - **`<comment-prefix> endregion[: <name>]`** — closes the current region.
//!   The optional `<name>` is informational; the first `endregion` after a
//!   `region:` closes the active region regardless of name match.
//!
//! Spec 152 §2.1 is normative: source files with no region markers have
//! whole-file authority, identical to the pre-152 model.

use std::ops::Range;

/// A source-file section with anchor name and 1-based half-open line range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionSection {
    pub name: String,
    pub lines: Range<usize>,
}

/// Comment style for a given file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStyle {
    /// `//` line comments (Rust, TypeScript, JavaScript, C, C++, Go, …).
    DoubleSlash,
    /// `#` line comments (shell, env files, YAML, TOML, Python, …).
    Hash,
}

impl CommentStyle {
    /// Detect the comment style from a file path's extension or basename.
    /// Returns `None` for unknown extensions (the caller can decide whether
    /// to fall back to whole-file or skip region parsing).
    pub fn for_path(path: &str) -> Option<Self> {
        let basename = path
            .rsplit('/')
            .next()
            .unwrap_or(path);

        // `.env*` (including `.env.example`, `.env.local`, etc.) → Hash.
        if basename == ".env" || basename.starts_with(".env.") {
            return Some(CommentStyle::Hash);
        }

        // Match by extension.
        let ext = basename
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())?;
        match ext.as_str() {
            "rs" | "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => Some(CommentStyle::DoubleSlash),
            "sh" | "bash" | "zsh" | "yaml" | "yml" | "toml" | "py" | "ini" | "conf" | "env" => {
                Some(CommentStyle::Hash)
            }
            _ => None,
        }
    }

    fn comment_prefix(self) -> &'static str {
        match self {
            CommentStyle::DoubleSlash => "//",
            CommentStyle::Hash => "#",
        }
    }
}

/// Parse `content` for region markers using the given comment style.
///
/// Returns regions in document order. Overlapping regions are not produced;
/// the first `endregion` closes the active region, and a new `region:`
/// without a closing `endregion` is treated as opening a new section
/// (the prior unclosed region is closed at the new line).
pub fn parse(content: &str, style: CommentStyle) -> Vec<RegionSection> {
    let prefix = style.comment_prefix();
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let mut out: Vec<RegionSection> = Vec::new();
    let mut open: Option<(String, usize)> = None; // (name, start 1-based)

    for (i, raw_line) in lines.iter().enumerate() {
        let lineno = i + 1;
        let trimmed = raw_line.trim_start();
        // Must start with the comment prefix (allowing trailing space).
        let after = match trimmed.strip_prefix(prefix) {
            Some(rest) => rest.trim_start(),
            None => continue,
        };

        if let Some(name) = after.strip_prefix("region:") {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            // Close any open region at the new region's line (overlap is
            // not allowed; rather, the previous region ends here).
            if let Some((open_name, start)) = open.take() {
                out.push(RegionSection {
                    name: open_name,
                    lines: start..lineno,
                });
            }
            open = Some((name.to_string(), lineno));
        } else if after.starts_with("endregion") {
            // Allow `endregion`, `endregion:`, `endregion <name>`, `endregion: <name>`.
            if let Some((name, start)) = open.take() {
                // The endregion line itself is included.
                out.push(RegionSection {
                    name,
                    lines: start..lineno + 1,
                });
            }
        }
    }

    // Unclosed region: extend to EOF.
    if let Some((name, start)) = open {
        out.push(RegionSection {
            name,
            lines: start..total + 1,
        });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rust_region() {
        let content = "\
fn outer() {}

// region: section-matching
pub fn check() {}
pub fn match_section() {}
// endregion

fn after() {}
";
        let s = parse(content, CommentStyle::DoubleSlash);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "section-matching");
        // region on line 3, endregion on line 6 → range 3..7
        assert_eq!(s[0].lines, 3..7);
    }

    #[test]
    fn parses_shell_region_hash() {
        let content = "\
#!/bin/bash
set -e

# region: bootstrap
echo bootstrap-step-1
echo bootstrap-step-2
# endregion

echo done
";
        let s = parse(content, CommentStyle::Hash);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "bootstrap");
        // region on line 4, endregion on line 7 → range 4..8
        assert_eq!(s[0].lines, 4..8);
    }

    #[test]
    fn two_regions_in_one_file() {
        let content = "\
// region: alpha
body a
// endregion

// region: beta
body b
// endregion
";
        let s = parse(content, CommentStyle::DoubleSlash);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].name, "alpha");
        assert_eq!(s[0].lines, 1..4);
        assert_eq!(s[1].name, "beta");
        assert_eq!(s[1].lines, 5..8);
    }

    #[test]
    fn unclosed_region_extends_to_eof() {
        let content = "// region: dangling\nbody\nbody\n";
        let s = parse(content, CommentStyle::DoubleSlash);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "dangling");
        assert_eq!(s[0].lines, 1..4);
    }

    #[test]
    fn no_regions_yields_empty() {
        assert!(parse("just code\nno markers\n", CommentStyle::DoubleSlash).is_empty());
        assert!(parse("", CommentStyle::DoubleSlash).is_empty());
    }

    #[test]
    fn for_path_detects_extensions() {
        assert_eq!(CommentStyle::for_path("foo.rs"), Some(CommentStyle::DoubleSlash));
        assert_eq!(CommentStyle::for_path("foo.ts"), Some(CommentStyle::DoubleSlash));
        assert_eq!(CommentStyle::for_path("foo.tsx"), Some(CommentStyle::DoubleSlash));
        assert_eq!(CommentStyle::for_path("setup.sh"), Some(CommentStyle::Hash));
        assert_eq!(CommentStyle::for_path("values.yaml"), Some(CommentStyle::Hash));
        assert_eq!(CommentStyle::for_path("config.toml"), Some(CommentStyle::Hash));
        assert_eq!(CommentStyle::for_path(".env"), Some(CommentStyle::Hash));
        assert_eq!(CommentStyle::for_path(".env.example"), Some(CommentStyle::Hash));
        assert_eq!(CommentStyle::for_path(".env.local"), Some(CommentStyle::Hash));
        assert_eq!(CommentStyle::for_path("path/to/Makefile"), None);
        assert_eq!(CommentStyle::for_path("README.md"), None);
        assert_eq!(CommentStyle::for_path("noext"), None);
    }

    #[test]
    fn second_region_implicitly_closes_first() {
        // Malformed input: region without endregion, followed by another region.
        let content = "\
// region: alpha
body a
// region: beta
body b
// endregion
";
        let s = parse(content, CommentStyle::DoubleSlash);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].name, "alpha");
        assert_eq!(s[0].lines, 1..3);
        assert_eq!(s[1].name, "beta");
        assert_eq!(s[1].lines, 3..6);
    }

    #[test]
    fn endregion_with_name_closes_correctly() {
        let content = "\
// region: alpha
body
// endregion alpha
";
        let s = parse(content, CommentStyle::DoubleSlash);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "alpha");
        assert_eq!(s[0].lines, 1..4);
    }

    #[test]
    fn indented_region_marker_recognised() {
        let content = "    // region: indented\n    body\n    // endregion\n";
        let s = parse(content, CommentStyle::DoubleSlash);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "indented");
    }
}
