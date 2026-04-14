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

#[test]
fn w006_non_canonical_status() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let feat = root.join("specs/099-w6-test");
    fs::create_dir_all(&feat).unwrap();
    fs::write(
        feat.join("spec.md"),
        r#"---
id: "099-w6-test"
title: "t"
status: implemented
created: "2026-03-22"
summary: "x"
---
# Body
"#,
    )
    .unwrap();

    let w = lint_feature_dir(root, &feat);
    assert!(w.iter().any(|x| x.code == "W-006"));
}

#[test]
fn w006_canonical_status_ok() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    for status in &["draft", "approved", "superseded", "retired"] {
        let slug = format!("099-w6-{}", status);
        let feat = root.join(format!("specs/{}", slug));
        fs::create_dir_all(&feat).unwrap();
        fs::write(
            feat.join("spec.md"),
            format!(
                r#"---
id: "{slug}"
title: "t"
status: {status}
created: "2026-03-22"
summary: "x"
---
# Body

Superseded by `000-bootstrap-spec-system`. Retirement rationale noted.
"#
            ),
        )
        .unwrap();

        let w = lint_feature_dir(root, &feat);
        assert!(
            !w.iter().any(|x| x.code == "W-006"),
            "canonical status '{}' should not trigger W-006",
            status
        );
    }
}

#[test]
fn w007_non_canonical_implementation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let feat = root.join("specs/099-w7-test");
    fs::create_dir_all(&feat).unwrap();
    fs::write(
        feat.join("spec.md"),
        r#"---
id: "099-w7-test"
title: "t"
status: draft
implementation: done
created: "2026-03-22"
summary: "x"
---
# Body
"#,
    )
    .unwrap();

    let w = lint_feature_dir(root, &feat);
    assert!(w.iter().any(|x| x.code == "W-007"));
}

#[test]
fn w007_canonical_implementation_ok() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    for imp in &["pending", "in-progress", "complete", "n/a"] {
        let slug = format!("099-w7-{}", imp.replace('/', "-"));
        let feat = root.join(format!("specs/{}", slug));
        fs::create_dir_all(&feat).unwrap();
        fs::write(
            feat.join("spec.md"),
            format!(
                r#"---
id: "{slug}"
title: "t"
status: draft
implementation: {imp}
created: "2026-03-22"
summary: "x"
---
# Body
"#
            ),
        )
        .unwrap();

        let w = lint_feature_dir(root, &feat);
        assert!(
            !w.iter().any(|x| x.code == "W-007"),
            "canonical implementation '{}' should not trigger W-007",
            imp
        );
    }
}
