// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: XRAY_ANALYSIS
// Spec: spec/xray/analysis.md

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use xray::{docs, schema};

#[derive(Parser)]
#[command(name = "xray")]
#[command(about = "Deterministic repository scanner", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scans the repository and updates .axiomregent
    Scan {
        /// Target directory to scan (default: .)
        #[arg(default_value = ".")]
        target: String,

        /// Output directory override
        #[arg(long)]
        output: Option<String>,
    },
    /// Generate documentation from index
    Docs {
        /// Input index.json path (default: index.json)
        #[arg(long, default_value = "index.json")]
        input: String,

        /// Output directory (default: docs)
        #[arg(long, default_value = "docs")]
        output: String,
    },
    /// Run all steps (placeholder)
    All,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Scan { target, output } => {
            let target_path = PathBuf::from(target);
            // Default output logic is handled by CLI wrapper if needed,
            // but for now we pass the Option down or resolve it here.
            // The lib function handles the None case by NOT writing?
            // Wait, my lib impl logic for None was "do not write".
            // But CLI typically WANTS to write to default location if output is None.
            // Let's handle default path logic here to keep lib pure?
            // Actually, the previous main.rs had logic:
            // None => repo_root.join(".axiomregent").join("data")

            let final_output = match output {
                Some(p) => Some(PathBuf::from(p)),
                None => {
                    let repo_root = std::env::current_dir()?;
                    Some(repo_root.join(".axiomregent").join("data"))
                }
            };

            xray::scan_target(&target_path, final_output)?;
            Ok(())
        }
        Commands::Docs { input, output } => {
            let input_path = std::path::Path::new(input);
            let output_path = std::path::Path::new(output);

            eprintln!("Reading index from {:?}", input_path);
            let index_bytes = std::fs::read(input_path).context("Failed to read index file")?;
            let index: schema::XrayIndex =
                serde_json::from_slice(&index_bytes).context("Failed to parse index JSON")?;

            eprintln!("Generating docs to {:?}", output_path);
            let generator = docs::DocsGenerator::new(&index, output_path);
            generator.generate()?;

            eprintln!("Docs generated successfully.");
            Ok(())
        }
        Commands::All => {
            println!("All steps not implemented yet");
            Ok(())
        }
    }
}
