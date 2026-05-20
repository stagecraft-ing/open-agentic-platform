//! Makefile section parser (spec 152 §2.1).
//!
//! Parses a Makefile into named sections with 1-based line ranges.
//!
//! Three anchor forms are recognised (in priority order):
//!
//! 1. **`## tag: <name>`** — explicit tag comment. Opens a new section
//!    named `<name>` at the comment's line. The section extends until
//!    the next `## tag:` line, a `# BEGIN`/`# END` boundary, or EOF.
//!
//! 2. **`# BEGIN <name>`** / **`# END <name>`** — explicit begin/end
//!    sentinels (the convention used by `ci-parity-check`). The section
//!    spans the lines strictly between the sentinel pair. Nested pairs
//!    are not supported; the first `# END` closes the current `# BEGIN`.
//!
//! 3. **First target name in a target group** — a line whose first
//!    non-space token ends with `:` (and is not a variable assignment or
//!    a recipe line) opens a section whose name is the text before the
//!    first `:`. This fallback fires only when the line falls outside
//!    any `## tag:` or `# BEGIN`/`# END` section.
//!
//! Line numbers are 1-based and ranges are half-open `[start, end)`.

use std::ops::Range;

/// A named section within a Makefile with a 1-based half-open line range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MakefileSection {
    /// Section name: the tag value, BEGIN label, or first-target name.
    pub name: String,
    /// 1-based half-open range `[start, end)` covering the section's lines.
    pub lines: Range<usize>,
}

/// Parse `content` (the full text of a Makefile) into a list of named
/// sections in document order. Overlapping sections are not possible by
/// design; `# BEGIN`/`# END` blocks are disjoint from tag-anchored and
/// target-anchored sections.
pub fn parse(content: &str) -> Vec<MakefileSection> {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let mut sections: Vec<MakefileSection> = Vec::new();

    // First pass: collect `# BEGIN <name>` / `# END <name>` blocks.
    // These are highest priority — lines inside a BEGIN/END block are
    // claimed before target-name or tag anchoring applies.
    let begin_end_sections = collect_begin_end(&lines);

    // Second pass: collect `## tag: <name>` sections.
    let tag_sections = collect_tag_sections(&lines, total);

    // Third pass: collect target-name anchored groups for lines not
    // already covered by a BEGIN/END or tag section.
    let target_sections = collect_target_sections(&lines, total, &begin_end_sections, &tag_sections);

    sections.extend(begin_end_sections);
    sections.extend(tag_sections);
    sections.extend(target_sections);

    // Sort by start line for deterministic ordering.
    sections.sort_by_key(|s| s.lines.start);
    sections
}

/// Returns true when `line_no` (1-based) falls inside any of `sections`.
fn covered_by(line_no: usize, sections: &[MakefileSection]) -> bool {
    sections.iter().any(|s| s.lines.contains(&line_no))
}

/// Collect `# BEGIN <name>` / `# END <name>` block sections.
fn collect_begin_end(lines: &[&str]) -> Vec<MakefileSection> {
    let mut out = Vec::new();
    let mut open: Option<(String, usize)> = None; // (name, start_line 1-based)

    for (i, &line) in lines.iter().enumerate() {
        let lineno = i + 1;
        let trimmed = line.trim();

        if let Some(name) = parse_begin(trimmed) {
            // Close any previously open block (malformed nesting — treat as
            // a new open).
            if let Some((prev_name, prev_start)) = open.take() {
                out.push(MakefileSection {
                    name: prev_name,
                    lines: prev_start..lineno,
                });
            }
            open = Some((name, lineno));
        } else if let Some(name) = parse_end(trimmed) {
            if let Some((open_name, start)) = open.take() {
                if open_name == name {
                    // +1: the END line itself is included in the range.
                    out.push(MakefileSection {
                        name: open_name,
                        lines: start..lineno + 1,
                    });
                } else {
                    // Mismatched END — close the current open block anyway.
                    out.push(MakefileSection {
                        name: open_name,
                        lines: start..lineno + 1,
                    });
                }
            }
        }
    }

    // Unclosed block: extend to EOF.
    if let Some((name, start)) = open {
        out.push(MakefileSection {
            name,
            lines: start..lines.len() + 1,
        });
    }

    out
}

