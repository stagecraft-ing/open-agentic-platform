//! File-type section parsers for diff-to-section attribution (spec 152 §2).
//!
//! Each submodule implements anchor enumeration for one file-type
//! family. Per-file-type schemes are normative in spec 152 §2.1:
//!
//! - [`makefile`] — `## tag: <name>` comments, `# BEGIN`/`# END` block
//!   sentinels, and target-name fallback.
//! - [`markdown`] — ATX headings (`## Heading`) with kebab-cased slug
//!   anchors. Nested subheadings remain inside their parent's range.
//! - [`region`] — `region: <name>` / `endregion` comment markers, with
//!   per-language comment-prefix detection (`//` for Rust/TS, `#` for
//!   shell/env/YAML/TOML). Covers the "Other source files — Same region
//!   convention" line of spec 152 §2.1.
//!
//! Workflow-YAML `jobs.<name>` parsing is intentionally absent: the
//! corpus has no `co_authority:` annotation targeting `.github/workflows/*.{yml,yaml}`,
//! so per operator decision #1 (build exactly the parsers the audit
//! surfaces) the implementation is deferred. The region-marker parser
//! already covers other YAML files via `# region:` markers.

pub mod makefile;
pub mod markdown;
pub mod region;

pub use makefile::MakefileSection;
pub use markdown::MarkdownSection;
pub use region::{CommentStyle, RegionSection};
