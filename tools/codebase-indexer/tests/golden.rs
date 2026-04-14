//! Two runs produce identical `index.json` bytes (build-meta.json excluded).

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

#[test]
fn index_json_is_deterministic_across_runs() {
    let root = repo_root();

    let out1 = open_agentic_codebase_indexer::compile(&root).expect("compile run 1");
    let out2 = open_agentic_codebase_indexer::compile(&root).expect("compile run 2");

    assert_eq!(
        out1.index_json, out2.index_json,
        "index.json must be byte-identical across runs"
    );
}