/// Collect `## tag: <name>` sections. Each tag extends until the next
/// `## tag:` line or EOF.
fn collect_tag_sections(lines: &[&str], total: usize) -> Vec<MakefileSection> {
    let mut out = Vec::new();
    let mut open: Option<(String, usize)> = None;

    for (i, &line) in lines.iter().enumerate() {
        let lineno = i + 1;
        let trimmed = line.trim();
        if let Some(name) = parse_tag(trimmed) {
            if let Some((prev_name, prev_start)) = open.take() {
                out.push(MakefileSection {
                    name: prev_name,
                    lines: prev_start..lineno,
                });
            }
            open = Some((name, lineno));
        }
    }

    if let Some((name, start)) = open {
        out.push(MakefileSection {
            name,
            lines: start..total + 1,
        });
    }

    out
}

/// Collect target-name anchored sections for lines not covered by
/// `begin_end` or `tag` sections.
fn collect_target_sections(
    lines: &[&str],
    total: usize,
    begin_end: &[MakefileSection],
    tag_sections: &[MakefileSection],
) -> Vec<MakefileSection> {
    let mut out = Vec::new();
    let mut open: Option<(String, usize)> = None;

    for (i, &line) in lines.iter().enumerate() {
        let lineno = i + 1;

        // Skip lines already covered by higher-priority sections.
        if covered_by(lineno, begin_end) || covered_by(lineno, tag_sections) {
            // Close any open target-section before skipping.
            if let Some((name, start)) = open.take() {
                out.push(MakefileSection {
                    name,
                    lines: start..lineno,
                });
            }
            continue;
        }

        if let Some(target) = parse_target_name(line) {
            // New target: close the previous group.
            if let Some((name, start)) = open.take() {
                out.push(MakefileSection {
                    name,
                    lines: start..lineno,
                });
            }
            open = Some((target, lineno));
        }
    }

    if let Some((name, start)) = open {
        out.push(MakefileSection {
            name,
            lines: start..total + 1,
        });
    }

    out
}

// ─── Anchor-line parsers ──────────────────────────────────────────────────────

/// `# BEGIN <name>` or `# BEGIN <name> (...)` — returns `Some(name)`.
fn parse_begin(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("# BEGIN ")?;
    // Name is the first whitespace-delimited token.
    let name = rest.split_whitespace().next()?;
    Some(name.to_string())
}

/// `# END <name>` — returns `Some(name)`.
fn parse_end(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("# END ")?;
    let name = rest.split_whitespace().next()?;
    Some(name.to_string())
}

