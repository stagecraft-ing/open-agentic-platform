//! Spec 152 §2.1 — GFM heading-slug anchors for `*.md` files.
//!
//! A section spans from its heading line (inclusive) to the line
//! before the next heading at the same or higher level (or EOF). The
//! slug is GFM's standard derivation: lowercase, spaces become `-`,
//! everything outside `[a-z0-9-]` is stripped. Multiple consecutive
//! `-` are NOT collapsed here — the corpus does not exercise that
//! corner today; the rule lands when a real spec needs it.

use super::{AnchorParser, AnchorResult};
use crate::types::LineSpan;

pub struct MarkdownHeadingParser;

impl AnchorParser for MarkdownHeadingParser {
    fn find_anchor(&self, content: &str, anchor: &str) -> AnchorResult {
        let lines: Vec<&str> = content.lines().collect();
        let mut start: Option<(u32, usize)> = None;
        for (idx, line) in lines.iter().enumerate() {
            let lineno = (idx as u32) + 1;
            if let Some((level, text)) = parse_atx_heading(line) {
                let slug = gfm_slug(text);
                if let Some((s, s_level)) = start {
                    if level <= s_level {
                        let end = last_nonblank_line(&lines, idx, s as usize);
                        return Ok(Some(LineSpan {
                            start_line: s,
                            end_line: end,
                        }));
                    }
                    // Nested heading — continues to be part of the
                    // current section.
                    let _ = slug;
                    continue;
                }
                if slug == anchor {
                    start = Some((lineno, level));
                }
            }
        }
        match start {
            Some((s, _)) => {
                let end = last_nonblank_line(&lines, lines.len(), s as usize);
                Ok(Some(LineSpan {
                    start_line: s,
                    end_line: end,
                }))
            }
            None => Ok(None),
        }
    }
}

/// Walk backwards from `end_exclusive` (an exclusive 0-indexed upper
/// bound) until a non-blank line is found. Returns the 1-indexed line
/// number. Lower-bounded by `start_line_1based`.
fn last_nonblank_line(lines: &[&str], end_exclusive: usize, start_line_1based: usize) -> u32 {
    let mut i = end_exclusive;
    while i > start_line_1based {
        if !lines[i - 1].trim().is_empty() {
            return i as u32;
        }
        i -= 1;
    }
    start_line_1based as u32
}

/// Parse an ATX heading line (`#` ... `######`). Returns
/// `(level, text)` where `text` has leading/trailing whitespace
/// trimmed.
fn parse_atx_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let bytes = trimmed.as_bytes();
    let mut level = 0;
    while level < bytes.len() && bytes[level] == b'#' {
        level += 1;
    }
    if level == 0 || level > 6 {
        return None;
    }
    // The character after the `#` run must be a space or EOL.
    if level < bytes.len() && bytes[level] != b' ' && bytes[level] != b'\t' {
        return None;
    }
    let text = trimmed[level..].trim();
    Some((level, text))
}

/// GFM-style slug: lowercase, spaces → `-`, strip everything that
/// isn't `[a-z0-9-]`. Inline backticks / formatting markers are
/// stripped (they're non-alphanumeric).
fn gfm_slug(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == ' ' || c == '-' || c == '_' {
            out.push('-');
        }
        // Everything else is dropped.
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# Title

intro

## Overview

paragraph one

## Configuration

config body line 1
config body line 2

### Sub-config

sub body

## Last

last body
";

    #[test]
    fn finds_section_to_next_same_level() {
        let p = MarkdownHeadingParser;
        let span = p.find_anchor(SAMPLE, "configuration").unwrap().unwrap();
        assert_eq!(span.start_line, 9);
        // ends before `## Last` (line 17, since lines are 1-indexed)
        assert_eq!(span.end_line, 16);
    }

    #[test]
    fn finds_section_to_eof() {
        let p = MarkdownHeadingParser;
        let span = p.find_anchor(SAMPLE, "last").unwrap().unwrap();
        assert_eq!(span.start_line, 18);
        assert_eq!(span.end_line, 20);
    }

    #[test]
    fn missing_anchor_returns_none() {
        let p = MarkdownHeadingParser;
        assert!(p.find_anchor(SAMPLE, "absent").unwrap().is_none());
    }

    #[test]
    fn slugify_lowercases_and_strips() {
        assert_eq!(gfm_slug("Sub-config"), "sub-config");
        assert_eq!(gfm_slug("Run `make ci`"), "run-make-ci");
        assert_eq!(gfm_slug("Configuration"), "configuration");
    }
}
