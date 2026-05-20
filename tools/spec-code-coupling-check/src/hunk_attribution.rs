//! Diff-hunk to section attribution (spec 152 §2.2).
//!
//! Given a diff hunk (file path + affected line range) and a parsed section
//! list for that file, this module answers the question: *which section(s)
//! does this hunk overlap?*
//!
//! **Scope for this session (partial activation, spec 152 §2)**:
//! only Makefile section parsing is implemented. For every other file type
//! the module returns an empty set, which triggers the whole-file-authority
//! fallback in the coupling gate.

use std::collections::BTreeSet;
use std::ops::Range;

use crate::section_parser::makefile::{self, MakefileSection};

/// Sections attributed to a single hunk.
///
/// An empty set means the hunk falls outside every named section —
/// the gate falls back to whole-file authority for this hunk.
pub type HunkSections = BTreeSet<String>;

/// A pre-attributed diff: the result of running section-attribution over
/// every hunk in a diff. Keyed by file path; values are the union of all
/// sections touched by any hunk in that file.
///
/// The gate builds this from the raw diff + parsed file content.
/// Tests may construct it directly.
pub type HunkAttributionMap = std::collections::BTreeMap<String, HunkSections>;

/// Return the set of section names that the half-open line range
/// `hunk_lines` overlaps in the pre-parsed `sections` list.
///
/// `hunk_lines` is 1-based and half-open `[start, end)`, matching the
/// representation used by `MakefileSection.lines`.
pub fn sections_for_hunk(
    hunk_lines: &Range<usize>,
    sections: &[MakefileSection],
) -> HunkSections {
    sections
        .iter()
        .filter(|s| ranges_overlap(hunk_lines, &s.lines))
        .map(|s| s.name.clone())
        .collect()
}

/// Return the union of section names for ALL hunks in `hunk_ranges`,
/// relative to the pre-parsed `sections`.
pub fn sections_for_hunks(
    hunk_ranges: &[Range<usize>],
    sections: &[MakefileSection],
) -> HunkSections {
    hunk_ranges
        .iter()
        .flat_map(|r| sections_for_hunk(r, sections))
        .collect()
}

/// Attribute all hunks for `file_path` given the raw file content.
///
/// Returns `None` when the file type has no parser (whole-file fallback
/// applies). Returns `Some(set)` for Makefile paths, where `set` may be
/// empty if no hunk falls inside a named section.
///
/// The `is_makefile` test is path-based: a path whose last component is
/// literally `Makefile` (case-sensitive) uses the Makefile parser.
pub fn attribute_hunks_for_file(
    file_path: &str,
    file_content: &str,
    hunk_ranges: &[Range<usize>],
) -> Option<HunkSections> {
    if is_makefile_path(file_path) {
        let parsed = makefile::parse(file_content);
        Some(sections_for_hunks(hunk_ranges, &parsed))
    } else {
        // All other file types: no parser in this session.
        None
    }
}

fn is_makefile_path(path: &str) -> bool {
    path == "Makefile"
        || path.ends_with("/Makefile")
        || std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "Makefile")
            .unwrap_or(false)
}

/// True when two half-open ranges share at least one line.
fn ranges_overlap(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(start: usize, end: usize) -> Range<usize> {
        start..end
    }

    fn section(name: &str, start: usize, end: usize) -> MakefileSection {
        MakefileSection {
            name: name.to_string(),
            lines: start..end,
        }
    }

    #[test]
    fn hunk_fully_inside_section() {
        let sections = vec![section("spec-code-coupling", 5, 20)];
        let result = sections_for_hunk(&range(7, 12), &sections);
        assert_eq!(result, BTreeSet::from(["spec-code-coupling".to_string()]));
    }

    #[test]
    fn hunk_outside_all_sections() {
        let sections = vec![section("spec-code-coupling", 5, 20)];
        let result = sections_for_hunk(&range(21, 25), &sections);
        assert!(result.is_empty());
    }

    #[test]
    fn hunk_spanning_two_sections() {
        let sections = vec![
            section("section-a", 1, 10),
            section("section-b", 10, 20),
        ];
        let result = sections_for_hunk(&range(8, 15), &sections);
        assert!(result.contains("section-a"));
        assert!(result.contains("section-b"));
    }

    #[test]
    fn hunk_touching_boundary() {
        let sections = vec![section("spec-code-coupling", 5, 15)];
        // Half-open: range 15..20 does NOT overlap 5..15.
        let no_overlap = sections_for_hunk(&range(15, 20), &sections);
        assert!(no_overlap.is_empty(), "expected empty; got {no_overlap:?}");
        // But 14..16 does overlap.
        let overlap = sections_for_hunk(&range(14, 16), &sections);
        assert!(overlap.contains("spec-code-coupling"));
    }

    #[test]
    fn non_makefile_path_returns_none() {
        let result = attribute_hunks_for_file(
            "crates/orchestrator/src/lib.rs",
            "fn main() {}",
            &[range(1, 5)],
        );
        assert!(result.is_none(), "non-Makefile should return None");
    }

    #[test]
    fn makefile_path_returns_some() {
        let content = "## tag: spec-code-coupling\nci-spec-code-coupling:\n\t@echo hi\n";
        let result = attribute_hunks_for_file("Makefile", content, &[range(2, 3)]);
        assert!(result.is_some());
        let sections = result.unwrap();
        assert!(sections.contains("spec-code-coupling"), "got: {sections:?}");
    }

    #[test]
    fn sections_for_hunks_union() {
        let sections = vec![
            section("alpha", 1, 5),
            section("beta", 5, 10),
        ];
        let ranges = vec![range(2, 3), range(6, 8)];
        let result = sections_for_hunks(&ranges, &sections);
        assert!(result.contains("alpha"));
        assert!(result.contains("beta"));
    }

    #[test]
    fn is_makefile_path_variants() {
        assert!(is_makefile_path("Makefile"));
        assert!(is_makefile_path("sub/Makefile"));
        assert!(!is_makefile_path("GNUmakefile"));
        assert!(!is_makefile_path("makefile"));
        assert!(!is_makefile_path("crates/foo/src/lib.rs"));
    }
}
