//! Integration tests for the spec 154 Segment 3 resolver.
//!
//! Each per-kind test stands up a small synthetic worktree under a
//! `tempfile::TempDir`, builds a `ResolverContext`, and drives the
//! `resolve()` dispatch with a handful of `LogicalUnit` values. The
//! tests deliberately mirror the design doc §9 plan (per-kind
//! positive + negative + section-anchor coverage + integration +
//! determinism).

use open_agentic_codebase_indexer::resolver::{ResolveError, ResolverContext, resolve};
use open_agentic_codebase_indexer::types::{LineSpan, PackageKind, PackageRecord, ResolvedLocation};
use open_agentic_spec_types::LogicalUnit;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ── Fixture helpers ─────────────────────────────────────────────────────────

struct Fixture {
    _dir: TempDir,
    root: PathBuf,
    packages: Vec<PackageRecord>,
}

impl Fixture {
    fn ctx(&self) -> ResolverContext {
        ResolverContext::build(&self.root, &self.packages)
    }
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Mirror the workspace-root schema directory into the test's
/// synthetic worktree so `compile()`'s self-validation can find it.
/// CARGO_MANIFEST_DIR points at this crate; the schema lives two
/// levels up under `standards/schemas/spec-spine/`.
fn install_schemas(root: &Path) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_src = manifest_dir
        .ancestors()
        .nth(3)
        .expect("repo root above crate")
        .join("standards/schemas/spec-spine/codebase-index.schema.json");
    let schema_dst = root.join("standards/schemas/spec-spine/codebase-index.schema.json");
    fs::create_dir_all(schema_dst.parent().unwrap()).unwrap();
    fs::copy(&schema_src, &schema_dst).unwrap();
}

fn package(name: &str, path: &str, kind: PackageKind) -> PackageRecord {
    PackageRecord {
        name: name.to_string(),
        path: path.to_string(),
        kind,
        version: None,
        edition: None,
        entry_points: None,
        internal_deps: None,
        external_deps: None,
        spec_ref: None,
    }
}

/// Build a synthetic worktree with two workspace-member crates, a
/// Makefile, a workflow YAML, a markdown file, and a region-marker
/// Rust file. Used as the base for all per-kind tests.
fn build_worktree() -> Fixture {
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();

    // Root workspace Cargo.toml (not strictly needed by the resolver,
    // but included for realism).
    write(
        &root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/alpha\", \"crates/beta\"]\n",
    );

    // crates/alpha — has a lib with a top-level fn and an inline mod.
    write(
        &root.join("crates/alpha/Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &root.join("crates/alpha/src/lib.rs"),
        "\
pub fn alpha_top() -> u32 { 1 }

pub mod helpers {
    pub fn helper_one() -> u32 { 2 }
}

pub struct AlphaStruct;
",
    );
    write(
        &root.join("crates/alpha/src/extra.rs"),
        "\
// region: alpha_extra_block
pub fn extra_fn() -> u32 { 9 }
// endregion
",
    );

    // crates/beta — empty src dir except for a stub lib.
    write(
        &root.join("crates/beta/Cargo.toml"),
        "[package]\nname = \"beta\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(&root.join("crates/beta/src/lib.rs"), "pub fn beta() {}\n");

    // A Makefile with two `## tag:` sections.
    write(
        &root.join("Makefile"),
        "\
.PHONY: setup deploy

## tag: setup
setup:
\t@echo setup line 1
\t@echo setup line 2

## tag: deploy
deploy:
\t@echo deploy
",
    );

    // A GitHub workflow YAML.
    write(
        &root.join(".github/workflows/ci.yml"),
        "\
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
",
    );

    // A markdown file with headings.
    write(
        &root.join("docs/notes.md"),
        "\
# Title

intro

## Overview

paragraph

## Configuration

config line 1
config line 2

## Last

last
",
    );

    let packages = vec![
        package("alpha", "crates/alpha", PackageKind::RustLib),
        package("beta", "crates/beta", PackageKind::RustLib),
    ];

    Fixture {
        _dir: dir,
        root,
        packages,
    }
}

// ── Per-kind tests ──────────────────────────────────────────────────────────

#[test]
fn resolve_crate_valid_lists_member_files() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Crate {
        id: "alpha".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert!(locs.iter().any(|l| l.file == "crates/alpha/src/lib.rs"));
    assert!(locs.iter().any(|l| l.file == "crates/alpha/src/extra.rs"));
    // Sort contract: filenames in lexicographic order.
    for w in locs.windows(2) {
        assert!(w[0].file <= w[1].file);
    }
    // Every location is whole-file.
    assert!(locs.iter().all(|l| l.span.is_none()));
}

#[test]
fn resolve_crate_missing_is_hard_error() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Crate {
        id: "gamma".to_string(),
    };
    match resolve(&unit, &ctx) {
        Err(ResolveError::UnknownCrate { id }) => assert_eq!(id, "gamma"),
        other => panic!("expected UnknownCrate, got {other:?}"),
    }
}

#[test]
fn resolve_symbol_valid_returns_qualified_location() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Symbol {
        id: "alpha::alpha_top".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].file, "crates/alpha/src/lib.rs");
    let span = locs[0].span.as_ref().unwrap();
    assert!(span.start_line >= 1);
    assert!(span.end_line >= span.start_line);
}

