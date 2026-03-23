use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let grammars_dir = manifest_dir.join("../../grammars");

    let dirs: Vec<PathBuf> = [
        "tree-sitter-rust",
        "tree-sitter-python",
        "tree-sitter-javascript",
    ]
    .iter()
    .map(|name| grammars_dir.join(name).join("src"))
    .collect();

    let mut cc_build = cc::Build::new();

    for dir in dirs {
        cc_build.include(&dir).file(dir.join("parser.c"));

        if !dir.ends_with("tree-sitter-c/src") {
            cc_build.file(dir.join("scanner.c"));
        }
    }

    cc_build.compile("tree-sitter-languages");
}
