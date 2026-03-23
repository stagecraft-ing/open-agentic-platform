use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "spec-compiler", version, about = "Emit registry.json + build-meta.json per specs/001-spec-compiler-mvp")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Compile all specs/*/spec.md under the repository root
    Compile {
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
            match open_agentic_spec_compiler::compile_and_write(&root) {
                Ok(out) => {
                    if out.validation_passed {
                        0
                    } else {
                        1
                    }
                }
                Err(e) => {
                    eprintln!("spec-compiler: {e}");
                    3
                }
            }
        }
    };
    std::process::exit(code);
}
