//! Spec 184: `.mcp.json` and `.claude/settings.json` are hashed inputs.
//!
//! AC-3: `dump-inputs` lists both files when present at their expected
//! paths.
//! AC-4: editing either file changes the content hash; the staleness
//! gate sees the edit.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn indexer_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_codebase-indexer"))
}

/// Build a minimal repo tree just rich enough for `compile` /
/// `dump-inputs` to walk: a Cargo.toml, a specs/ dir with one spec, and
/// an empty .derived/ scratch. Returns the tempdir handle so its
/// lifetime extends through the test.
fn minimal_repo() -> tempfile::TempDir {
    let tmp = tempfile::TempDir::new().expect("create tempdir");
    let root = tmp.path();

    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = []\nresolver = \"2\"\n",
    )
    .unwrap();

    fs::create_dir_all(root.join("specs/000-placeholder")).unwrap();
    fs::write(
        root.join("specs/000-placeholder/spec.md"),
        "---\nid: \"000-placeholder\"\nstatus: approved\n---\n# placeholder\n",
    )
    .unwrap();

    fs::create_dir_all(root.join(".derived")).unwrap();

    tmp
}

fn dump_inputs_lines(repo: &std::path::Path) -> String {
    let out = Command::new(indexer_exe())
        .arg("dump-inputs")
        .arg("--repo")
        .arg(repo)
        .output()
        .expect("run dump-inputs");
    assert!(
        out.status.success(),
        "dump-inputs failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf8 stdout")
}

#[test]
fn mcp_json_is_hashed_when_present() {
    let tmp = minimal_repo();
    let root = tmp.path();
    fs::write(root.join(".mcp.json"), "{\"mcpServers\":{}}\n").unwrap();

    let lines = dump_inputs_lines(root);

    assert!(
        lines.lines().any(|l| l.starts_with(".mcp.json\t")),
        ".mcp.json must appear in dump-inputs output when present.\nfull output:\n{lines}"
    );
}

#[test]
fn claude_settings_json_is_hashed_when_present() {
    let tmp = minimal_repo();
    let root = tmp.path();
    fs::create_dir_all(root.join(".claude")).unwrap();
    fs::write(root.join(".claude/settings.json"), "{\"permissions\":{}}\n").unwrap();

    let lines = dump_inputs_lines(root);

    assert!(
        lines
            .lines()
            .any(|l| l.starts_with(".claude/settings.json\t")),
        ".claude/settings.json must appear in dump-inputs output when present.\nfull output:\n{lines}"
    );
}

#[test]
fn shared_config_absent_does_not_panic() {
    // A downstream consumer of this repo with neither file present must
    // still see a clean dump-inputs run.
    let tmp = minimal_repo();
    let lines = dump_inputs_lines(tmp.path());

    assert!(!lines.lines().any(|l| l.starts_with(".mcp.json\t")));
    assert!(!lines
        .lines()
        .any(|l| l.starts_with(".claude/settings.json\t")));
}

fn hash_for(lines: &str, path: &str) -> Option<String> {
    lines
        .lines()
        .find(|l| l.starts_with(&format!("{path}\t")))
        .and_then(|l| l.split('\t').nth(1).map(str::to_owned))
}

#[test]
fn editing_shared_config_changes_input_hashes() {
    let tmp = minimal_repo();
    let root = tmp.path();
    fs::write(root.join(".mcp.json"), "{\"mcpServers\":{}}\n").unwrap();
    fs::create_dir_all(root.join(".claude")).unwrap();
    fs::write(root.join(".claude/settings.json"), "{\"permissions\":{}}\n").unwrap();

    let baseline = dump_inputs_lines(root);
    let baseline_mcp = hash_for(&baseline, ".mcp.json").expect("baseline mcp hash");
    let baseline_settings =
        hash_for(&baseline, ".claude/settings.json").expect("baseline settings hash");

    // Edit .mcp.json — its per-file digest must change.
    fs::write(root.join(".mcp.json"), "{\"mcpServers\":{\"x\":{}}}\n").unwrap();
    let after_mcp = dump_inputs_lines(root);
    let new_mcp = hash_for(&after_mcp, ".mcp.json").expect("post-edit mcp hash");
    assert_ne!(
        baseline_mcp, new_mcp,
        "editing .mcp.json must change its hashed-input digest (AC-4)"
    );

    // Edit .claude/settings.json — its per-file digest must change too.
    fs::write(
        root.join(".claude/settings.json"),
        "{\"permissions\":{\"allow\":[]}}\n",
    )
    .unwrap();
    let after_settings = dump_inputs_lines(root);
    let new_settings =
        hash_for(&after_settings, ".claude/settings.json").expect("post-edit settings hash");
    assert_ne!(
        baseline_settings, new_settings,
        "editing .claude/settings.json must change its hashed-input digest (AC-4)"
    );
}
