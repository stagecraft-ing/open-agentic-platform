use clap::Parser;
use open_agentic_spec_lint::lint_repo;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "spec-lint",
    version,
    about = "Conformance warnings W-001+ (specs/006-conformance-lint-mvp)"
)]
struct Cli {
    /// Repository root (default: current directory)
    #[arg(long)]
    repo: Option<PathBuf>,

    /// Exit with status 1 if any warning was emitted
    #[arg(long)]
    fail_on_warn: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = cli
        .repo
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));

    let warnings = lint_repo(&root);
    for w in &warnings {
        eprintln!("{} {}: {}", w.code, w.path, w.message);
    }

    if warnings.is_empty() {
        ExitCode::SUCCESS
    } else if cli.fail_on_warn {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
