// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! CLI entry point for the artifact extractor.
//!
//! Default layout mirrors `extract_artifacts.py`: given a project root that
//! contains `.artifacts/raw/`, we extract under `.artifacts/extracted/`.
//! Explicit `--raw` and `--out` override the defaults.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use artifact_extract::{RunOptions, run};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "artifact-extract",
    about = "Extract business artifacts (DOCX/XLSX/PPTX/PDF/JSON/PBIX/ZIP) to plain text.",
    long_about = None,
)]
struct Args {
    /// Project root containing .artifacts/raw/ (and target for
    /// .artifacts/extracted/). Defaults to the current directory.
    #[arg(long, default_value = ".")]
    root: PathBuf,

    /// Explicit raw directory (overrides `<root>/.artifacts/raw/`).
    #[arg(long)]
    raw: Option<PathBuf>,

    /// Explicit output directory (overrides `<root>/.artifacts/extracted/`).
    #[arg(long)]
    out: Option<PathBuf>,

    /// Force re-extraction even when destinations look up-to-date.
    #[arg(long)]
    force: bool,

    /// Suppress per-file output; only the SUMMARY block is printed.
    #[arg(long)]
    quiet: bool,

    /// Emit per-file and summary output as JSON lines on stdout instead of
    /// the human-readable text form. The final line is the summary object.
    #[arg(long)]
    json: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();

    let raw_dir = args
        .raw
        .unwrap_or_else(|| args.root.join(".artifacts").join("raw"));
    let extracted_dir = args
        .out
        .unwrap_or_else(|| args.root.join(".artifacts").join("extracted"));

    let opts = RunOptions {
        raw_dir: raw_dir.clone(),
        extracted_dir,
        force: args.force,
    };

    if !opts.raw_dir.exists() {
        eprintln!("Raw dir missing: {}", opts.raw_dir.display());
        return ExitCode::from(2);
    }

    if !args.quiet {
        let count = walkdir::WalkDir::new(&opts.raw_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();
        if args.json {
            println!(
                "{}",
                serde_json::json!({
                    "kind": "run-start",
                    "raw_dir": opts.raw_dir.display().to_string(),
                    "file_count": count
                })
            );
        } else {
            println!("Found {count} files under {}", opts.raw_dir.display());
        }
    }

    let result = run(&opts, |src, outcome| {
        if args.quiet {
            return;
        }
        let rel = src.strip_prefix(&opts.raw_dir).unwrap_or(src);
        let rel_posix = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        if args.json {
            println!(
                "{}",
                serde_json::json!({
                    "kind": "file",
                    "path": rel_posix,
                    "status": outcome.status.tag(),
                    "message": outcome.message,
                })
            );
        } else {
            println!(
                "[{}] {} -- {}",
                outcome.status.tag(),
                rel_posix,
                outcome.message
            );
        }
    });

    let counts = match result {
        Ok(c) => c,
        Err(e) => {
            eprintln!("run failed: {e}");
            return ExitCode::from(2);
        }
    };

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "kind": "summary",
                "ok": counts.ok,
                "cached": counts.cached,
                "error": counts.error,
                "skip_unsupported": counts.skip_unsupported,
            })
        );
    } else if !args.quiet {
        println!("\n=== SUMMARY ===");
        println!("ok: {}", counts.ok);
        println!("cached: {}", counts.cached);
        println!("error: {}", counts.error);
        println!("skip-unsupported: {}", counts.skip_unsupported);
    }

    if counts.has_errors() {
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    }
}

// Ensures compiler checks the Path import is in scope for doc examples.
#[allow(dead_code)]
fn _unused(_: &Path) {}
