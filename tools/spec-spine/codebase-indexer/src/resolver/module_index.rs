//! Spec 154 Segment 3 — Rust module index.
//!
//! Two kinds of module entries are indexed:
//!
//! - **File modules.** Every `*.rs` source file under a workspace
//!   member maps to a module key (`<crate_import>::<segments>`). The
//!   `file:` value is the source-file path; `span:` is `None`
//!   (whole-file ownership).
//! - **Inline modules.** Every `mod foo { ... }` body inside a parent
//!   file maps to `<parent-prefix>::foo`. The span covers the `mod
//!   foo {` declaration line through the closing `}` (inclusive),
//!   per OQ-7 closure in segment-3-design.md §4.3.
//!
//! Lookups happen by exact qualified path. A `module:` resolution
//! returns at most one entry — a path either names a file module
//! (single `ResolvedLocation`) or an inline module (single
//! `ResolvedLocation` with a span).

use crate::types::{LineSpan, ResolvedLocation};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tree_sitter::{Node, Parser};
use walkdir::WalkDir;

#[derive(Default)]
pub struct ModuleIndex {
    pub by_path: BTreeMap<String, ResolvedLocation>,
}

pub fn build(repo_root: &Path, workspace_members: &BTreeMap<String, String>) -> ModuleIndex {
    let mut idx = ModuleIndex::default();
    let mut parser = Parser::new();
    let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    let parser_ok = parser.set_language(&lang).is_ok();

    for (crate_name, crate_path) in workspace_members {
        let crate_import = crate_name.replace('-', "_");
        let abs_root = repo_root.join(crate_path);
        let src_root = abs_root.join("src");
        let walk_root = if src_root.is_dir() { src_root } else { abs_root };
        let walker = WalkDir::new(&walk_root).sort_by_file_name();
        for ent in walker {
            let Ok(ent) = ent else {
                continue;
            };
            if !ent.file_type().is_file() {
                continue;
            }
            let path = ent.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let Ok(rel) = path.strip_prefix(repo_root) else {
                continue;
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let module_path = file_module_path(&crate_import, &walk_root, path);

            // Record the file-module entry. Span: None (whole-file
            // ownership) per design §4.3.
            idx.by_path.entry(module_path.clone()).or_insert(ResolvedLocation {
                file: rel_str.clone(),
                span: None,
            });

            if !parser_ok {
                continue;
            }
            let Ok(src) = fs::read_to_string(path) else {
                continue;
            };
            let Some(tree) = parser.parse(&src, None) else {
                continue;
            };
            extract_inline_modules(tree.root_node(), src.as_bytes(), &rel_str, &module_path, &mut idx);
        }
    }
    idx
}

/// File-module path, matching the prefix used by the symbol index.
fn file_module_path(crate_import: &str, src_root: &Path, file: &Path) -> String {
    let Ok(rel) = file.strip_prefix(src_root) else {
        return crate_import.to_string();
    };
    let mut segments: Vec<String> = Vec::new();
    for component in rel.parent().into_iter().flatten() {
        if let Some(s) = component.to_str() {
            segments.push(s.replace('-', "_"));
        }
    }
    let stem = rel
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let is_root = matches!(stem, "lib" | "main" | "mod");
    let mut parts: Vec<String> = vec![crate_import.to_string()];
    parts.extend(segments);
    if !is_root && !stem.is_empty() {
        parts.push(stem.replace('-', "_"));
    }
    parts.join("::")
}

/// Walk the AST descending into every `mod foo { ... }` body. Records
/// the span of each inline module from its declaration line through
/// the closing `}` (inclusive), per OQ-7.
fn extract_inline_modules(
    node: Node<'_>,
    src: &[u8],
    file: &str,
    prefix: &str,
    idx: &mut ModuleIndex,
) {
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        let child = cursor.node();
        if child.kind() == "mod_item" {
            if let Some(name_node) = child.child_by_field_name("name") {
                if let Ok(name) = name_node.utf8_text(src) {
                    if child.child_by_field_name("body").is_some() {
                        let qpath = format!("{prefix}::{name}");
                        let span = LineSpan {
                            start_line: child.start_position().row as u32 + 1,
                            end_line: child.end_position().row as u32 + 1,
                        };
                        idx.by_path.entry(qpath.clone()).or_insert(ResolvedLocation {
                            file: file.to_string(),
                            span: Some(span),
                        });
                        // Recurse: nested `mod a { mod b { ... } }`.
                        if let Some(body) = child.child_by_field_name("body") {
                            let nested_prefix = qpath;
                            extract_inline_modules(body, src, file, &nested_prefix, idx);
                        }
                    }
                }
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}
