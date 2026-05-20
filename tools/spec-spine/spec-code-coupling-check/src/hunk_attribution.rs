//! Diff-hunk to section attribution (spec 152 §2.2).
//!
//! Given a diff hunk (file path + affected line range) and a parsed
//! section list for that file, this module answers: *which section(s)
//! does this hunk overlap?*
//!
//! The dispatch rules below are normative per spec 152 §2.1:
//!
//! - `Makefile` (any path whose basename is `Makefile`) → [`section_parser::makefile`].
//! - `*.md` → [`section_parser::markdown`].
//! - `*.{rs,ts,tsx,js,jsx,sh,yaml,yml,toml,env*,...}` with `region:`
//!   markers → [`section_parser::region`] with per-language comment
//!   prefix detection.
//! - Anything else, or a path whose detected parser yields no sections,
//!   → no section attribution; the gate falls back to whole-file authority
//!   for those hunks (spec 152 §2.2 last paragraph).

use std::collections::BTreeSet;
use std::ops::Range;

use crate::section_parser::{
    makefile::{self, MakefileSection},
    markdown::{self, MarkdownSection},
    region::{self, CommentStyle, RegionSection},
};

/// Sections attributed to a single hunk.
///
/// An empty set means the hunk falls outside every named section —
/// the gate falls back to whole-file authority for this hunk.
pub type HunkSections = BTreeSet<String>;

/// A pre-attributed diff: the result of running section-attribution over
/// every hunk in a diff. Keyed by file path; values are the union of all
/// sections touched by any hunk in that file.
pub type HunkAttributionMap = std::collections::BTreeMap<String, HunkSections>;

/// Trait erasing the per-parser section type so [`sections_for_hunks`]
/// can iterate over any parser's output uniformly.
trait HasLineRange {
    fn name(&self) -> &str;
    fn lines(&self) -> &Range<usize>;
}

impl HasLineRange for MakefileSection {
    fn name(&self) -> &str { &self.name }
    fn lines(&self) -> &Range<usize> { &self.lines }
}

impl HasLineRange for MarkdownSection {
    fn name(&self) -> &str { &self.name }
    fn lines(&self) -> &Range<usize> { &self.lines }
}

impl HasLineRange for RegionSection {
    fn name(&self) -> &str { &self.name }
    fn lines(&self) -> &Range<usize> { &self.lines }
}

/// Return the set of section names that `hunk_lines` overlaps in
/// `sections` (sections come from a single parser, so they share a
/// section type).
fn sections_for_hunk_typed<S: HasLineRange>(
    hunk_lines: &Range<usize>,
    sections: &[S],
) -> HunkSections {
    sections
        .iter()
        .filter(|s| ranges_overlap(hunk_lines, s.lines()))
        .map(|s| s.name().to_string())
        .collect()
}

fn sections_for_hunks_typed<S: HasLineRange>(
    hunk_ranges: &[Range<usize>],
    sections: &[S],
) -> HunkSections {
    hunk_ranges
        .iter()
        .flat_map(|r| sections_for_hunk_typed(r, sections))
        .collect()
}

/// Back-compat helper for the Makefile-only call sites used in earlier
/// tests. New code should call [`attribute_hunks_for_file`].
pub fn sections_for_hunk(
    hunk_lines: &Range<usize>,
    sections: &[MakefileSection],
) -> HunkSections {
    sections_for_hunk_typed(hunk_lines, sections)
}

/// Back-compat helper for the Makefile-only call sites used in earlier
/// tests. New code should call [`attribute_hunks_for_file`].
pub fn sections_for_hunks(
    hunk_ranges: &[Range<usize>],
    sections: &[MakefileSection],
) -> HunkSections {
    sections_for_hunks_typed(hunk_ranges, sections)
}

