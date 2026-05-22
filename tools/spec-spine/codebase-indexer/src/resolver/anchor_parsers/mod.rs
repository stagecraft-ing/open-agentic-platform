//! Anchor-parser dispatch. One implementation per file kind; the
//! `section:` resolver looks up the file's extension in
//! `AnchorParserRegistry` and delegates.
//!
//! Adding a new file kind in a future spec: implement `AnchorParser`
//! for the new type and add an `insert` line to
//! `default_anchor_parsers`. No other code changes are required.

use crate::types::LineSpan;
use std::collections::HashMap;

pub mod makefile;
pub mod markdown_heading;
pub mod region_marker;
pub mod workflow_yaml;

pub use makefile::MakefileAnchorParser;
pub use markdown_heading::MarkdownHeadingParser;
pub use region_marker::RegionMarkerParser;
pub use workflow_yaml::WorkflowYamlAnchorParser;

/// Outcome of an anchor lookup.
///
/// - `Ok(Some(span))` — anchor found.
/// - `Ok(None)` — file parsed cleanly but anchor is absent.
/// - `Err(reason)` — the file itself is malformed (e.g. a
///   `// region:` marker with no matching `// endregion`).
pub type AnchorResult = Result<Option<LineSpan>, String>;

pub trait AnchorParser: Send + Sync {
    fn find_anchor(&self, content: &str, anchor: &str) -> AnchorResult;
}

/// Lookup-only — iteration order doesn't matter for determinism, so
/// `HashMap` over `BTreeMap` is fine and lets the registry be plain
/// `&'static str` keys.
pub type AnchorParserRegistry = HashMap<&'static str, Box<dyn AnchorParser>>;

/// Build the default registry. Files whose extension is not in this
/// table fall through to `RegionMarkerParser`, per spec 152 §2.1
/// ("Other source files — same `// region:` convention").
pub fn default_anchor_parsers() -> AnchorParserRegistry {
    let mut m: AnchorParserRegistry = HashMap::new();
    // Makefile lives at an extensionless path.
    m.insert("", Box::new(MakefileAnchorParser));
    m.insert("yml", Box::new(WorkflowYamlAnchorParser));
    m.insert("yaml", Box::new(WorkflowYamlAnchorParser));
    m.insert("rs", Box::new(RegionMarkerParser));
    m.insert("ts", Box::new(RegionMarkerParser));
    m.insert("tsx", Box::new(RegionMarkerParser));
    m.insert("js", Box::new(RegionMarkerParser));
    m.insert("sh", Box::new(RegionMarkerParser));
    m.insert("toml", Box::new(RegionMarkerParser));
    m.insert("md", Box::new(MarkdownHeadingParser));
    m
}

/// Resolve the parser for a path's extension, with `RegionMarkerParser`
/// as the fallback per spec 152 §2.1.
pub(crate) fn dispatch<'a>(
    registry: &'a AnchorParserRegistry,
    path: &str,
) -> &'a dyn AnchorParser {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    // Makefile has no extension — the empty-string key matches.
    let key = if ext.is_empty() && is_makefile(path) {
        ""
    } else {
        ext
    };
    if let Some(parser) = registry.get(key) {
        return parser.as_ref();
    }
    // Default fallback: region markers for any other source-like file.
    registry
        .get("rs")
        .expect("rs parser is registered by default_anchor_parsers")
        .as_ref()
}

fn is_makefile(path: &str) -> bool {
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    name == "Makefile" || name == "makefile" || name == "GNUmakefile"
}
