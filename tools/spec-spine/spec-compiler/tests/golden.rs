//! Two runs produce identical `registry.json` bytes (build-meta.json excluded).

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn spec_compiler_exe() -> PathBuf {
    // Cargo sets `CARGO_BIN_EXE_<name>` at compile time of the integration
    // test, pointing to the freshly-built binary for the current cargo
    // profile (debug or release). Resolving at compile time avoids
    // accidentally running a stale `target/debug/spec-compiler` left over
    // from an earlier build when `cargo test --release` is invoked.
    PathBuf::from(env!("CARGO_BIN_EXE_spec-compiler"))
}

#[test]
fn registry_json_is_deterministic_across_runs() {
    let exe = spec_compiler_exe();
    let root = repo_root();
    let out = root.join("build/spec-registry/registry.json");

    let status = Command::new(&exe)
        .arg("compile")
        .arg("--repo")
        .arg(&root)
        .status()
        .expect("spawn spec-compiler run 1");
    assert!(
        status.success(),
        "run 1 should exit 0 when validation passes"
    );
    assert!(out.is_file(), "registry.json should exist after run 1");
    let first = std::fs::read(&out).expect("read registry after run 1");

    let status = Command::new(&exe)
        .arg("compile")
        .arg("--repo")
        .arg(&root)
        .status()
        .expect("spawn spec-compiler run 2");
    assert!(
        status.success(),
        "run 2 should exit 0 when validation passes"
    );
    let second = std::fs::read(&out).expect("read registry after run 2");

    assert_eq!(
        first, second,
        "registry.json must be byte-identical across runs"
    );
}
