//! Spec 154 Segment 3 §7 — resolver performance characterization.
//!
//! Runs through `cargo bench`, not `make ci` (multi-sample runs are
//! too heavy for the ~5 min fast loop per spec 135 §1). The bench
//! cases here are the regression-detection surface; the operational
//! budget (<10s warm for `compile()` against the full corpus on M1
//! Pro) is verified at `make ci-strict` time as a follow-up.

use criterion::{Criterion, criterion_group, criterion_main};
use open_agentic_codebase_indexer::resolver::{ResolverContext, resolve};
use open_agentic_codebase_indexer::types::{PackageKind, PackageRecord};
use open_agentic_spec_types::LogicalUnit;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

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

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn make_fixture() -> (TempDir, PathBuf, Vec<PackageRecord>) {
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    write(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/alpha\"]\n",
    );
    write(
        &root.join("crates/alpha/Cargo.toml"),
        "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    // Spread items across a few files so the symbol index pass has
    // realistic shape.
    write(
        &root.join("crates/alpha/src/lib.rs"),
        "pub mod a;\npub mod b;\n\npub fn lib_fn() {}\n",
    );
    write(
        &root.join("crates/alpha/src/a.rs"),
        "pub fn a_fn() {}\npub struct AOne;\npub struct ATwo;\n",
    );
    write(
        &root.join("crates/alpha/src/b.rs"),
        "pub fn b_fn() {}\npub enum BEnum { One, Two }\n",
    );
    write(&root.join("Makefile"), "## tag: setup\nsetup:\n\t@echo ok\n");
    let packages = vec![package("alpha", "crates/alpha", PackageKind::RustLib)];
    (dir, root, packages)
}

fn bench_symbol_index_build(c: &mut Criterion) {
    let (_d, root, packages) = make_fixture();
    c.bench_function("symbol_index_build", |b| {
        b.iter(|| {
            let _ = ResolverContext::build(&root, &packages);
        });
    });
}

fn bench_resolve_crate(c: &mut Criterion) {
    let (_d, root, packages) = make_fixture();
    let ctx = ResolverContext::build(&root, &packages);
    let unit = LogicalUnit::Crate {
        id: "alpha".to_string(),
    };
    c.bench_function("resolve_crate", |b| {
        b.iter(|| {
            let _ = resolve(&unit, &ctx);
        });
    });
}

fn bench_resolve_symbol(c: &mut Criterion) {
    let (_d, root, packages) = make_fixture();
    let ctx = ResolverContext::build(&root, &packages);
    let unit = LogicalUnit::Symbol {
        id: "alpha::a::a_fn".to_string(),
    };
    c.bench_function("resolve_symbol", |b| {
        b.iter(|| {
            let _ = resolve(&unit, &ctx);
        });
    });
}

fn bench_resolve_section(c: &mut Criterion) {
    let (_d, root, packages) = make_fixture();
    let ctx = ResolverContext::build(&root, &packages);
    let unit = LogicalUnit::Section {
        file: "Makefile".to_string(),
        anchor: "setup".to_string(),
    };
    c.bench_function("resolve_section_makefile", |b| {
        b.iter(|| {
            let _ = resolve(&unit, &ctx);
        });
    });
}

criterion_group!(
    benches,
    bench_symbol_index_build,
    bench_resolve_crate,
    bench_resolve_symbol,
    bench_resolve_section
);
criterion_main!(benches);
