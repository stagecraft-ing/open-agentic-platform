//! Spec 152 §2.1 / spec 154 §3.4 — Makefile section anchors.
//!
//! Anchor syntax: `## tag: <name>` on a line of its own. A section
//! starts on the tag line itself (inclusive) and ends on the line
//! before the next `## tag:` comment (or EOF). Inclusion of the tag
//! line follows spec 152 §2.2: "H falls within section S if H's line
//! numbers are between the `## tag: S` line and the next `## tag:`
//! line."

use super::{AnchorParser, AnchorResult};
use crate::types::LineSpan;

pub struct MakefileAnchorParser;

impl AnchorParser for MakefileAnchorParser {
    fn find_anchor(&self, content: &str, anchor: &str) -> AnchorResult {
        // Walk lines once. When we find the matching `## tag:` line,
        // start recording; when we find the next `## tag:` (any name)
        // or hit EOF, close the span.
        let mut start: Option<u32> = None;
        let mut end_line: u32 = 0;
        for (idx, line) in content.lines().enumerate() {
            let lineno = (idx as u32) + 1;
            end_line = lineno;
            if let Some(tag) = parse_tag_line(line) {
                if start.is_some() {
                    // Closing on the next tag — end at the previous line.
                    return Ok(Some(LineSpan {
                        start_line: start.unwrap(),
                        end_line: lineno - 1,
                    }));
                }
                if tag == anchor {
                    start = Some(lineno);
                }
            }
        }
        // EOF without another tag.
        match start {
            Some(s) => Ok(Some(LineSpan {
                start_line: s,
                end_line: end_line.max(s),
            })),
            None => Ok(None),
        }
    }
}

/// Recognise `## tag: <name>` (with arbitrary leading whitespace and
/// trailing comment text after `<name>` ignored). Returns the
/// tag-name string. Other comment forms (`#`, `##`, `## comment text`)
/// return `None`.
fn parse_tag_line(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("##")?.trim_start();
    let rest = rest.strip_prefix("tag:")?.trim_start();
    // Tag name runs to the next whitespace; everything past that is
    // free-form comment.
    let name_end = rest
        .find(|c: char| c.is_whitespace())
        .unwrap_or(rest.len());
    let name = &rest[..name_end];
    if name.is_empty() { None } else { Some(name) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
header-content
\t@echo header

## tag: setup
setup:
\t@echo set up

## tag: deploy
deploy:
\t@echo deploy 1
\t@echo deploy 2

footer:
\t@echo footer
";

    #[test]
    fn finds_first_tag() {
        let p = MakefileAnchorParser;
        let span = p.find_anchor(SAMPLE, "setup").unwrap().unwrap();
        assert_eq!(span.start_line, 4);
        assert_eq!(span.end_line, 7);
    }

    #[test]
    fn finds_last_tag_to_eof() {
        let p = MakefileAnchorParser;
        let span = p.find_anchor(SAMPLE, "deploy").unwrap().unwrap();
        assert_eq!(span.start_line, 8);
        assert_eq!(span.end_line, 14);
    }

    #[test]
    fn missing_anchor_returns_none() {
        let p = MakefileAnchorParser;
        assert!(p.find_anchor(SAMPLE, "absent").unwrap().is_none());
    }
}
