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

/// Mirror the real repo into a tempdir via symlinks so each test owns its
/// own `build/` output without contending on the real repo's index.json.
/// `build/` and `target/` are skipped — those are per-test scratch space
/// (`build/`) or irrelevant build output (`target/`).
fn mirror_repo() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().expect("create tempdir");
    let real = repo_root();
    let entries = std::fs::read_dir(&real).expect("read real repo root");
    for ent in entries.flatten() {
        let name = ent.file_name();
        let name_str = name.to_string_lossy();
        if name_str == "build" || name_str == "target" {
            continue;
        }
        let src = ent.path();
        let dst = tmp.path().join(&name);
        #[cfg(unix)]
        std::os::unix::fs::symlink(&src, &dst).expect("symlink");
        #[cfg(windows)]
        {
            if src.is_dir() {
                std::os::windows::fs::symlink_dir(&src, &dst).expect("symlink_dir");
            } else {
                std::os::windows::fs::symlink_file(&src, &dst).expect("symlink_file");
            }
        }
    }
    tmp
}

#[test]
fn compile_exits_zero_on_success() {
    let exe = indexer_exe();
    if !exe.is_file() {
        // Skip in environments where the binary isn't built yet (e.g., cargo test --lib)
        return;
    }
    let scratch = mirror_repo();
    let status = Command::new(&exe)
        .arg("compile")
        .arg("--repo")
        .arg(scratch.path())
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
    let scratch = mirror_repo();

    // First compile to ensure index exists and is fresh
    let status = Command::new(&exe)
        .arg("compile")
        .arg("--repo")
        .arg(scratch.path())
        .status()
        .expect("spawn compile");
    assert!(status.success());

    // Check should pass
    let status = Command::new(&exe)
        .arg("check")
        .arg("--repo")
        .arg(scratch.path())
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
