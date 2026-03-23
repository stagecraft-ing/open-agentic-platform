//! CLI exit codes: 0 = ok + validation passed, 1 = validation failed, 3 = compile error.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn spec_compiler_exe() -> PathBuf {
    if let Some(e) = std::env::var_os("CARGO_BIN_EXE_spec_compiler") {
        return PathBuf::from(e);
    }
    #[cfg(windows)]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/spec-compiler.exe")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/spec-compiler")
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn exit_code_zero_on_success_repo() {
    let exe = spec_compiler_exe();
    let status = Command::new(&exe)
        .args(["compile", "--repo"])
        .arg(repo_root())
        .status()
        .expect("spawn");
    assert!(status.success(), "expected exit 0, got {status:?}");
}

#[test]
fn exit_code_one_on_validation_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let spec_dir = root.join("specs/001-a");
    let spec_dir2 = root.join("specs/002-b");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::create_dir_all(&spec_dir2).unwrap();

    let same_id = r#"---
id: "001-a"
title: "Dup"
status: draft
created: "2026-03-22"
summary: "x"
---
"#;
    fs::write(spec_dir.join("spec.md"), same_id).unwrap();
    fs::write(spec_dir2.join("spec.md"), same_id).unwrap();

    let exe = spec_compiler_exe();
    let status = Command::new(&exe)
        .args(["compile", "--repo"])
        .arg(root)
        .status()
        .expect("spawn");
    assert_eq!(
        status.code(),
        Some(1),
        "duplicate id should yield validation failure (exit 1), got {status:?}"
    );
}

#[test]
fn exit_code_three_on_invalid_utf8_spec() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let spec_dir = root.join("specs/001-a");
    fs::create_dir_all(&spec_dir).unwrap();
    // Invalid UTF-8: read_to_string fails during compile → CompileError → exit 3.
    fs::write(spec_dir.join("spec.md"), [0xffu8, 0xfe, 0xff]).unwrap();

    let exe = spec_compiler_exe();
    let status = Command::new(&exe)
        .args(["compile", "--repo"])
        .arg(root)
        .status()
        .expect("spawn");
    assert_eq!(
        status.code(),
        Some(3),
        "unreadable spec.md should yield exit 3, got {status:?}"
    );
}
