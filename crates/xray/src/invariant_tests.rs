// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: XRAY_ANALYSIS
// Spec: spec/xray/analysis.md

use crate::canonical::to_canonical_json;
use crate::digest::calculate_digest;
use crate::schema::{FileNode, RepoStats, XrayIndex};
use std::collections::BTreeMap;

fn make_valid_index() -> XrayIndex {
    XrayIndex {
        schema_version: "1.0.0".to_string(),
        root: "test".to_string(),
        target: ".".to_string(),
        files: vec![
            FileNode {
                path: "a.txt".to_string(),
                size: 10,
                hash: "opt".to_string(),
                lang: "Text".to_string(),
                loc: 1,
                complexity: 1,
            },
            FileNode {
                path: "b.txt".to_string(),
                size: 20,
                hash: "opt".to_string(),
                lang: "Text".to_string(),
                loc: 2,
                complexity: 1,
            },
        ],
        languages: BTreeMap::from([("Text".to_string(), 2)]),
        top_dirs: BTreeMap::from([(".".to_string(), 2)]),
        module_files: vec!["a.mod".to_string(), "b.mod".to_string()],
        stats: RepoStats {
            file_count: 2,
            total_size: 30,
        },
        digest: "".to_string(),
    }
}

#[test]
fn test_digest_enforces_sorted_files() {
    let mut index = make_valid_index();
    // Swap files to be unsorted
    index.files.swap(0, 1);

    // Expect error
    match calculate_digest(&index) {
        Ok(_) => panic!("Digest MUST fail on unsorted files"),
        Err(e) => assert!(
            e.to_string().contains("Files not sorted"),
            "Wrong error: {}",
            e
        ),
    }
}

#[test]
fn test_digest_enforces_sorted_modules() {
    let mut index = make_valid_index();
    index.module_files.swap(0, 1);

    match calculate_digest(&index) {
        Ok(_) => panic!("Digest MUST fail on unsorted module_files"),
        Err(e) => assert!(
            e.to_string().contains("Module files not sorted"),
            "Wrong error: {}",
            e
        ),
    }
}

#[test]
fn test_digest_enforces_unique_paths() {
    let mut index = make_valid_index();
    // Duplicate first file
    let duplicate = index.files[0].clone();
    index.files.insert(1, duplicate);
    // Sort them so sorting isn't the error
    // "a.txt", "a.txt", "b.txt" -> Sorted

    // Update stats to match count so that's not the error
    index.stats.file_count = 3;
    index.stats.total_size += 10;

    match calculate_digest(&index) {
        Ok(_) => panic!("Digest MUST fail on duplicate file paths"),
        Err(e) => assert!(
            e.to_string().contains("Duplicate file path"),
            "Wrong error: {}",
            e
        ),
    }
}

#[test]
fn test_digest_enforces_stats_integrity() {
    let mut index = make_valid_index();
    index.stats.file_count = 999; // Wrong count

    match calculate_digest(&index) {
        Ok(_) => panic!("Digest MUST fail on stats mismatch"),
        Err(e) => assert!(
            e.to_string().contains("File count mismatch"),
            "Wrong error: {}",
            e
        ),
    }
}

#[test]
fn test_canonical_json_refuses_invalid_index() {
    let mut index = make_valid_index();
    index.files.swap(0, 1); // Unsorted

    match to_canonical_json(&index) {
        Ok(_) => panic!("Canonical JSON MUST fail on unsorted index"),
        Err(e) => assert!(
            e.to_string().contains("Files not sorted"),
            "Wrong error: {}",
            e
        ),
    }
}

#[test]
fn test_canonical_json_stability() {
    let index = make_valid_index();
    let json1 = to_canonical_json(&index).unwrap();
    let json2 = to_canonical_json(&index).unwrap();
    assert_eq!(json1, json2, "Deterministic across multiple calls");

    // Check no whitespace
    let s = String::from_utf8(json1).unwrap();
    assert!(!s.contains(" "), "Must be compact (no spaces)");
    assert!(!s.contains("\n"), "Must be compact (no newlines)");
}

