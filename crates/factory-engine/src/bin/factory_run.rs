// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! `factory-run` CLI — runs a full Factory pipeline with real agent dispatch.
//! Supports `--resume <run-id>` to continue a previously failed pipeline.

use clap::Parser;
use factory_engine::{FactoryAgentBridge, FactoryEngine, FactoryEngineConfig};
use orchestrator::{
    AgentPromptLookup, ArtifactManager, AutoApproveGateHandler, ClaudeCodeExecutor, CliGateHandler,
    DispatchOptions, GateHandler, detect_resume_plan_for_run, dispatch_manifest,
    materialize_run_directory,
};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "factory-run",
    about = "Run a full Factory pipeline with real agent dispatch"
)]
struct Cli {
    /// Adapter name (e.g., aim-vue-node, next-prisma, rust-axum, encore-react)
    #[arg(long)]
    adapter: String,

    /// Path to the target project directory (will be created if needed)
    #[arg(long)]
    project: PathBuf,

    /// Path(s) to business requirement documents
    #[arg(long, num_args = 1..)]
    business_docs: Vec<PathBuf>,

    /// Path to the Factory root directory (default: factory/)
    #[arg(long, default_value = "factory")]
    factory_root: PathBuf,

    /// Auto-approve all gates without interactive prompts
    #[arg(long, default_value_t = false)]
    auto_approve: bool,

    /// Maximum agentic turns per step
    #[arg(long, default_value_t = 25)]
    max_turns: u32,

    /// Organisation slug (e.g. goa-cfs). Injected into the Build Spec if the
    /// agent did not produce one.
    #[arg(long)]
    org: Option<String>,

    /// Resume a previously failed pipeline run by its run ID
    #[arg(long)]
    resume: Option<Uuid>,

    /// Path to the scaffold source template directory to copy into the project
    #[arg(long)]
    scaffold_source: Option<PathBuf>,
}

/// Adapts FactoryAgentBridge to the orchestrator's AgentPromptLookup trait.
struct BridgeLookup(Arc<FactoryAgentBridge>);

