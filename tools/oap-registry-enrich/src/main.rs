use clap::{Parser, Subcommand};
use open_agentic_registry_enrich::{EnrichError, enrich_and_write};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "oap-registry-enrich",
    version,
    about = "OAP-side enricher: emits build/spec-registry/registry-oap.json from the generic registry.json + spec corpus + .factory/build-spec.yaml walk (specs 074 / 102)"
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
    /// Compute and write build/spec-registry/registry-oap.json (default).
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
        Err(EnrichError::Registry(e)) => {
            eprintln!("oap-registry-enrich: registry read failed: {e}");
            ExitCode::from(3)
        }
        Err(e) => {
            eprintln!("oap-registry-enrich: {e}");
            ExitCode::from(3)
        }
    }
}