#[test]
fn test_loc_determinism() {
    // Current behavior: lines().count()
    // "a\n" -> 1 line
    // "a" -> 1 line
    // "" -> 0 lines
    use crate::loc::compute_loc;
    use std::io::Write;

    let mut t = tempfile::NamedTempFile::new().unwrap();
    write!(t, "Line 1\nLine 2").unwrap(); // No trailing newline
    let stats = compute_loc(t.path()).unwrap();
    assert_eq!(stats.loc, 2);

    let mut t2 = tempfile::NamedTempFile::new().unwrap();
    write!(t2, "Line 1\nLine 2\n").unwrap(); // Trailing newline
    let stats2 = compute_loc(t2.path()).unwrap();
    assert_eq!(stats2.loc, 2, "lines().count() ignores trailing newline");
}

#[test]
fn test_language_unknown_policy() {
    use crate::language::detect_language;
    use std::path::Path;

    assert_eq!(detect_language(Path::new("unknown.xyz")), "Unknown");
    assert_eq!(detect_language(Path::new("Makefile")), "Makefile");
    assert_eq!(detect_language(Path::new("makefile")), "Makefile"); // CI check
}

#[test]
fn test_serde_preserve_order_feature_is_active() {
    // This test ensures that the "preserve_order" feature of serde_json is active.
    // If it is NOT active, `Map` iteration order is undefined (or not insertion order),
    // which breaks our canonicalization assumptions if we ever rely on it implicitly.
    // While our canonicalize_object sorts explicitly, we still build the output Map
    // expecting that *that* map will be iterated in insertion order (which we made sorted).

    use serde_json::{Map, Value};
    let mut map = Map::new();
    map.insert("z".to_string(), Value::Null);
    map.insert("a".to_string(), Value::Null);
    map.insert("c".to_string(), Value::Null);

    // Insertion order: z, a, c
    let keys: Vec<&String> = map.keys().collect();
    assert_eq!(
        keys,
        vec!["z", "a", "c"],
        "serde_json::Map must preserve insertion order! Check Cargo.toml features."
    );
}

#[test]
fn test_validate_languages_mismatch() {
    let mut index = make_valid_index();
    // Valid index has "Text": 2.
    // Let's tamper with it.
    index.languages.insert("Go".to_string(), 1);

    match to_canonical_json(&index) {
        Ok(_) => panic!("Validation MUST fail on languages mismatch"),
        Err(e) => assert!(
            e.to_string().contains("Languages aggregate mismatch"),
            "Wrong error: {}",
            e
        ),
    }
}

#[test]
fn test_validate_top_dirs_mismatch() {
    let mut index = make_valid_index();
    // Valid index has ".": 2.
    // Tamper.
    index.top_dirs.insert("src".to_string(), 1);

    match to_canonical_json(&index) {
        Ok(_) => panic!("Validation MUST fail on top_dirs mismatch"),
        Err(e) => assert!(
            e.to_string().contains("Top directories aggregate mismatch"),
            "Wrong error: {}",
            e
        ),
    }
}

#[test]
fn test_validate_unknown_exclusion() {
    use crate::schema::FileNode;
    let mut index = make_valid_index();

    // Add an "Unknown" file
    // 1. Add file
    index.files.push(FileNode {
        path: "unknown.bin".to_string(),
        size: 100,
        hash: "x".to_string(),
        lang: "Unknown".to_string(),
        loc: 0,
        complexity: 0,
    });
    // 2. Sort
    index.files.sort_by(|a, b| a.path.cmp(&b.path));
    // 3. Update stats
    index.stats.file_count += 1;
    index.stats.total_size += 100;

    // 4. Update top_dirs (it's in root)
    *index.top_dirs.entry(".".to_string()).or_default() += 1;

    // DO NOT update languages. Unknown should NOT be in languages.
    // This should PASS validation.
    if let Err(e) = to_canonical_json(&index) {
        panic!("Validation failed on valid Unknown exclusion: {}", e);
    }

    // Now, FORCE "Unknown" into languages map.
    index.languages.insert("Unknown".to_string(), 1);

    // This MUST fail.
    match to_canonical_json(&index) {
        Ok(_) => panic!("Validation MUST fail if Unknown is included in languages map"),
        Err(e) => assert!(
            e.to_string().contains("Languages aggregate mismatch"),
            "Wrong error: {}",
            e
        ),
    }
}
