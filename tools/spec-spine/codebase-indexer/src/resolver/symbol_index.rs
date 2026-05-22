//! Spec 154 Segment 3 — Rust symbol index.
//!
//! A single tree-sitter pass over every `*.rs` file under each
//! workspace-member root produces `BTreeMap<qualified-path,
//! Vec<ResolvedLocation>>` keyed by Rust item path
//! (e.g. `canonical_json::canonicalize_value`). The resolver's
//! `symbol:` kind looks up bare paths in this map.
//!
//! Scope today: top-level items (`fn`, `struct`, `enum`, `trait`,
//! `type`, `const`, `static`, `union`) under nested `mod foo { ... }`
//! blocks and file-modules. Methods inside `impl` blocks, `pub use`
//! re-exports, macro-synthesized items, and conditional-compilation
//! ambiguities are **deliberately not handled** in the initial
//! implementation. Each is a halt-and-surface point per the Segment
//! 3 calibration (segment-3-handoff.md stop condition #1) — the
//! resolver hard-errors on a missing symbol, which is the trigger to
//! land the decision before the corpus exercises that construct.
//!
//! Deterministic by construction: BTreeMap keying, file traversal
//! sorted, per-symbol location lists sorted at the resolver boundary.

use crate::types::{LineSpan, ResolvedLocation};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tree_sitter::{Node, Parser};
use walkdir::WalkDir;

#[derive(Default)]
pub struct SymbolIndex {
    pub by_path: BTreeMap<String, Vec<ResolvedLocation>>,
}

pub fn build(repo_root: &Path, workspace_members: &BTreeMap<String, String>) -> SymbolIndex {
    let mut idx = SymbolIndex::default();
    let mut parser = Parser::new();
    let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
    if parser.set_language(&lang).is_err() {
        return idx;
    }
    for (crate_name, crate_path) in workspace_members {
        // Rust's identifier-name rule: hyphens in the manifest name
        // become underscores in the import path.
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
            let Ok(src) = fs::read_to_string(path) else {
                continue;
            };
            let module_prefix = file_module_prefix(&crate_import, &walk_root, path);
            let Some(tree) = parser.parse(&src, None) else {
                continue;
            };
            extract_items_recursive(
                tree.root_node(),
                src.as_bytes(),
                &rel_str,
                &module_prefix,
                &mut idx,
            );
        }
    }
    idx
}

/// Compute the module path prefix for a file relative to its
/// crate-source root. Examples:
///
/// - `crates/canonical-json/src/lib.rs`    → `canonical_json`
/// - `crates/canonical-json/src/main.rs`   → `canonical_json`
/// - `crates/canonical-json/src/x.rs`      → `canonical_json::x`
/// - `crates/canonical-json/src/a/mod.rs`  → `canonical_json::a`
/// - `crates/canonical-json/src/a/b.rs`    → `canonical_json::a::b`
fn file_module_prefix(crate_import: &str, src_root: &Path, file: &Path) -> String {
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

/// Walk the source-file AST, descending into `mod foo { ... }` blocks
/// to extend the current module-path stack, and recording top-level
/// item names. `impl` and `trait` bodies are not descended into —
/// methods inside them are out of scope for the initial pass.
fn extract_items_recursive(
    node: Node<'_>,
    src: &[u8],
    file: &str,
    module_prefix: &str,
    idx: &mut SymbolIndex,
) {
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        let child = cursor.node();
        match child.kind() {
            "function_item"
            | "struct_item"
            | "enum_item"
            | "union_item"
            | "trait_item"
            | "const_item"
            | "static_item"
            | "type_item" => {
                if let Some(name) = item_name(&child, src) {
                    let span = node_to_span(&child);
                    let qpath = format!("{module_prefix}::{name}");
                    idx.by_path
                        .entry(qpath)
                        .or_default()
                        .push(ResolvedLocation {
                            file: file.to_string(),
                            span: Some(span),
                        });
                }
            }
            "mod_item" => {
                // Nested `mod foo { ... }` — recurse with extended
                // prefix only if the module has an inline body.
                if let Some(name_node) = child.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(src) {
                        let nested_prefix = format!("{module_prefix}::{name}");
                        if let Some(body) = child.child_by_field_name("body") {
                            extract_items_recursive(body, src, file, &nested_prefix, idx);
                        }
                    }
                }
            }
            // impl blocks: stop-and-surface point. Methods inside an
            // `impl<T> Foo<T> { ... }` block need a qualified-path
            // convention (`Foo::method`? `<impl-NNN>::method`?). Out
            // of scope until a `symbol:` unit forces the decision.
            "impl_item" => {}
            // pub use re-exports: canonical-decl-path vs
            // re-export-path is a separate halt-and-surface decision.
            "use_declaration" => {}
            // macros and macro-rules: synthesised items don't appear
            // in the AST, by definition.
            "macro_definition" | "macro_invocation" => {}
            _ => {}
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

/// Best-effort item-name extraction. Top-level items expose `name`
/// as a `field_name` on the node; missing names (e.g. anonymous
/// constants `const _: () = ...`) are skipped.
fn item_name<'a>(node: &Node<'_>, src: &'a [u8]) -> Option<&'a str> {
    let name_node = node.child_by_field_name("name")?;
    name_node.utf8_text(src).ok()
}

pub fn node_to_span(node: &Node<'_>) -> LineSpan {
    let start = node.start_position().row as u32 + 1;
    let end = node.end_position().row as u32 + 1;
    LineSpan {
        start_line: start,
        end_line: end,
    }
}
