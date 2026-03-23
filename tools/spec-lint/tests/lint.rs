//! Integration tests for spec-lint heuristics.

use open_agentic_spec_lint::lint_feature_dir;
use std::fs;

#[test]
fn w002_superseded_without_pointer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let feat = root.join("specs/099-w2-test");
    fs::create_dir_all(&feat).unwrap();
    fs::write(
        feat.join("spec.md"),
        r#"---
id: "099-w2-test"
title: "t"
status: superseded
created: "2026-03-22"
summary: "x"
---
# Body

No replacement here.
"#,
    )
    .unwrap();
    fs::write(feat.join("tasks.md"), "# T\n").unwrap();

    let w = lint_feature_dir(root, &feat);
    assert!(w.iter().any(|x| x.code == "W-002"));
}

#[test]
fn w002_superseded_with_backtick_id_ok() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let feat = root.join("specs/099-w2-ok");
    fs::create_dir_all(&feat).unwrap();
    fs::write(
        feat.join("spec.md"),
        r#"---
id: "099-w2-ok"
title: "t"
status: superseded
created: "2026-03-22"
summary: "x"
---
Superseded by `010-other-feature`.
"#,
    )
    .unwrap();
    fs::write(feat.join("tasks.md"), "# T\n").unwrap();

    let w = lint_feature_dir(root, &feat);
    assert!(!w.iter().any(|x| x.code == "W-002"));
}
