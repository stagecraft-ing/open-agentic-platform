use clap::{Parser, Subcommand};
use open_agentic_registry_consumer::{
    DEFAULT_REGISTRY_REL_PATH, KNOWN_IMPLEMENTATIONS, KNOWN_STATUSES,
    authoritative_or_allow_invalid, features_sorted, filter_features, find_feature_by_id,
    load_registry, serialize_json_compact_or_pretty, status_report,
};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "registry-consumer",
    version,
    about = "Read-only CLI over build/spec-registry/registry.json (specs/002-registry-consumer-mvp)"
)]
struct Cli {
    /// Path to registry.json (default: build/spec-registry/registry.json relative to cwd)
    #[arg(long = "registry-path", value_name = "PATH")]
    registry_path: Option<PathBuf>,

    /// Allow reading when validation.passed is false (diagnostics only)
    #[arg(long)]
    allow_invalid: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List features (human-readable table), sorted by id
    List {
        #[arg(long)]
        status: Option<String>,
        /// Filter by implementation lifecycle status
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(KNOWN_IMPLEMENTATIONS))]
        implementation: Option<String>,
        #[arg(long)]
        id_prefix: Option<String>,
        /// Emit filtered features as a JSON array (pretty-printed)
        #[arg(long, conflicts_with_all = ["compact", "ids_only"])]
        json: bool,
        /// Single-line compact JSON array (mutually exclusive with --json)
        #[arg(long, conflicts_with_all = ["json", "ids_only"])]
        compact: bool,
        /// Emit only feature ids, one per line (sorted/filter semantics preserved)
        #[arg(long = "ids-only", conflicts_with_all = ["json", "compact"])]
        ids_only: bool,
    },
    /// Print one feature record as JSON
    Show {
        feature_id: String,
        /// Explicit JSON contract output (pretty-printed object; same as default show today)
        #[arg(long, conflicts_with = "compact")]
        json: bool,
        /// Single-line compact JSON object (mutually exclusive with --json)
        #[arg(long, conflicts_with = "json")]
        compact: bool,
    },
    /// Generate compliance framework-to-spec mapping (spec 102 FR-025)
    ComplianceReport {
        /// Filter to a specific framework identifier (e.g. "owasp-asi-2026")
        #[arg(long)]
        framework: Option<String>,
        /// Emit as JSON
        #[arg(long)]
        json: bool,
    },
    /// Print lifecycle/status summary report
    StatusReport {
        /// Include sorted feature ids per status
        #[arg(long)]
        show_ids: bool,
        /// Emit machine-readable JSON report rows
        #[arg(long, conflicts_with = "compact")]
        json: bool,
        /// Single-line compact JSON array of report rows (mutually exclusive with --json)
        #[arg(long, conflicts_with = "json")]
        compact: bool,
        /// Omit statuses with zero counts
        #[arg(long)]
        nonzero_only: bool,
        /// Filter report to one lifecycle status
        #[arg(long, value_parser = clap::builder::PossibleValuesParser::new(KNOWN_STATUSES))]
        status: Option<String>,
    },
}

fn default_registry_path() -> PathBuf {
    PathBuf::from(DEFAULT_REGISTRY_REL_PATH)
}

fn exit_with_prefixed_message(code: u8, message: impl std::fmt::Display) -> ExitCode {
    eprintln!("registry-consumer: {message}");
    ExitCode::from(code)
}

