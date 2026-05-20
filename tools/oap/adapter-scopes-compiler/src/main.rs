use clap::Parser;
use open_agentic_adapter_scopes_compiler::{compile_from_adapters_dir, serialize_to_string};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "adapter-scopes-compiler",
    version,
    about = "Compile factory/adapters/*/manifest.yaml into adapter-scopes.json (spec 105)."
)]
struct Cli {
    /// Repository root. The tool writes outputs relative to this.
    #[arg(long)]
    repo: Option<PathBuf>,

    /// Override adapters directory (defaults to `<repo>/factory/adapters`).
    #[arg(long)]
    adapters_dir: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let repo = cli
        .repo
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));
    let adapters_dir = cli
        .adapters_dir
        .unwrap_or_else(|| repo.join("factory").join("adapters"));

    let compiled = match compile_from_adapters_dir(&adapters_dir) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("adapter-scopes-compiler: {e}");
            return ExitCode::from(1);
        }
    };

    let json = match serialize_to_string(&compiled) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("adapter-scopes-compiler: {e}");
            return ExitCode::from(1);
        }
    };

    let build_dir = repo.join("build");
    if let Err(e) = std::fs::create_dir_all(&build_dir) {
        eprintln!(
            "adapter-scopes-compiler: create {}: {e}",
            build_dir.display()
        );
        return ExitCode::from(1);
    }

    let build_path = build_dir.join("adapter-scopes.json");
    if let Err(e) = std::fs::write(&build_path, &json) {
        eprintln!("adapter-scopes-compiler: write {}: {e}", build_path.display());
        return ExitCode::from(1);
    }
    println!("wrote {}", build_path.display());

    let stagecraft_path = repo
        .join("platform")
        .join("services")
        .join("stagecraft")
        .join("api")
        .join("factory")
        .join("adapter-scopes.json");
    if let Err(e) = std::fs::write(&stagecraft_path, &json) {
        eprintln!(
            "adapter-scopes-compiler: write {}: {e}",
            stagecraft_path.display()
        );
        return ExitCode::from(1);
    }
    println!("wrote {}", stagecraft_path.display());

    for (name, scope) in &compiled.adapters {
        println!("  {name}:");
        println!("    file_write_scope: [{}]", scope.file_write_scope.join(", "));
        println!("    allowed_commands: [{}]", scope.allowed_commands.join(", "));
    }

    ExitCode::SUCCESS
}