#[test]
fn resolve_symbol_inline_module_path_resolves() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Symbol {
        id: "alpha::helpers::helper_one".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].file, "crates/alpha/src/lib.rs");
}

#[test]
fn resolve_symbol_missing_is_hard_error() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Symbol {
        id: "alpha::ghost_fn".to_string(),
    };
    match resolve(&unit, &ctx) {
        Err(ResolveError::UnknownSymbol { id }) => assert_eq!(id, "alpha::ghost_fn"),
        other => panic!("expected UnknownSymbol, got {other:?}"),
    }
}

#[test]
fn resolve_module_file_returns_whole_file() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Module {
        id: "alpha::extra".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].file, "crates/alpha/src/extra.rs");
    assert!(locs[0].span.is_none());
}

#[test]
fn resolve_module_inline_returns_span() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Module {
        id: "alpha::helpers".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].file, "crates/alpha/src/lib.rs");
    let span = locs[0].span.as_ref().expect("inline mod has a span");
    // Inline-module span includes the `mod helpers {` declaration line (OQ-7).
    assert!(span.start_line >= 1);
    assert!(span.end_line >= span.start_line);
}

#[test]
fn resolve_module_missing_is_hard_error() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Module {
        id: "alpha::ghost".to_string(),
    };
    match resolve(&unit, &ctx) {
        Err(ResolveError::MissingModule { id }) => assert_eq!(id, "alpha::ghost"),
        other => panic!("expected MissingModule, got {other:?}"),
    }
}

#[test]
fn resolve_section_makefile() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Section {
        file: "Makefile".to_string(),
        anchor: "deploy".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].file, "Makefile");
    let span = locs[0].span.as_ref().unwrap();
    assert!(span.start_line >= 1);
    assert!(span.end_line >= span.start_line);
}

#[test]
fn resolve_section_workflow_yaml() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Section {
        file: ".github/workflows/ci.yml".to_string(),
        anchor: "jobs.build".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert_eq!(locs.len(), 1);
    let span = locs[0].span.as_ref().unwrap();
    assert!(span.start_line >= 1);
    assert!(span.end_line >= span.start_line);
}

#[test]
fn resolve_section_region_marker() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Section {
        file: "crates/alpha/src/extra.rs".to_string(),
        anchor: "alpha_extra_block".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert_eq!(locs.len(), 1);
    let span = locs[0].span.as_ref().unwrap();
    assert_eq!(span.start_line, 1);
    assert_eq!(span.end_line, 3);
}

#[test]
fn resolve_section_markdown_heading() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Section {
        file: "docs/notes.md".to_string(),
        anchor: "configuration".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert_eq!(locs.len(), 1);
    let span = locs[0].span.as_ref().unwrap();
    // `## Configuration` heading at line 9, content lines 11/12,
    // trailing blank trimmed before next `## Last`.
    assert_eq!(span.start_line, 9);
    assert_eq!(span.end_line, 12);
}

#[test]
fn resolve_section_missing_anchor_is_hard_error() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Section {
        file: "Makefile".to_string(),
        anchor: "absent".to_string(),
    };
    match resolve(&unit, &ctx) {
        Err(ResolveError::AnchorNotFound { file, anchor }) => {
            assert_eq!(file, "Makefile");
            assert_eq!(anchor, "absent");
        }
        other => panic!("expected AnchorNotFound, got {other:?}"),
    }
}

#[test]
fn resolve_section_missing_file_is_hard_error() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Section {
        file: "nope.mk".to_string(),
        anchor: "any".to_string(),
    };
    match resolve(&unit, &ctx) {
        Err(ResolveError::SectionFileMissing { file }) => assert_eq!(file, "nope.mk"),
        other => panic!("expected SectionFileMissing, got {other:?}"),
    }
}

#[test]
fn resolve_directory_excludes_target_tree() {
    let f = build_worktree();
    // Drop a target/ tree under crates/alpha to exercise the §3.7 exclusion.
    write(
        &f.root.join("crates/alpha/target/debug/build.json"),
        "{}",
    );
    let ctx = f.ctx();
    let unit = LogicalUnit::Directory {
        path: "crates/alpha".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert!(locs.iter().all(|l| !l.file.contains("/target/")));
    assert!(locs.iter().any(|l| l.file == "crates/alpha/src/lib.rs"));
}

#[test]
fn resolve_directory_missing_is_hard_error() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Directory {
        path: "no/such/dir".to_string(),
    };
    match resolve(&unit, &ctx) {
        Err(ResolveError::MissingDirectory { path }) => assert_eq!(path, "no/such/dir"),
        other => panic!("expected MissingDirectory, got {other:?}"),
    }
}

#[test]
fn resolve_file_valid() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::File {
        path: "Makefile".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    assert_eq!(locs, vec![ResolvedLocation { file: "Makefile".to_string(), span: None }]);
}

