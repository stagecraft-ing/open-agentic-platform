use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "codebase-indexer",
    version,
    about = "Emit index.json + build-meta.json per specs/101-codebase-index-mvp"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Full index: emit index.json + build-meta.json
    Compile {
        /// Repository root (default: current directory)
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Check if index.json is stale vs current tree
    Check {
        /// Repository root (default: current directory)
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Dump every input file's path + content hash (sorted). Diagnostic for
    /// cross-platform hash divergence (issue #46).
    DumpInputs {
        /// Repository root (default: current directory)
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Render the generic Layer 1+2+Diagnostics markdown to stdout.
    /// (Epic 2 I11: restored after Cut D W-07b moved full rendering
    /// to oap-code-index-enrich.) Generic view is the spec-spine's
    /// self-sufficient consumer protocol; OAP Layers 3-5 require
    /// `oap-code-index-enrich render`.
    Render {
        /// Repository root (default: current directory)
        #[arg(long)]
        repo: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Compile { repo } => {
            let root = repo.unwrap_or_else(|| std::env::current_dir().expect("cwd"));
            match open_agentic_codebase_indexer::compile_and_write(&root) {
                Ok(_) => 0,
                Err(e) => {
                    eprintln!("codebase-indexer: {e}");
                    match e {
                        open_agentic_codebase_indexer::IndexError::Schema(_) => 1,
                        _ => 3,
                    }
                }
            }
        }
        Command::Check { repo } => {
            let root = repo.unwrap_or_else(|| std::env::current_dir().expect("cwd"));
            match open_agentic_codebase_indexer::check(&root) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("codebase-indexer: {e}");
                    match e {
                        open_agentic_codebase_indexer::IndexError::Stale { .. } => 2,
                        open_agentic_codebase_indexer::IndexError::Blocking { .. } => 2,
                        _ => 3,
                    }
                }
            }
        }
        Command::DumpInputs { repo } => {
            let root = repo.unwrap_or_else(|| std::env::current_dir().expect("cwd"));
            match open_agentic_codebase_indexer::dump_inputs(&root) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("codebase-indexer: {e}");
                    3
                }
            }
        }
        Command::Render { repo } => {
            let root = repo.unwrap_or_else(|| std::env::current_dir().expect("cwd"));
            let path = root.join(".derived/codebase-index/index.json");
            match std::fs::read_to_string(&path) {
                Ok(raw) => match serde_json::from_str::<
                    open_agentic_codebase_indexer::types::CodebaseIndex,
                >(&raw)
                {
                    Ok(index) => {
                        print!(
                            "{}",
                            open_agentic_codebase_indexer::render::render_generic(&index)
                        );
                        0
                    }
                    Err(e) => {
                        eprintln!(
                            "codebase-indexer: failed to parse {}: {e}",
                            path.display()
                        );
                        1
                    }
                },
                Err(e) => {
                    eprintln!(
                        "codebase-indexer: failed to read {}: {e}",
                        path.display()
                    );
                    1
                }
            }
        }
    };
    std::process::exit(code);
}