/// Attribute all hunks for `file_path` given the raw file content.
///
/// Returns `None` when the file type has no parser support (caller falls
/// back to whole-file authority). Returns `Some(set)` for supported file
/// types; the set may be empty if no hunk falls inside a named section.
pub fn attribute_hunks_for_file(
    file_path: &str,
    file_content: &str,
    hunk_ranges: &[Range<usize>],
) -> Option<HunkSections> {
    if is_makefile_path(file_path) {
        let parsed = makefile::parse(file_content);
        return Some(sections_for_hunks_typed(hunk_ranges, &parsed));
    }
    if is_markdown_path(file_path) {
        let parsed = markdown::parse(file_content);
        return Some(sections_for_hunks_typed(hunk_ranges, &parsed));
    }
    if let Some(style) = CommentStyle::for_path(file_path) {
        let parsed = region::parse(file_content, style);
        return Some(sections_for_hunks_typed(hunk_ranges, &parsed));
    }
    None
}

fn is_makefile_path(path: &str) -> bool {
    let basename = path.rsplit('/').next().unwrap_or(path);
    basename == "Makefile"
}

fn is_markdown_path(path: &str) -> bool {
    path.to_ascii_lowercase().ends_with(".md")
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
        let no_overlap = sections_for_hunk(&range(15, 20), &sections);
        assert!(no_overlap.is_empty(), "expected empty; got {no_overlap:?}");
        let overlap = sections_for_hunk(&range(14, 16), &sections);
        assert!(overlap.contains("spec-code-coupling"));
    }

    #[test]
    fn non_supported_path_returns_none() {
        // Random binary / unknown extension.
        let result = attribute_hunks_for_file(
            "vendor/blob.bin",
            "fn main() {}",
            &[range(1, 5)],
        );
        assert!(result.is_none(), "unknown extension should return None");
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
    fn markdown_path_returns_some() {
        let content = "## Section One\nbody one\n## Section Two\nbody two\n";
        let result = attribute_hunks_for_file("docs/foo.md", content, &[range(2, 3)]);
        assert!(result.is_some());
        let sections = result.unwrap();
        assert!(sections.contains("section-one"), "got: {sections:?}");
    }

    #[test]
    fn rust_region_path_returns_some() {
        let content = "// region: section-matching\nfn x() {}\n// endregion\n";
        let result = attribute_hunks_for_file(
            "tools/spec-code-coupling-check/src/lib.rs",
            content,
            &[range(2, 3)],
        );
        assert!(result.is_some());
        let sections = result.unwrap();
        assert!(sections.contains("section-matching"), "got: {sections:?}");
    }

    #[test]
    fn shell_region_path_returns_some() {
        let content = "#!/bin/bash\n# region: bootstrap\necho hi\n# endregion\n";
        let result = attribute_hunks_for_file(
            "platform/infra/hetzner/setup.sh",
            content,
            &[range(3, 4)],
        );
        assert!(result.is_some());
        let sections = result.unwrap();
        assert!(sections.contains("bootstrap"), "got: {sections:?}");
    }

    #[test]
    fn yaml_region_path_returns_some() {
        let content = "key: val\n# region: access-gate\nfoo: bar\n# endregion\n";
        let result = attribute_hunks_for_file(
            "platform/charts/tenant-hello/values.yaml",
            content,
            &[range(3, 4)],
        );
        assert!(result.is_some());
        let sections = result.unwrap();
        assert!(sections.contains("access-gate"), "got: {sections:?}");
    }

    #[test]
    fn env_region_path_returns_some() {
        let content = "# region: env-vars\nVAR=1\n# endregion\n";
        let result = attribute_hunks_for_file(
            "platform/infra/hetzner/.env.example",
            content,
            &[range(2, 3)],
        );
        assert!(result.is_some());
        let sections = result.unwrap();
        assert!(sections.contains("env-vars"), "got: {sections:?}");
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

    #[test]
    fn is_markdown_path_variants() {
        assert!(is_markdown_path("README.md"));
        assert!(is_markdown_path("docs/architecture.md"));
        assert!(is_markdown_path("specs/127/spec.MD"));
        assert!(!is_markdown_path("foo.markdown"));
        assert!(!is_markdown_path("Makefile"));
    }
}
