// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: XRAY_ANALYSIS
// Spec: spec/xray/analysis.md

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

#[test]
fn test_index_format() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let output_dir = PathBuf::from(&manifest_dir).join("tests/outputs/index_format");
    let fixture_src = PathBuf::from(&manifest_dir).join("tests/fixtures/min_repo");

    // Create temp dir for fixture to ensure it's clean and has .git
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let fixture_dst = temp_dir.path().join("min_repo");

    let copy_options = fs_extra::dir::CopyOptions::new()
        .overwrite(true)
        .copy_inside(true);
    fs_extra::dir::copy(&fixture_src, temp_dir.path(), &copy_options)
        .expect("Failed to copy fixture");

    // Add .git to ensure it is treated as a repo
    let git_dir = fixture_dst.join(".git");
    fs::create_dir_all(&git_dir).expect("Failed to create .git dir");

    // Clean output
    let _ = fs::remove_dir_all(&output_dir);

    // Use CARGO_BIN_EXE_axiomregent provided by Cargo for integration tests
    // Utilize CARGO_BIN_EXE_axiomregent if set (integration tests), or fallback to searching target dir (unit tests/workspace)
    let axiomregent_bin = env::var("CARGO_BIN_EXE_axiomregent").unwrap_or_else(|_| {
        let mut path = PathBuf::from(&manifest_dir);
        path.pop(); // crates
        path.pop(); // axiomregent root
        path.push("target");
        if cfg!(debug_assertions) {
            path.push("debug");
        } else {
            path.push("release");
        }
        path.push("axiomregent");

        // Final fallback try: check the other profile if not found (e.g. running test in debug but binary built in release)
        if !path.exists() {
            let mut alt_path = PathBuf::from(&manifest_dir);
            alt_path.pop();
            alt_path.pop();
            alt_path.push("target");
            if cfg!(debug_assertions) {
                alt_path.push("release");
            } else {
                alt_path.push("debug");
            }
            alt_path.push("axiomregent");
            if alt_path.exists() {
                return alt_path.to_string_lossy().to_string();
            }
        }

        path.to_string_lossy().to_string()
    });

    // Run Scan
    let status = Command::new(axiomregent_bin)
        .arg("scan")
        .arg(&fixture_dst)
        .arg("--output")
        .arg(&output_dir)
        .current_dir(&manifest_dir)
        .status()
        .expect("Failed to run scan");
    assert!(status.success());

    let index_path = output_dir.join("index.json");
    let content = fs::read_to_string(&index_path).expect("Failed to read index.json");

    // Assert JSON Structure Matches Contract (index.json schemaVersion 1.0.0)
    let v: Value = serde_json::from_str(&content).expect("index.json must be valid JSON");

    // 1. Root fields
    assert_eq!(
        v.get("schemaVersion").and_then(Value::as_str),
        Some("1.0.0"),
        "schemaVersion must be 1.0.0"
    );

    assert!(
        v.get("digest").and_then(Value::as_str).is_some(),
        "Missing digest"
    );

    assert!(
        v.get("root").and_then(Value::as_str).is_some(),
        "Missing root"
    );

    assert!(
        v.get("target").and_then(Value::as_str).is_some(),
        "Missing target"
    );

    // These were in the old test but are not in the provided index.json structure
    assert!(
        v.get("scanId").is_none(),
        "scanId must not be present in index.json"
    );
    assert!(
        v.get("rootHash").is_none(),
        "rootHash must not be present in index.json"
    );

    // 2. Summary fields
    let languages = v
        .get("languages")
        .and_then(Value::as_object)
        .expect("Missing languages object");
    assert!(!languages.is_empty(), "languages object must not be empty");

    // 3. Module files
    let module_files = v
        .get("moduleFiles")
        .and_then(Value::as_array)
        .expect("Missing moduleFiles array");
    assert!(!module_files.is_empty(), "moduleFiles must not be empty");

    // 4. stats
    let stats = v
        .get("stats")
        .and_then(Value::as_object)
        .expect("Missing stats object");
    assert!(
        stats.get("fileCount").and_then(Value::as_u64).is_some(),
        "stats.fileCount missing or not a number"
    );
    assert!(
        stats.get("totalSize").and_then(Value::as_u64).is_some(),
        "stats.totalSize missing or not a number"
    );

    // 5. topDirs
    let top_dirs = v
        .get("topDirs")
        .and_then(Value::as_object)
        .expect("Missing topDirs object");
    assert!(!top_dirs.is_empty(), "topDirs must not be empty");

    // 6. File listing
    let files = v
        .get("files")
        .and_then(Value::as_array)
        .expect("Missing files array");
    assert!(!files.is_empty(), "files array must not be empty");

    // Ensure a specific expected entry exists (matches your sample)
    assert!(
        files
            .iter()
            .any(|f| f.get("path").and_then(Value::as_str) == Some("main.go")),
        "Missing expected file entry: main.go"
    );

    // Validate per-file object contract for every entry (matches sample shape)
    for (i, f) in files.iter().enumerate() {
        let obj = f
            .as_object()
            .unwrap_or_else(|| panic!("files[{i}] must be an object"));

        assert!(
            obj.get("path").and_then(Value::as_str).is_some(),
            "files[{i}] missing path"
        );
        assert!(
            obj.get("lang").and_then(Value::as_str).is_some(),
            "files[{i}] missing lang"
        );
        assert!(
            obj.get("hash").and_then(Value::as_str).is_some(),
            "files[{i}] missing hash"
        );

        assert!(
            obj.get("loc").and_then(Value::as_u64).is_some(),
            "files[{i}] missing loc"
        );
        assert!(
            obj.get("size").and_then(Value::as_u64).is_some(),
            "files[{i}] missing size"
        );
        assert!(
            obj.get("complexity").and_then(Value::as_u64).is_some(),
            "files[{i}] missing complexity"
        );
    }
}
