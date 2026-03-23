use clap::{Parser, Subcommand};
use open_agentic_registry_consumer::{
    authoritative_or_allow_invalid, filter_features, find_feature_by_id, features_sorted,
    load_registry, status_report, DEFAULT_REGISTRY_REL_PATH, KNOWN_STATUSES,
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
        #[arg(long)]
        id_prefix: Option<String>,
        /// Emit filtered features as a JSON array (pretty-printed)
        #[arg(long)]
        json: bool,
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
    /// Print lifecycle/status summary report
    StatusReport {
        /// Include sorted feature ids per status
        #[arg(long)]
        show_ids: bool,
        /// Emit machine-readable JSON report rows
        #[arg(long)]
        json: bool,
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

fn main() -> ExitCode {
    let cli = Cli::parse();
    let path = cli.registry_path.unwrap_or_else(default_registry_path);

    let registry = match load_registry(&path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("registry-consumer: {}: {e}", path.display());
            return ExitCode::from(3);
        }
    };

    if let Err(msg) = authoritative_or_allow_invalid(&registry, cli.allow_invalid) {
        eprintln!("registry-consumer: {msg}");
        return ExitCode::from(1);
    }

    match cli.command {
        Command::List {
            status,
            id_prefix,
            json,
        } => {
            let sorted = match features_sorted(&registry) {
                Ok(f) => f,
                Err(msg) => {
                    eprintln!("registry-consumer: {msg}");
                    return ExitCode::from(3);
                }
            };
            let filtered = filter_features(sorted, status.as_deref(), id_prefix.as_deref());
            if json {
                match serde_json::to_string_pretty(&filtered) {
                    Ok(s) => println!("{s}"),
                    Err(e) => {
                        eprintln!("registry-consumer: {e}");
                        return ExitCode::from(3);
                    }
                }
                return ExitCode::SUCCESS;
            }
            print_list_table(&filtered);
            ExitCode::SUCCESS
        }
        Command::Show {
            feature_id,
            json: _json,
            compact,
        } => {
            match find_feature_by_id(&registry, &feature_id) {
                Some(rec) => {
                    let serialized = if compact {
                        serde_json::to_string(&rec)
                    } else {
                        serde_json::to_string_pretty(&rec)
                    };
                    match serialized {
                        Ok(s) => println!("{s}"),
                        Err(e) => {
                            eprintln!("registry-consumer: {e}");
                            return ExitCode::from(3);
                        }
                    }
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!("registry-consumer: feature id not found: {feature_id}");
                    ExitCode::from(1)
                }
            }
        }
        Command::StatusReport {
            show_ids,
            json,
            nonzero_only,
            status,
        } => {
            let mut report = match status_report(&registry) {
                Ok(r) => r,
                Err(msg) => {
                    eprintln!("registry-consumer: {msg}");
                    return ExitCode::from(3);
                }
            };
            if let Some(status) = status {
                report.retain(|(row_status, _, _)| row_status == &status);
            }
            if nonzero_only {
                report.retain(|(_, count, _)| *count > 0);
            }
            if json {
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
                match serde_json::to_string_pretty(&rows) {
                    Ok(s) => println!("{s}"),
                    Err(e) => {
                        eprintln!("registry-consumer: {e}");
                        return ExitCode::from(3);
                    }
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
    println!("{:<44} {:<10} {}", "id", "status", "title");
    for f in features {
        let id = f.get("id").and_then(|x| x.as_str()).unwrap_or("?");
        let status = f.get("status").and_then(|x| x.as_str()).unwrap_or("?");
        let title = f.get("title").and_then(|x| x.as_str()).unwrap_or("?");
        let id_disp = truncate(id, 43);
        let title_disp = truncate(title, 72);
        println!("{:<44} {:<10} {}", id_disp, status, title_disp);
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let t: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{t}…")
}
