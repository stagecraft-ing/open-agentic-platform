//! Spec 152 §2.1 — `.github/workflows/*.yml` anchors keyed
//! `jobs.<name>`.
//!
//! Implementation uses a line-based scan keyed on YAML's
//! indentation contract. The dispatcher targets only files under
//! `.github/workflows/`; for any other YAML the corpus uses today
//! (none yet), the parser still behaves: it finds the top-level
//! `jobs:` mapping, then the keyed job, then walks until the next
//! sibling at the same indent.
//!
//! Why line-scan over a full YAML parse: `serde_yaml` does not
//! preserve line numbers in its parsed model. A round-trip-and-locate
//! would need either a different parser (`yaml-rust2` retains
//! positions) or a second pass. The line-scan is short, deterministic,
//! and treats indent the way the YAML format itself does for block
//! mappings, which is the only shape `.github/workflows/` uses.

use super::{AnchorParser, AnchorResult};
use crate::types::LineSpan;

pub struct WorkflowYamlAnchorParser;

impl AnchorParser for WorkflowYamlAnchorParser {
    fn find_anchor(&self, content: &str, anchor: &str) -> AnchorResult {
        // Anchor grammar: "jobs.<name>" — required prefix
        // disambiguates from arbitrary nested keys.
        let job_name = match anchor.strip_prefix("jobs.") {
            Some(rest) => rest,
            None => anchor, // tolerate bare name; matches `jobs.<name>` semantics
        };
        let lines: Vec<&str> = content.lines().collect();

        // Locate top-level `jobs:` line.
        let Some(jobs_line) = lines.iter().position(|line| {
            let trimmed = line.trim_start();
            line.len() - trimmed.len() == 0 && trimmed.starts_with("jobs:")
        }) else {
            return Ok(None);
        };

        // Find the job-name key inside the jobs mapping. The job-name
        // key sits at the indent depth immediately below `jobs:`. We
        // discover that depth from the first non-blank, non-comment
        // line after `jobs:`.
        let mut job_indent: Option<usize> = None;
        let mut start: Option<u32> = None;
        for (idx, line) in lines.iter().enumerate().skip(jobs_line + 1) {
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            if job_indent.is_none() {
                if indent == 0 {
                    // Reached a top-level sibling of `jobs:` without
                    // finding any job entry.
                    return Ok(None);
                }
                job_indent = Some(indent);
            }
            let depth = job_indent.unwrap();
            if indent < depth {
                // Left the jobs mapping entirely.
                break;
            }
            if indent == depth {
                let key = line[depth..]
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .trim();
                if let Some(s) = start {
                    // Next sibling — close the span on the last
                    // non-blank line before idx.
                    let end = last_nonblank_line(&lines, idx, s as usize);
                    return Ok(Some(LineSpan {
                        start_line: s,
                        end_line: end,
                    }));
                }
                if key == job_name {
                    start = Some((idx as u32) + 1);
                }
            }
        }
        // EOF — close at last non-empty content line.
        match start {
            Some(s) => {
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

/// Walk backwards from `end_exclusive` (an exclusive upper bound,
/// already converted from 0-indexed `idx`) until a non-blank line is
/// found. Returns the 1-indexed line number. Lower-bounded by
/// `start_line_1based` so the span is never inverted.
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

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
name: ci
on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build

  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test
";

    #[test]
    fn finds_first_job() {
        let p = WorkflowYamlAnchorParser;
        let span = p.find_anchor(SAMPLE, "jobs.build").unwrap().unwrap();
        assert_eq!(span.start_line, 5);
        assert_eq!(span.end_line, 9);
    }

    #[test]
    fn finds_last_job_to_eof() {
        let p = WorkflowYamlAnchorParser;
        let span = p.find_anchor(SAMPLE, "jobs.test").unwrap().unwrap();
        assert_eq!(span.start_line, 11);
        // EOF trim brings end to last non-empty line.
        assert_eq!(span.end_line, 15);
    }

    #[test]
    fn missing_anchor_returns_none() {
        let p = WorkflowYamlAnchorParser;
        assert!(p.find_anchor(SAMPLE, "jobs.absent").unwrap().is_none());
    }
}
