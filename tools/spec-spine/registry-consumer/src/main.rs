use clap::{Parser, Subcommand};
use open_agentic_spec_registry_reader::{
    DEFAULT_REGISTRY_REL_PATH, Feature, FeatureFilter, KNOWN_IMPLEMENTATIONS, KNOWN_STATUSES,
    Registry, RegistryError, load, serialize_json_canonical,
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
        /// Filter by `kind:` frontmatter value (exact match; spec 147 AC-006)
        #[arg(long)]
        kind: Option<String>,
        /// Filter by `shape:` frontmatter value (exact match; spec 147 AC-006)
        #[arg(long)]
        shape: Option<String>,
        /// Filter by `category:` list membership (exact match against any list entry; spec 147 AC-006)
        #[arg(long)]
        category: Option<String>,
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
    /// Show outgoing and incoming relationships for a spec
    ShowRelationships {
        spec_id: String,
        /// Emit structured JSON output
        #[arg(long)]
        json: bool,
    },
    /// Walk supersedes relationships back and forward from a spec
    ShowSupersessionChain {
        spec_id: String,
        /// Emit structured JSON output
        #[arg(long)]
        json: bool,
    },
    /// Find specs with constrains: pointing at a target spec's target_specs
    ShowConstraintsOn {
        spec_id: String,
        /// Emit structured JSON output
        #[arg(long)]
        json: bool,
    },
    /// Output the authority set for a code path
    ByAuthority {
        path: String,
        /// Filter to co_authority entries claiming this section name
        #[arg(long)]
        section: Option<String>,
        /// Emit structured JSON output
        #[arg(long)]
        json: bool,
    },
    /// Validate the spec relationship graph for structural problems
    ValidateGraph {
        /// Emit problems as a JSON array
        #[arg(long)]
        json: bool,
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
    // Route every CLI JSON emission through `serialize_json_canonical` so
    // object keys are lex-ordered regardless of `serde_json`'s
    // `preserve_order` feature (active workspace-wide via `crates/xray`).
    // See `open_agentic_spec_registry_reader::canonicalize_value` for
    // the full rationale; this is the single emission boundary for
    // CLI stdout JSON.
    match serialize_json_canonical(value, compact) {
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

    let registry: Registry = match load(&path) {
        Ok(r) => r,
        Err(RegistryError::Io(e)) => {
            return exit_with_prefixed_message(3, format_args!("{}: {e}", path.display()));
        }
        Err(RegistryError::Json(e)) => {
            return exit_with_prefixed_message(3, format_args!("{}: {e}", path.display()));
        }
        Err(RegistryError::UnknownSchemaVersion(v)) => {
            return exit_with_prefixed_message(
                3,
                format_args!("{}: unsupported registry specVersion: {v}", path.display()),
            );
        }
        Err(RegistryError::MissingFeaturesArray) => {
            return exit_with_prefixed_message(3, "missing features array");
        }
    };

    if let Err(msg) = registry.authoritative_or_allow_invalid(cli.allow_invalid) {
        return exit_with_prefixed_message(1, msg);
    }

    match cli.command {
        Command::List {
            status,
            implementation,
            id_prefix,
            kind,
            shape,
            category,
            json,
            compact,
            ids_only,
        } => {
            let filtered = registry.filter(FeatureFilter {
                status: status.as_deref(),
                id_prefix: id_prefix.as_deref(),
                implementation: implementation.as_deref(),
                kind: kind.as_deref(),
                shape: shape.as_deref(),
                category: category.as_deref(),
            });
            if json || compact {
                let raws: Vec<&serde_json::Value> = filtered.iter().map(|f| &f.raw).collect();
                if let Err(code) = print_json_or_exit(&raws, compact) {
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
        } => match registry.find_by_id(&feature_id) {
            Some(f) => {
                if let Err(code) = print_json_or_exit(&f.raw, compact) {
                    return code;
                }
                ExitCode::SUCCESS
            }
            None => {
                exit_with_prefixed_message(1, format_args!("feature id not found: {feature_id}"))
            }
        },
        Command::StatusReport {
            show_ids,
            json,
            compact,
            nonzero_only,
            status,
        } => {
            let mut report = registry.status_report();
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
        Command::ShowRelationships { spec_id, json } => {
            match registry.graph_relationships(&spec_id) {
                None => exit_with_prefixed_message(
                    1,
                    format_args!("feature id not found: {spec_id}"),
                ),
                Some(view) => {
                    if json {
                        if let Err(code) = print_json_or_exit(&view, false) {
                            return code;
                        }
                    } else {
                        print_relationships_human(&view);
                    }
                    ExitCode::SUCCESS
                }
            }
        }
        Command::ShowSupersessionChain { spec_id, json } => {
            match registry.supersession_chain(&spec_id) {
                None => exit_with_prefixed_message(
                    1,
                    format_args!("feature id not found: {spec_id}"),
                ),
                Some(chain) => {
                    if json {
                        if let Err(code) = print_json_or_exit(&chain, false) {
                            return code;
                        }
                    } else {
                        print_supersession_chain_human(&chain);
                    }
                    ExitCode::SUCCESS
                }
            }
        }
        Command::ShowConstraintsOn { spec_id, json } => {
            let result = registry.constraints_on(&spec_id);
            if json {
                let rows: Vec<serde_json::Value> = result
                    .iter()
                    .map(|(sid, kind)| {
                        serde_json::json!({
                            "spec_id": sid,
                            "kind": kind
                        })
                    })
                    .collect();
                if let Err(code) = print_json_or_exit(&rows, false) {
                    return code;
                }
            } else {
                if result.is_empty() {
                    println!("No constraints point at {spec_id}");
                } else {
                    println!("Constraints pointing at {spec_id}:");
                    for (sid, kind) in &result {
                        println!("  {:<44} kind: {}", sid, kind);
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Command::ByAuthority { path, section, json } => {
            let result = registry.authority_for_path(&path, section.as_deref());
            if json {
                if let Err(code) = print_json_or_exit(&result, false) {
                    return code;
                }
            } else {
                if result.is_empty() {
                    println!("No specs claim authority over: {path}");
                } else {
                    println!("Authority set for path: {path}");
                    for entry in &result {
                        println!("  {:<44} via: {}", entry.spec_id, entry.relationship);
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Command::ValidateGraph { json } => {
            let problems = registry.validate_graph();
            if json {
                if let Err(code) = print_json_or_exit(&problems, false) {
                    return code;
                }
            } else {
                if problems.is_empty() {
                    println!("Graph validation passed: no problems found.");
                } else {
                    for p in &problems {
                        println!("[{}] {}", p.kind, p.message);
                    }
                }
            }
            if problems.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
    }
}

fn print_relationships_human(view: &open_agentic_spec_registry_reader::RelationshipView) {
    println!("Relationships for: {}", view.spec_id);
    println!();
    println!("Outgoing ({}):", view.outgoing.len());
    if view.outgoing.is_empty() {
        println!("  (none)");
    } else {
        for edge in &view.outgoing {
            let spec_part = edge
                .spec
                .as_deref()
                .map(|s| format!(" → {s}"))
                .unwrap_or_default();
            let paths_part = if edge.paths.is_empty() {
                String::new()
            } else {
                format!(" [{}]", edge.paths.join(", "))
            };
            println!("  {}{}{}", edge.kind, spec_part, paths_part);
        }
    }
    println!();
    println!("Incoming ({}):", view.incoming.len());
    if view.incoming.is_empty() {
        println!("  (none)");
    } else {
        for edge in &view.incoming {
            println!("  {} ← {}", edge.kind, edge.from_spec);
        }
    }
}

fn print_supersession_chain_human(
    chain: &[open_agentic_spec_registry_reader::ChainEntry],
) {
    println!("Supersession chain (oldest → newest):");
    for (i, entry) in chain.iter().enumerate() {
        let scope_part = if entry.scope.is_empty() {
            String::new()
        } else {
            format!(" (scope: {})", entry.scope)
        };
        let arrow = if i == 0 { "  " } else { "→ " };
        println!("  {}{}{}", arrow, entry.spec_id, scope_part);
    }
}

fn print_list_table(features: &[&Feature]) {
    // id (max ~40), status (8), title — simple fixed layout
    println!("{:<44} {:<10} title", "id", "status");
    for f in features {
        let id = f.id.as_str();
        let status = f.status.as_deref().unwrap_or("?");
        let title = f.title.as_deref().unwrap_or("?");
        let id_disp = truncate(id, 43);
        let title_disp = truncate(title, 72);
        println!("{:<44} {:<10} {}", id_disp, status, title_disp);
    }
}

fn print_list_ids(features: &[&Feature]) {
    for f in features {
        println!("{}", f.id);
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let t: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{t}…")
}