#[test]
fn resolve_file_missing_is_hard_error() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::File {
        path: "ghost.rs".to_string(),
    };
    match resolve(&unit, &ctx) {
        Err(ResolveError::MissingFile { path }) => assert_eq!(path, "ghost.rs"),
        other => panic!("expected MissingFile, got {other:?}"),
    }
}

// ── Sort / determinism contract ─────────────────────────────────────────────

#[test]
fn resolver_locations_are_sorted_and_deduped() {
    let f = build_worktree();
    let ctx = f.ctx();
    let unit = LogicalUnit::Crate {
        id: "alpha".to_string(),
    };
    let locs = resolve(&unit, &ctx).unwrap();
    let mut sorted = locs.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(locs, sorted);
}

#[test]
fn compile_is_byte_deterministic_against_worktree() {
    // Stand up a synthetic worktree containing a single spec with
    // path-list `establishes:` claims and a `references:` field
    // mixing bare-string and unit forms. Compile twice; assert
    // byte-equal output.
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    install_schemas(&root);

    // Workspace root.
    write(
        &root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"crates/alpha\"]\n",
    );
    write(
        &root.join("crates/alpha/Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(&root.join("crates/alpha/src/lib.rs"), "pub fn alpha() {}\n");

    write(
        &root.join("Makefile"),
        "## tag: setup\nsetup:\n\t@echo setup\n",
    );

    // A spec that exercises every relationship-graph field shape:
    // - `establishes:` flat list (paths)
    // - `extends:` structured (paths)
    // - `references:` bare + role-tagged unit form
    write(
        &root.join("specs/200-fixture-spec/spec.md"),
        r#"---
id: "200-fixture-spec"
title: "Fixture"
status: approved
kind: tooling
establishes:
  - crates/alpha/src/lib.rs
  - Makefile
extends:
  - spec: "001"
    paths:
      - crates/alpha
references:
  - "Makefile"
  - role: example
    unit: { kind: section, file: Makefile, anchor: setup }
---
body
"#,
    );

    let out1 = open_agentic_codebase_indexer::compile(&root).expect("compile run 1");
    let out2 = open_agentic_codebase_indexer::compile(&root).expect("compile run 2");
    assert_eq!(out1.index_json, out2.index_json);

    // Round-trip check: parsed traceability surfaces resolved units.
    let parsed: open_agentic_codebase_indexer::types::CodebaseIndex =
        serde_json::from_slice(&out1.index_json).unwrap();
    let mapping = parsed
        .traceability
        .mappings
        .iter()
        .find(|m| m.spec_id == "200-fixture-spec")
        .expect("fixture spec is mapped");
    assert!(!mapping.resolved_units.is_empty());
    let kinds: std::collections::BTreeSet<String> =
        mapping.resolved_units.iter().map(|u| u.kind.clone()).collect();
    assert!(kinds.contains("file"));
    assert!(kinds.contains("section"));
    // The role-tagged section unit lives under `references` (non-ownership).
    let section_units: Vec<_> = mapping
        .resolved_units
        .iter()
        .filter(|u| u.kind == "section")
        .collect();
    assert!(section_units.iter().any(|u| !u.ownership));
    assert!(section_units.iter().all(|u| !u.locations.is_empty()));
}

// ── Bare-vs-explicit MissingFile severity split (S3-DROP-1 capture) ─────────

#[test]
fn missing_file_severity_splits_on_authoring_shape() {
    // Two specs against the same fixture: one declares the missing
    // path as a bare string, the other as an explicit `{kind: file}`
    // unit. The compat-window severity diverges per the V-023 mirror:
    // bare → I-108 (non-blocking); explicit → I-008 (blocking).
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    install_schemas(&root);
    write(&root.join("Cargo.toml"), "[workspace]\nmembers = []\n");

    write(
        &root.join("specs/300-bare-missing/spec.md"),
        r#"---
id: "300-bare-missing"
title: "Bare-string missing path"
status: approved
kind: tooling
establishes:
  - does/not/exist/here.rs
---
body
"#,
    );
    write(
        &root.join("specs/301-explicit-missing/spec.md"),
        r#"---
id: "301-explicit-missing"
title: "Explicit missing path"
status: approved
kind: tooling
extends:
  - spec: "300"
    units:
      - { kind: file, path: also/does/not/exist.rs }
---
body
"#,
    );

    let out = open_agentic_codebase_indexer::compile(&root).expect("compile fixture");
    let parsed: open_agentic_codebase_indexer::types::CodebaseIndex =
        serde_json::from_slice(&out.index_json).unwrap();

    let warnings: Vec<&str> = parsed
        .diagnostics
        .warnings
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    let errors: Vec<&str> = parsed
        .diagnostics
        .errors
        .iter()
        .map(|d| d.code.as_str())
        .collect();

    assert!(
        warnings.iter().any(|c| *c == "I-108"),
        "bare-string MissingFile must emit I-108 (warning)"
    );
    assert!(
        errors.iter().any(|c| *c == "I-008"),
        "explicit MissingFile must emit I-008 (error)"
    );
}

// ── Suppress unused-variable lint on the fixture builder ────────────────────
#[allow(dead_code)]
fn _suppress_dead_code(_l: LineSpan) {}
