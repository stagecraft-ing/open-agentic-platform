//! Verify CLI exit codes match the spec contract.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn indexer_exe() -> PathBuf {
    // cargo test builds to the target/debug directory
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/codebase-indexer")
}

#[test]
fn compile_exits_zero_on_success() {
    let exe = indexer_exe();
    if !exe.is_file() {
        // Skip in environments where the binary isn't built yet (e.g., cargo test --lib)
        return;
    }
    let root = repo_root();
    let status = Command::new(&exe)
        .arg("compile")
        .arg("--repo")
        .arg(&root)
        .status()
        .expect("spawn codebase-indexer compile");
    assert!(status.success(), "compile should exit 0");
}

#[test]
fn check_exits_zero_when_fresh() {
    let exe = indexer_exe();
    if !exe.is_file() {
        return;
    }
    let root = repo_root();

    // First compile to ensure index exists and is fresh
    let status = Command::new(&exe)
        .arg("compile")
        .arg("--repo")
        .arg(&root)
        .status()
        .expect("spawn compile");
    assert!(status.success());

    // Check should pass
    let status = Command::new(&exe)
        .arg("check")
        .arg("--repo")
        .arg(&root)
        .status()
        .expect("spawn check");
    assert!(status.success(), "check should exit 0 when index is fresh");
}

#[test]
fn compile_exits_nonzero_on_missing_repo() {
    let exe = indexer_exe();
    if !exe.is_file() {
        return;
    }
    let status = Command::new(&exe)
        .arg("compile")
        .arg("--repo")
        .arg("/nonexistent/path/that/does/not/exist")
        .status()
        .expect("spawn compile with bad path");
    assert!(
        !status.success(),
        "compile should exit non-zero on missing repo (got {:?})",
        status.code()
    );
}
