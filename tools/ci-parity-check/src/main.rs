use clap::Parser;
use open_agentic_ci_parity_check::{check_parity, ENFORCING_WORKFLOWS};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "ci-parity-check",
    version,
    about = "Assert the root Makefile's ci targets mirror every enforcing GitHub Actions workflow (spec 104)."
)]
struct Cli {
    /// Repository root (defaults to the current working directory).
    #[arg(long)]
    repo: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = cli
        .repo
        .unwrap_or_else(|| std::env::current_dir().expect("cwd"));

    match check_parity(&root) {
        Err(e) => {
            eprintln!("ci-parity-check: {e}");
            ExitCode::from(2)
        }
        Ok(drifts) if drifts.is_empty() => {
            println!(
                "ci-parity-check: OK — Makefile mirrors {} enforcing workflow(s).",
                ENFORCING_WORKFLOWS.len()
            );
            ExitCode::SUCCESS
        }
        Ok(drifts) => {
            eprintln!("ci-parity-check: {} drift(s) detected", drifts.len());
            for d in &drifts {
                eprintln!(
                    "  [{}] {} / {} — missing token: `{}`",
                    d.workflow, d.job, d.step, d.missing_token,
                );
                eprintln!("    line: {}", d.source_line);
            }
            eprintln!();
            eprintln!("Each line is a `run:` step in an enforcing workflow whose command");
            eprintln!("tokens are not all present in the root Makefile. Add the missing");
            eprintln!("recipe to the Makefile (spec 104), or extend the allow list in");
            eprintln!("tools/ci-parity-check/src/lib.rs if the step has no local analogue.");
            ExitCode::from(1)
        }
    }
}
