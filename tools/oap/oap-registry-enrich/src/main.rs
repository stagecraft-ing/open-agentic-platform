use clap::{Parser, Subcommand};
use open_agentic_registry_enrich::{EnrichError, enrich_and_write};
use open_agentic_spec_registry_reader as srr;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "oap-registry-enrich",
    version,
    about = "OAP-side enricher: emits build/spec-registry/registry-oap.json from the generic registry.json + spec corpus + .factory/build-spec.yaml walk (specs 074 / 102). Hosts the compliance-report subcommand migrated from registry-consumer (Cut D W-06b)."
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
    /// Generate compliance framework-to-spec mapping (spec 102 FR-025).
    /// Moved from `registry-consumer compliance-report` in Cut D W-06b.
    /// Post-W-06c the default registry path is the enriched
    /// `registry-oap.json` (compliance is no longer emitted by
    /// spec-compiler into the generic `registry.json`).
    ComplianceReport {
        /// Path to the enriched registry-oap.json (default:
        /// build/spec-registry/registry-oap.json)
        #[arg(long = "registry-path", value_name = "PATH")]
        registry_path: Option<PathBuf>,
        /// Filter to a specific framework identifier (e.g. "owasp-asi-2026")
        #[arg(long)]
        framework: Option<String>,
        /// Emit as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let repo_root = cli
        .repo_root
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    match cli.command.unwrap_or(Command::Enrich) {
        Command::Enrich => match enrich_and_write(&repo_root) {
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
        },
        Command::ComplianceReport {
            registry_path,
            framework,
            json,
        } => {
            let path = registry_path
                .unwrap_or_else(|| repo_root.join(".derived/spec-registry/registry-oap.json"));
            let registry = match srr::load(&path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("oap-registry-enrich: {}: {e}", path.display());
                    return ExitCode::from(3);
                }
            };
            let mut control_map: std::collections::BTreeMap<String, Vec<String>> =
                std::collections::BTreeMap::new();
            for f in registry.features_sorted() {
                let Some(compliance) = f.compliance.as_ref().and_then(|v| v.as_array()) else {
                    continue;
                };
                for entry in compliance {
                    let fw = entry
                        .get("framework")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if let Some(ref filter) = framework {
                        if fw != filter.as_str() {
                            continue;
                        }
                    }
                    if let Some(controls) = entry.get("controls").and_then(|v| v.as_array()) {
                        for c in controls {
                            if let Some(ctrl) = c.as_str() {
                                let key = format!("{fw}/{ctrl}");
                                control_map.entry(key).or_default().push(f.id.clone());
                            }
                        }
                    }
                }
            }
            if control_map.is_empty() {
                eprintln!("No compliance mappings found.");
                return ExitCode::from(0);
            }
            if json {
                let rows: Vec<serde_json::Value> = control_map
                    .iter()
                    .map(|(control, specs)| {
                        serde_json::json!({
                            "control": control,
                            "specIds": specs,
                            "count": specs.len()
                        })
                    })
                    .collect();
                match serde_json::to_string_pretty(&rows) {
                    Ok(s) => println!("{s}"),
                    Err(e) => {
                        eprintln!("oap-registry-enrich: {e}");
                        return ExitCode::from(3);
                    }
                }
            } else {
                println!("{:<40} {:<6} specs", "control", "count");
                for (control, specs) in &control_map {
                    println!("{:<40} {:<6} {}", control, specs.len(), specs.join(", "));
                }
            }
            ExitCode::SUCCESS
        }
    }
}
