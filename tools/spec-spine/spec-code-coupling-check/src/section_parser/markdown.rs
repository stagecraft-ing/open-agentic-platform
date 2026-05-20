//! Markdown ATX-heading section parser (spec 152 §2.1).
//!
//! Each ATX heading (`# Foo`, `## Foo`, `### Foo`, …) opens a section
//! whose anchor is the kebab-cased slug of the heading text. The
//! section extends until the next heading of the same or higher level
//! (lower `#` count), or EOF. Lower-level subheadings nested under a
//! parent heading remain inside the parent's range.
//!
//! Slug derivation matches the GitHub-style convention used by spec
//! 152 §2.1: lowercase, strip non-alphanumerics except dashes, collapse
//! whitespace and other separators into single `-`. The em-dash `—` and
//! hyphen-minus `-` collapse to `-`; runs of underscore/space/punctuation
//! collapse to a single `-`.
//!
//! Note: code fences (triple-backtick) hide heading-like lines inside
//! their span. A `#`-prefixed line inside a fenced block is NOT a
//! heading — it's content. This matters because many spec files
//! include ` ```yaml ` blocks containing `#` comments that would
//! otherwise be mis-parsed as headings.

use std::ops::Range;

/// A Markdown section with kebab-cased anchor and 1-based half-open line range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownSection {
    pub name: String,
    pub lines: Range<usize>,
}

/// Parse `content` (the full text of a Markdown file) into named sections.
///
/// Sections are emitted in document order. Lower-level subheadings nest
/// inside parents — every line belongs to the lowest-level (most specific)
/// section whose range covers it, but parent sections cover the union of
/// their own header line plus all nested content.
///
/// The current implementation emits one section per heading with its
/// own range. A hunk that overlaps both a parent and a child section is
/// attributed to both (callers can decide which to honour).
pub fn parse(content: &str) -> Vec<MarkdownSection> {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();

    let mut headings: Vec<(usize, usize, String)> = Vec::new(); // (lineno 1-based, level, slug)
    let mut in_fence = false;
    for (i, raw_line) in lines.iter().enumerate() {
        let lineno = i + 1;
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((level, text)) = parse_atx_heading(raw_line) {
            headings.push((lineno, level, slugify(text)));
        }
    }

    // For each heading, find the next heading at the same-or-higher level
    // (lower number); the range ends at that line - 1, half-open at that line.
    let mut sections: Vec<MarkdownSection> = Vec::with_capacity(headings.len());
    for (idx, (start, level, slug)) in headings.iter().enumerate() {
        let mut end = total + 1; // EOF (1-based half-open)
        for (next_start, next_level, _) in headings.iter().skip(idx + 1) {
            if *next_level <= *level {
                end = *next_start;
                break;
            }
        }
        sections.push(MarkdownSection {
            name: slug.clone(),
            lines: *start..end,
        });
    }
    sections
}

/// Parse an ATX heading line. Returns `(level, text)` where level is 1..=6.
/// Setext headings (underline-style) are NOT recognised — only ATX.
fn parse_atx_heading(line: &str) -> Option<(usize, &str)> {
    // ATX rule: up to 3 leading spaces, then 1-6 `#`, then a required space
    // (or end-of-line). Trailing `#` characters are stripped.
    let lstripped = line.trim_start_matches(' ');
    if line.len() - lstripped.len() > 3 {
        return None;
    }
    let hash_count = lstripped.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hash_count) {
        return None;
    }
    let after_hashes = &lstripped[hash_count..];
    // Empty heading (`#` followed by newline) is valid; non-empty headings
    // need a space.
    let text = match after_hashes.strip_prefix(' ') {
        Some(rest) => rest,
        None if after_hashes.is_empty() => "",
        None => return None,
    };
    let text = text.trim_end_matches('#').trim();
    Some((hash_count, text))
}

/// Kebab-case slug of an ATX heading's text.
///
/// Rules:
/// - lowercase ASCII letters and digits kept as-is.
/// - em-dash `—`, en-dash `–`, hyphen `-`, underscore, whitespace, punctuation
///   collapse to a single `-`.
/// - leading/trailing `-` are stripped.
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_was_dash = true; // suppress leading dash
    for ch in text.chars() {
        let lc = ch.to_ascii_lowercase();
        if lc.is_ascii_alphanumeric() {
            out.push(lc);
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_h2() {
        let content = "\
intro text
## Phase 1
phase 1 body
";
        let s = parse(content);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "phase-1");
        assert_eq!(s[0].lines.start, 2);
        // EOF: 3 lines → total=3, end=4
        assert_eq!(s[0].lines.end, 4);
    }

    #[test]
    fn h2_section_ends_at_next_h2() {
        let content = "\
## Alpha
alpha body
## Beta
beta body
";
        let s = parse(content);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].name, "alpha");
        assert_eq!(s[0].lines, 1..3);
        assert_eq!(s[1].name, "beta");
        assert_eq!(s[1].lines, 3..5);
    }

    #[test]
    fn h3_nests_inside_h2() {
        let content = "\
## Phase 2
intro
### Subheading
sub body
## Phase 3
";
        let s = parse(content);
        assert_eq!(s.len(), 3);
        // ## Phase 2 ends at ## Phase 3 (same-or-higher level).
        assert_eq!(s[0].name, "phase-2");
        assert_eq!(s[0].lines, 1..5);
        // ### Subheading ends at ## Phase 3.
        assert_eq!(s[1].name, "subheading");
        assert_eq!(s[1].lines, 3..5);
        assert_eq!(s[2].name, "phase-3");
    }

    #[test]
    fn em_dash_in_heading_collapses_to_dash() {
        let content = "## Phase 2 — Stagecraft API CRUD\n";
        let s = parse(content);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "phase-2-stagecraft-api-crud");
    }

    #[test]
    fn code_fence_hides_heading_lines() {
        let content = "\
## Real Section
```yaml
# this is yaml content, not a markdown heading
foo: bar
```
## Another Section
";
        let s = parse(content);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].name, "real-section");
        assert_eq!(s[1].name, "another-section");
    }

    #[test]
    fn slugify_drops_punctuation() {
        assert_eq!(slugify("Phase 2 — Stagecraft API CRUD"), "phase-2-stagecraft-api-crud");
        assert_eq!(slugify("CLI / Tools"), "cli-tools");
        assert_eq!(slugify("authority-derivation"), "authority-derivation");
        assert_eq!(slugify("  leading and trailing  "), "leading-and-trailing");
        assert_eq!(slugify(""), "");
        assert_eq!(slugify("---"), "");
    }

    #[test]
    fn empty_input_yields_no_sections() {
        assert!(parse("").is_empty());
        assert!(parse("no headings here\njust body\n").is_empty());
    }

    #[test]
    fn at_most_six_hashes_recognised() {
        // 7 hashes is not a heading (ATX max is 6).
        let content = "####### Not a heading\n## Real\n";
        let s = parse(content);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].name, "real");
    }

    #[test]
    fn heading_at_eof_extends_to_total_plus_one() {
        let content = "body\n## Last\n";
        let s = parse(content);
        assert_eq!(s.len(), 1);
        // total = 2 lines, EOF end = 3
        assert_eq!(s[0].lines, 2..3);
    }
}
