//! Spec 188 Phase 3: the narrow `check-config` gate over the Claude
//! shared-config slice (`.claude/settings.json` + `.mcp.json`).
//!
//! The slice hash (`build.claudeConfigHash`) is INDEPENDENT of the broad
//! `contentHash`. That independence is the load-bearing property: it lets
//! the narrow PR gate stay valid in a merge queue regardless of unrelated
//! input churn (FR-006 robustness), while still making a quiet config edit
//! impossible to merge unacknowledged (spec 184 guarantee, FR-009).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn indexer_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_codebase-indexer"))
}

/// Minimal repo with both config files present, plus one spec so the
/// broad index has an unrelated input to perturb.
fn repo_with_config() -> tempfile::TempDir {
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
    fs::write(root.join(".mcp.json"), "{\"mcpServers\":{}}\n").unwrap();
    fs::create_dir_all(root.join(".claude")).unwrap();
    fs::write(root.join(".claude/settings.json"), "{\"permissions\":{}}\n").unwrap();

    // `compile` self-validates BOTH the broad index against
    // codebase-index.schema.json AND the re-homed config-hash.json against
    // config-hash.schema.json (spec 188 Phase 4, FR-09 parity), so the temp
    // repo needs both schemas at their expected paths. Copying the in-tree
    // schemas keeps the test round-tripping through the real validation
    // contracts — the `2.`→`3.` index bump and the new config-hash schema —
    // not just asserting CLI exit codes.
    fs::create_dir_all(root.join("standards/schemas/spec-spine")).unwrap();
    for schema_rel in [
        "standards/schemas/spec-spine/codebase-index.schema.json",
        "standards/schemas/spec-spine/config-hash.schema.json",
    ] {
        let real_schema = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .join(schema_rel);
        fs::copy(&real_schema, root.join(schema_rel))
            .unwrap_or_else(|e| panic!("copy schema from {}: {e}", real_schema.display()));
    }

    tmp
}

fn run(sub: &str, repo: &Path) -> std::process::Output {
    Command::new(indexer_exe())
        .arg(sub)
        .arg("--repo")
        .arg(repo)
        .output()
        .unwrap_or_else(|e| panic!("run {sub}: {e}"))
}

fn compile(repo: &Path) {
    let out = run("compile", repo);
    assert!(
        out.status.success(),
        "compile failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn index_json(repo: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(repo.join(".derived/codebase-index/index.json"))
        .expect("read committed index");
    serde_json::from_str(&raw).expect("parse index.json")
}

fn config_hash_json(repo: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(repo.join(".derived/codebase-index/config-hash.json"))
        .expect("read committed config-hash.json");
    serde_json::from_str(&raw).expect("parse config-hash.json")
}

#[test]
fn compile_emits_64hex_claude_config_hash() {
    // Spec 188 Phase 4: the hash lives in its own re-homed file now, and
    // `compile` self-validates it against config-hash.schema.json — so a
    // successful compile already proves the value matched the schema's
    // `^[0-9a-f]{64}$` pattern. Assert shape here too for a direct signal.
    let tmp = repo_with_config();
    compile(tmp.path());
    let doc = config_hash_json(tmp.path());
    assert_eq!(
        doc["schemaVersion"].as_str(),
        Some("1.0.0"),
        "config-hash.json carries its own schema version"
    );
    let h = doc["claudeConfigHash"]
        .as_str()
        .expect("claudeConfigHash present in config-hash.json");
    assert_eq!(h.len(), 64, "claudeConfigHash must be a sha256 hex string");
    assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn index_is_3_0_0_and_carries_no_config_hash() {
    // Spec 188 Phase 4: the broad index round-trips through the real 3.0.0
    // schema (compile self-validates it) and must NOT carry the re-homed
    // field — the cache holds nothing governed.
    let tmp = repo_with_config();
    compile(tmp.path());
    let doc = index_json(tmp.path());
    assert_eq!(
        doc["schemaVersion"].as_str(),
        Some("3.0.0"),
        "broad index bumped to 3.0.0 (claudeConfigHash removed)"
    );
    assert!(
        doc["build"].get("claudeConfigHash").is_none(),
        "build.claudeConfigHash must be re-homed out of the broad index"
    );
}

#[test]
fn check_config_passes_on_fresh_index() {
    let tmp = repo_with_config();
    compile(tmp.path());
    let out = run("check-config", tmp.path());
    assert!(
        out.status.success(),
        "check-config must pass immediately after compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn editing_config_fails_check_config() {
    let tmp = repo_with_config();
    let root = tmp.path();
    compile(root);

    // A quiet edit to settings.json after the index was built.
    fs::write(root.join(".claude/settings.json"), "{\"permissions\":{\"allow\":[]}}\n").unwrap();

    let out = run("check-config", root);
    assert!(
        !out.status.success(),
        "check-config must fail when settings.json changed without re-index (spec 184 guarantee)"
    );
    assert_eq!(
        out.status.code(),
        Some(2),
        "ConfigStale must map to exit code 2"
    );
}

#[test]
fn editing_mcp_json_fails_check_config() {
    let tmp = repo_with_config();
    let root = tmp.path();
    compile(root);

    fs::write(root.join(".mcp.json"), "{\"mcpServers\":{\"x\":{}}}\n").unwrap();

    let out = run("check-config", root);
    assert!(
        !out.status.success(),
        "check-config must fail when .mcp.json changed without re-index"
    );
}

/// The merge-queue robustness property (FR-006): editing an UNRELATED
/// hashed input (a spec.md) perturbs the broad `contentHash` but MUST NOT
/// perturb the narrow config slice — so `check` fails (broad staleness)
/// while `check-config` still passes. A config-touching PR therefore can't
/// be ejected from a merge queue by another PR's unrelated code change.
#[test]
fn unrelated_input_edit_does_not_trip_check_config() {
    let tmp = repo_with_config();
    let root = tmp.path();
    compile(root);

    // Perturb an unrelated input: add a second spec.
    fs::create_dir_all(root.join("specs/001-other")).unwrap();
    fs::write(
        root.join("specs/001-other/spec.md"),
        "---\nid: \"001-other\"\nstatus: approved\n---\n# other\n",
    )
    .unwrap();

    // Broad check sees the new spec → stale.
    let broad = run("check", root);
    assert!(
        !broad.status.success(),
        "broad `check` should see the unrelated spec change as staleness"
    );

    // Narrow config gate is blind to it → still fresh.
    let narrow = run("check-config", root);
    assert!(
        narrow.status.success(),
        "check-config MUST stay green when only unrelated inputs changed (merge-queue robustness): {}",
        String::from_utf8_lossy(&narrow.stderr)
    );
}
