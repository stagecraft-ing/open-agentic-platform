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
    /// Render CODEBASE-INDEX.md from existing index.json
    Render {
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
        Command::Render { repo } => {
            let root = repo.unwrap_or_else(|| std::env::current_dir().expect("cwd"));
            match open_agentic_codebase_indexer::render_to_file(&root) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("codebase-indexer: {e}");
                    3
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
    };
    std::process::exit(code);
}