/// `## tag: <name>` — returns `Some(name)`.
fn parse_tag(trimmed: &str) -> Option<String> {
    let rest = trimmed.strip_prefix("## tag: ")?;
    let name = rest.split_whitespace().next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Returns the target name if `line` is a Makefile target definition
/// (e.g. `ci-spec-code-coupling:`). Returns `None` for recipe lines
/// (start with a tab), variable assignments, and `.PHONY` declarations.
fn parse_target_name(line: &str) -> Option<String> {
    // Recipe lines start with a hard tab.
    if line.starts_with('\t') {
        return None;
    }
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    // Variable assignment: `VAR = val`, `VAR := val`, `VAR ?= val`, `VAR += val`
    if trimmed.contains('=') {
        let before_eq = trimmed.split('=').next().unwrap_or("");
        if !before_eq.contains(':') {
            return None; // pure assignment
        }
        // Could be `target: var=val` but that's rare; treat as target if
        // colon precedes `=`.
    }
    // Must end with `:` or `<name>: deps...`
    let colon_pos = trimmed.find(':')?;
    let name = trimmed[..colon_pos].trim();
    if name.is_empty() || name.contains(' ') || name.starts_with('.') {
        return None;
    }
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_begin_end_blocks() {
        let content = "\
preamble-target:
\t@echo preamble

# BEGIN ci-fast (spec 134)
ci-fast-rust:
\t@echo rust

ci-fast-tools:
\t@echo tools

# END ci-fast

post-target:
\t@echo post
";
        let sections = parse(content);
        let ci_fast = sections.iter().find(|s| s.name == "ci-fast");
        assert!(ci_fast.is_some(), "expected ci-fast section; got: {sections:?}");
        let s = ci_fast.unwrap();
        // BEGIN is on line 4, END is on line 10 → range 4..11
        assert_eq!(s.lines.start, 4, "start line mismatch");
        assert!(s.lines.contains(&5), "ci-fast-rust should be in section");
        assert!(s.lines.contains(&7), "ci-fast-tools should be in section");
    }

    #[test]
    fn parses_tag_sections() {
        let content = "\
## tag: spec-code-coupling
ci-spec-code-coupling:
\t@echo coupling

## tag: supply-chain
ci-supply-chain:
\t@echo supply
";
        let sections = parse(content);
        let sc = sections.iter().find(|s| s.name == "spec-code-coupling");
        assert!(sc.is_some(), "expected spec-code-coupling; got {sections:?}");
        assert!(sc.unwrap().lines.contains(&2));

        let sup = sections.iter().find(|s| s.name == "supply-chain");
        assert!(sup.is_some());
        assert!(sup.unwrap().lines.contains(&6));
    }

    #[test]
    fn parses_target_name_fallback() {
        let content = "\
my-target:
\t@echo hello

another-target: dep
\t@echo world
";
        let sections = parse(content);
        let my = sections.iter().find(|s| s.name == "my-target");
        assert!(my.is_some(), "expected my-target; got {sections:?}");
        let another = sections.iter().find(|s| s.name == "another-target");
        assert!(another.is_some());
    }

    #[test]
    fn begin_end_takes_priority_over_target_names() {
        let content = "\
# BEGIN ci-fast
inner-target:
\t@echo inside

# END ci-fast
outer-target:
\t@echo outside
";
        let sections = parse(content);
        // inner-target inside the BEGIN/END block must NOT produce a separate
        // target-name section.
        let inner_target = sections.iter().find(|s| s.name == "inner-target");
        assert!(
            inner_target.is_none(),
            "inner-target inside BEGIN/END should not produce its own section"
        );
        // The ci-fast section should be present.
        assert!(sections.iter().any(|s| s.name == "ci-fast"));
        // outer-target after END should produce its own section.
        assert!(sections.iter().any(|s| s.name == "outer-target"));
    }

    #[test]
    fn parse_begin_extracts_name() {
        assert_eq!(parse_begin("# BEGIN ci-fast (spec 134)"), Some("ci-fast".to_string()));
        assert_eq!(parse_begin("# BEGIN spec-code-coupling"), Some("spec-code-coupling".to_string()));
        assert_eq!(parse_begin("# not a begin"), None);
    }

    #[test]
    fn parse_end_extracts_name() {
        assert_eq!(parse_end("# END ci-fast"), Some("ci-fast".to_string()));
        assert_eq!(parse_end("# END"), None);
    }

    #[test]
    fn parse_tag_extracts_name() {
        assert_eq!(parse_tag("## tag: spec-code-coupling"), Some("spec-code-coupling".to_string()));
        assert_eq!(parse_tag("## tag: "), None);
        assert_eq!(parse_tag("# tag: not-a-double-hash"), None);
    }

    #[test]
    fn parse_target_name_rejects_recipe_and_comments() {
        assert_eq!(parse_target_name("\t@echo hi"), None);
        assert_eq!(parse_target_name("# a comment"), None);
        assert_eq!(parse_target_name("VAR = value"), None);
        assert_eq!(parse_target_name("VAR ?= value"), None);
        assert_eq!(parse_target_name(""), None);
        // Valid target.
        assert_eq!(
            parse_target_name("ci-spec-code-coupling:"),
            Some("ci-spec-code-coupling".to_string())
        );
        // Target with deps.
        assert_eq!(
            parse_target_name("pr-prep: index ci-fast"),
            Some("pr-prep".to_string())
        );
    }
}
