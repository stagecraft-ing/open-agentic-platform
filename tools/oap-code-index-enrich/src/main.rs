use clap::{Parser, Subcommand};
use open_agentic_code_index_enrich::{EnrichError, enrich_and_write};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "oap-code-index-enrich",
    version,
    about = "OAP-side enricher: reads build/codebase-index/index.json + walks factory/adapters, .claude/{agents,commands,rules,schemas}, .github/workflows, emits index-oap.json (specs 101 + 118). Cut D W-07b will host the `render` subcommand."
)]
struct Cli {
    /// Repository root (default: current working directory)
    #[arg(long = "repo-root", value_name = "PATH")]
    repo_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compute and write build/codebase-index/index-oap.json (default).
    Enrich,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let repo_root = cli
        .repo_root
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let _command = cli.command.unwrap_or(Command::Enrich);

    match enrich_and_write(&repo_root) {
        Ok(path) => {
            println!("wrote {}", path.display());
            ExitCode::SUCCESS
        }
        Err(EnrichError::Index(e)) => {
            eprintln!("oap-code-index-enrich: codebase-index read failed: {e}");
            ExitCode::from(3)
        }
        Err(e) => {
            eprintln!("oap-code-index-enrich: {e}");
            ExitCode::from(3)
        }
    }
}