fn print_json_or_exit<T: serde::Serialize>(value: &T, compact: bool) -> Result<(), ExitCode> {
    match serialize_json_compact_or_pretty(value, compact) {
        Ok(s) => {
            println!("{s}");
            Ok(())
        }
        Err(e) => Err(exit_with_prefixed_message(3, e)),
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let path = cli.registry_path.unwrap_or_else(default_registry_path);

    let registry = match load_registry(&path) {
        Ok(v) => v,
        Err(e) => return exit_with_prefixed_message(3, format_args!("{}: {e}", path.display())),
    };

    if let Err(msg) = authoritative_or_allow_invalid(&registry, cli.allow_invalid) {
        return exit_with_prefixed_message(1, msg);
    }

    match cli.command {
        Command::List {
            status,
            implementation,
            id_prefix,
            json,
            compact,
            ids_only,
        } => {
            let sorted = match features_sorted(&registry) {
                Ok(f) => f,
                Err(msg) => return exit_with_prefixed_message(3, msg),
            };
            let filtered = filter_features(
                sorted,
                status.as_deref(),
                id_prefix.as_deref(),
                implementation.as_deref(),
            );
            if json || compact {
                if let Err(code) = print_json_or_exit(&filtered, compact) {
                    return code;
                }
                return ExitCode::SUCCESS;
            }
            if ids_only {
                print_list_ids(&filtered);
                return ExitCode::SUCCESS;
            }
            print_list_table(&filtered);
            ExitCode::SUCCESS
        }
        Command::Show {
            feature_id,
            json: _json,
            compact,
        } => match find_feature_by_id(&registry, &feature_id) {
            Some(rec) => {
                if let Err(code) = print_json_or_exit(&rec, compact) {
                    return code;
                }
                ExitCode::SUCCESS
            }
            None => {
                exit_with_prefixed_message(1, format_args!("feature id not found: {feature_id}"))
            }
        },
        Command::ComplianceReport { framework, json } => {
            let sorted = match features_sorted(&registry) {
                Ok(f) => f,
                Err(msg) => return exit_with_prefixed_message(3, msg),
            };
            // Build a map of control → vec of spec IDs that declare coverage.
            let mut control_map: std::collections::BTreeMap<String, Vec<String>> =
                std::collections::BTreeMap::new();
            for f in &sorted {
                let id = f.get("id").and_then(|x| x.as_str()).unwrap_or("?");
                if let Some(compliance) = f.get("compliance").and_then(|v| v.as_array()) {
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
                                    control_map
                                        .entry(key)
                                        .or_default()
                                        .push(id.to_string());
                                }
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
                if let Err(code) = print_json_or_exit(&rows, false) {
                    return code;
                }
            } else {
                println!("{:<40} {:<6} specs", "control", "count");
                for (control, specs) in &control_map {
                    println!("{:<40} {:<6} {}", control, specs.len(), specs.join(", "));
                }
            }
            ExitCode::SUCCESS
        }
        Command::StatusReport {
            show_ids,
            json,
            compact,
            nonzero_only,
            status,
        } => {
            let mut report = match status_report(&registry) {
                Ok(r) => r,
                Err(msg) => return exit_with_prefixed_message(3, msg),
            };
            if let Some(status) = status {
                report.retain(|(row_status, _, _)| row_status == &status);
            }
            if nonzero_only {
                report.retain(|(_, count, _)| *count > 0);
            }
            if json || compact {
                let rows: Vec<serde_json::Value> = report
                    .into_iter()
                    .map(|(status, count, ids)| {
                        serde_json::json!({
                            "status": status,
                            "count": count,
                            "ids": ids
                        })
                    })
                    .collect();
                if let Err(code) = print_json_or_exit(&rows, compact) {
                    return code;
                }
                return ExitCode::SUCCESS;
            }
            for (status, count, ids) in report {
                println!("{:<10} {}", status, count);
                if show_ids {
                    if ids.is_empty() {
                        println!("  ids: (none)");
                    } else {
                        println!("  ids: {}", ids.join(", "));
                    }
                }
            }
            ExitCode::SUCCESS
        }
    }
}

fn print_list_table(features: &[serde_json::Value]) {
    // id (max ~40), status (8), title — simple fixed layout
    println!("{:<44} {:<10} title", "id", "status");
    for f in features {
        let id = f.get("id").and_then(|x| x.as_str()).unwrap_or("?");
        let status = f.get("status").and_then(|x| x.as_str()).unwrap_or("?");
        let title = f.get("title").and_then(|x| x.as_str()).unwrap_or("?");
        let id_disp = truncate(id, 43);
        let title_disp = truncate(title, 72);
        println!("{:<44} {:<10} {}", id_disp, status, title_disp);
    }
}

fn print_list_ids(features: &[serde_json::Value]) {
    for f in features {
        let id = f.get("id").and_then(|x| x.as_str()).unwrap_or("?");
        println!("{id}");
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let t: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{t}…")
}
