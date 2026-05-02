//! `spec-code-coupling-check` binary entrypoint (spec 127).

use clap::Parser;
use open_agentic_spec_code_coupling_check::{check_coupling, load_index, render};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

#[derive(Parser, Debug)]
#[command(
    name = "spec-code-coupling-check",
    about = "PR-time gate: every diff path claimed by a spec's `implements:` list must be accompanied by an edit to that spec's spec.md (spec 127).",
    version
)]
struct Cli {
    /// Repo root (defaults to current working directory).
    #[arg(long, default_value = ".")]
    repo: PathBuf,

    /// Base ref for the diff (default: origin/main).
    #[arg(long, default_value = "origin/main")]
    base: String,

    /// Head ref for the diff (default: HEAD).
    #[arg(long, default_value = "HEAD")]
    head: String,

    /// Override the diff: read newline-delimited paths from this file.
    /// When set, --base/--head are ignored. Useful for tests and for
    /// CI flows that pipe `gh pr diff --name-only` from a previous step.
    #[arg(long)]
    paths_from: Option<PathBuf>,

    /// PR body (waiver source). Path to a file containing the PR body;
    /// defaults to empty if unset and $GITHUB_PR_BODY is also unset.
    #[arg(long)]
    pr_body: Option<PathBuf>,

    /// Path to the codebase index (default: build/codebase-index/index.json
    /// resolved from --repo).
    #[arg(long)]
    index: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let index_path = cli.index.clone().unwrap_or_else(|| {
        cli.repo.join("build/codebase-index/index.json")
    });
    let index = match load_index(&index_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("spec-code-coupling-check: {e}");
            return ExitCode::from(2);
        }
    };

    let diff_paths = match collect_diff_paths(&cli) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("spec-code-coupling-check: {e}");
            return ExitCode::from(2);
        }
    };

    let pr_body = match read_pr_body(&cli) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("spec-code-coupling-check: {e}");
            return ExitCode::from(2);
        }
    };

    let outcome = check_coupling(&index, &diff_paths, &pr_body);
    let rendered = render(&outcome);
    if !rendered.is_empty() {
        // Stdout for clean run summaries; stderr for failure blocks so
        // the violation header lands in the GitHub Actions step error pane.
        if outcome.exit_code() == 0 {
            println!("{rendered}");
        } else {
            eprintln!("{rendered}");
        }
    }

    let code = outcome.exit_code();
    if code == 0 {
        if outcome.violations.is_empty() {
            println!("spec-code-coupling-check: OK — {} diff path(s) checked.", diff_paths.len());
        }
        ExitCode::SUCCESS
    } else {
        ExitCode::from(code as u8)
    }
}

fn collect_diff_paths(cli: &Cli) -> Result<BTreeSet<String>, String> {
    if let Some(path) = &cli.paths_from {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("read --paths-from {}: {e}", path.display()))?;
        Ok(text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect())
    } else {
        // git diff --name-only <base>...<head>
        let out = Command::new("git")
            .arg("-C")
            .arg(&cli.repo)
            .args(["diff", "--name-only"])
            .arg(format!("{}...{}", cli.base, cli.head))
            .output()
            .map_err(|e| format!("spawn git diff: {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!(
                "git diff exited {:?}: {stderr}",
                out.status.code()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect())
    }
}

fn read_pr_body(cli: &Cli) -> Result<String, String> {
    if let Some(path) = &cli.pr_body {
        std::fs::read_to_string(path)
            .map_err(|e| format!("read --pr-body {}: {e}", path.display()))
    } else if let Ok(s) = std::env::var("GITHUB_PR_BODY") {
        Ok(s)
    } else {
        Ok(String::new())
    }
}