impl AgentPromptLookup for BridgeLookup {
    fn get_prompt(&self, agent_id: &str) -> Option<String> {
        self.0.get_prompt(agent_id).map(String::from)
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Ensure project directory exists.
    if let Err(e) = std::fs::create_dir_all(&cli.project) {
        eprintln!("Failed to create project directory: {e}");
        return ExitCode::FAILURE;
    }

    let project_path = match cli.project.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to resolve project path: {e}");
            return ExitCode::FAILURE;
        }
    };

    let factory_root = match cli.factory_root.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to resolve factory root: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Copy scaffold source into project directory if provided (skip on --resume).
    if cli.resume.is_none() {
        if let Some(ref scaffold_src) = cli.scaffold_source {
            if !scaffold_src.exists() {
                eprintln!("Scaffold source does not exist: {}", scaffold_src.display());
                return ExitCode::FAILURE;
            }
            eprintln!(
                "Copying scaffold from {} into project...",
                scaffold_src.display()
            );
            let status = std::process::Command::new("rsync")
                .args([
                    "-a",
                    "--exclude",
                    ".git",
                    &format!("{}/", scaffold_src.display()),
                    &format!("{}/", project_path.display()),
                ])
                .status();
            match status {
                Ok(s) if s.success() => eprintln!("  Scaffold copied successfully"),
                Ok(s) => {
                    eprintln!(
                        "  Scaffold copy failed with exit code: {}",
                        s.code().unwrap_or(-1)
                    );
                    return ExitCode::FAILURE;
                }
                Err(e) => {
                    eprintln!("  Scaffold copy failed: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    eprintln!("Factory pipeline starting");
    eprintln!("  Adapter:      {}", cli.adapter);
    eprintln!("  Project:      {}", project_path.display());
    eprintln!("  Factory root: {}", factory_root.display());
    eprintln!("  Business docs: {:?}", cli.business_docs);
    eprintln!("  Auto-approve: {}", cli.auto_approve);
    if let Some(resume_id) = &cli.resume {
        eprintln!("  Resuming:     {resume_id}");
    }
    if let Some(ref src) = cli.scaffold_source {
        eprintln!("  Scaffold src: {}", src.display());
    }
    eprintln!();

    // ── Initialize engine ───────────────────────────────────────────────
    let config = FactoryEngineConfig {
        factory_root: factory_root.clone(),
        project_path: project_path.clone(),
        concurrency_limit: 4,
        max_total_tokens: None,
    };

    let engine = match FactoryEngine::new(config) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Engine initialization failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    // ── Phase 1: Process stages ─────────────────────────────────────────
    eprintln!("Phase 1: Generating process manifest (s0-s5)...");

    let start = match engine.start_pipeline(&cli.adapter, &cli.business_docs) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Pipeline start failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Use the resume run ID if provided, otherwise the new one.
    let run_id = cli.resume.unwrap_or(start.run_id);
    eprintln!("  Run ID: {run_id}");
    eprintln!(
        "  Stages: {} | Agents: {}",
        start.manifest.steps.len(),
        start.agent_bridge.len()
    );

    // Set up artifact manager under the project directory.
    let artifact_dir = project_path.join(".factory").join("runs");
    let am = ArtifactManager::new(&artifact_dir);

    // Detect resume plan if resuming an existing run.
    let phase1_skip: HashSet<String> = if cli.resume.is_some() {
        match detect_resume_plan_for_run(&am, run_id, &start.manifest) {
            Ok(Some(plan)) => {
                eprintln!(
                    "  Resuming: skipping {} completed steps, starting from step {}",
                    plan.completed_step_ids.len(),
                    plan.first_non_completed_step_index
                );
                plan.completed_step_ids.into_iter().collect()
            }
            Ok(None) => {
                eprintln!("  No prior state found for {run_id}, starting fresh");
                HashSet::new()
            }
            Err(e) => {
                eprintln!("  Warning: failed to load resume state: {e}, starting fresh");
                HashSet::new()
            }
        }
    } else {
        HashSet::new()
    };

    if let Err(e) = materialize_run_directory(&am, run_id, &start.manifest) {
        eprintln!("Failed to materialize run directory: {e}");
        return ExitCode::FAILURE;
    }

    // Create executor with agent prompt lookup.
    let bridge = Arc::new(start.agent_bridge);
    let lookup = Arc::new(BridgeLookup(bridge.clone()));

    let executor = Arc::new(
        ClaudeCodeExecutor::new(project_path.clone())
            .with_prompt_lookup(lookup)
            .with_max_turns(cli.max_turns),
    );

    // Create gate handler.
    let gate_handler: Arc<dyn GateHandler> = if cli.auto_approve {
        Arc::new(AutoApproveGateHandler)
    } else {
        Arc::new(CliGateHandler)
    };

    let options = DispatchOptions {
        gate_handler: Some(gate_handler.clone()),
        project_root: Some(project_path.clone()),
        skip_completed_steps: phase1_skip,
    };

    eprintln!("\nDispatching Phase 1...\n");

    let summary1 = match dispatch_manifest(
        &am,
        run_id,
        &start.manifest,
        bridge.clone(),
        executor.clone(),
        &options,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\nPhase 1 dispatch failed: {e}");
            eprintln!("To resume, re-run with: --resume {run_id}");
            return ExitCode::FAILURE;
        }
    };

    let phase1_tokens: u64 = summary1.steps.iter().filter_map(|s| s.tokens_used).sum();
    eprintln!(
        "\nPhase 1 complete: {} steps, {} tokens",
        summary1.steps.len(),
        phase1_tokens
    );

    for step in &summary1.steps {
        eprintln!("  {} — {:?}", step.step_id, step.status);
    }

    // ── Phase transition ────────────────────────────────────────────────
    eprintln!("\nPhase transition: reading frozen Build Spec...");

    let build_spec_path = am.output_artifact_path(run_id, "s5-ui-specification", "build-spec.yaml");
    if !build_spec_path.exists() {
        eprintln!("Build Spec not found at {}", build_spec_path.display());
        return ExitCode::FAILURE;
    }

    let mut pipeline_state =
        factory_engine::FactoryPipelineState::new(run_id.to_string(), &cli.adapter);

    let transition = match engine.transition_to_scaffolding(
        &cli.adapter,
        &build_spec_path,
        &mut pipeline_state,
        cli.org.as_deref(),
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Phase transition failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    eprintln!(
        "  Phase 2 manifest: {} steps",
        transition.manifest.steps.len()
    );
    eprintln!("  Policy shards: {}", transition.policy_bundle.shards.len());

    // ── Phase 2: Scaffolding ────────────────────────────────────────────
    eprintln!("\nDispatching Phase 2...\n");

    // Materialize Phase 2 run directory (reuse same run_id).
    if let Err(e) = materialize_run_directory(&am, run_id, &transition.manifest) {
        eprintln!("Failed to materialize Phase 2 run directory: {e}");
        return ExitCode::FAILURE;
    }

    // Detect Phase 2 resume plan if resuming an existing run.
    let phase2_skip: HashSet<String> = if cli.resume.is_some() {
        match detect_resume_plan_for_run(&am, run_id, &transition.manifest) {
            Ok(Some(plan)) => {
                eprintln!(
                    "  Resuming Phase 2: skipping {} completed steps, starting from step {}",
                    plan.completed_step_ids.len(),
                    plan.first_non_completed_step_index
                );
                plan.completed_step_ids.into_iter().collect()
            }
            Ok(None) => {
                eprintln!("  No prior Phase 2 state found, starting fresh");
                HashSet::new()
            }
            Err(e) => {
                eprintln!("  Warning: failed to load Phase 2 resume state: {e}, starting fresh");
                HashSet::new()
            }
        }
    } else {
        HashSet::new()
    };

    let phase2_options = DispatchOptions {
        gate_handler: Some(gate_handler),
        project_root: Some(project_path.clone()),
        skip_completed_steps: phase2_skip,
    };

    let summary2 = match dispatch_manifest(
        &am,
        run_id,
        &transition.manifest,
        bridge.clone(),
        executor,
        &phase2_options,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\nPhase 2 dispatch failed: {e}");
            eprintln!("To resume, re-run with: --resume {run_id}");
            return ExitCode::FAILURE;
        }
    };

    let phase2_tokens: u64 = summary2.steps.iter().filter_map(|s| s.tokens_used).sum();
    eprintln!(
        "\nPhase 2 complete: {} steps, {} tokens",
        summary2.steps.len(),
        phase2_tokens
    );

    for step in &summary2.steps {
        eprintln!("  {} — {:?}", step.step_id, step.status);
    }

    // ── Summary ─────────────────────────────────────────────────────────
    let total_tokens = phase1_tokens + phase2_tokens;
    let total_steps = summary1.steps.len() + summary2.steps.len();

    eprintln!("\n========================================");
    eprintln!("Factory pipeline complete");
    eprintln!("  Total steps:  {total_steps}");
    eprintln!("  Total tokens: {total_tokens}");
    eprintln!(
        "  Artifacts:    {}",
        artifact_dir.join(run_id.to_string()).display()
    );
    eprintln!("========================================");

    ExitCode::SUCCESS
}
