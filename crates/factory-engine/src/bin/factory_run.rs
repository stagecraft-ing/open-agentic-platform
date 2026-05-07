// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! `factory-run` CLI — runs a full Factory pipeline with real agent dispatch.
//! Supports `--resume <run-id>` to continue a previously failed pipeline.

use clap::Parser;
use factory_engine::{
    FactoryAgentBridge, FactoryEngine, FactoryEngineConfig, FactoryPipelineState,
    FactoryStandardsResolver, generate_certificate, persist_certificate,
};
use orchestrator::{
    AgentPromptLookup, ArtifactManager, AutoApproveGateHandler, ClaudeCodeExecutor, CliGateHandler,
    DispatchOptions, GateHandler, ThinkingLevel, detect_resume_plan_for_run, dispatch_manifest,
    materialize_run_directory_with_phase,
};
use factory_engine::stages::s_minus_1_extract::{KnowledgeBundleRef, sniff_mime_or_fallback};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use uuid::Uuid;

/// SHA-256 of the concatenated bytes of every supplied requirements document
/// (spec 102 FR-003 — `intent.requirementsHash`). An empty input list hashes
/// to the SHA-256 of the empty string. Unreadable paths are skipped silently;
/// the hash still reflects the contents the agents actually received.
fn compute_requirements_hash(paths: &[PathBuf]) -> String {
    let mut hasher = Sha256::new();
    for p in paths {
        if let Ok(bytes) = std::fs::read(p) {
            hasher.update(&bytes);
        }
    }
    format!("{:x}", hasher.finalize())
}

/// Generate and persist a governance certificate for the current pipeline state.
///
/// Spec 102 FR-003 / FR-009 / FR-010 — every pipeline termination (success
/// OR halt) must emit `governance-certificate.json` under the run directory.
/// Failures to persist log a warning but do not propagate; the pipeline's
/// own exit status remains the source of truth for run outcome.
fn emit_certificate(
    am: &ArtifactManager,
    run_id: Uuid,
    pipeline_state: &FactoryPipelineState,
    requirements_hash: &str,
) {
    let run_dir = am.run_dir(run_id);
    let cert = generate_certificate(pipeline_state, requirements_hash, &run_dir, None);
    let cert_path = run_dir.join("governance-certificate.json");
    match persist_certificate(&cert, &run_dir) {
        Ok(()) => eprintln!(
            "Governance certificate emitted: {} (status={:?}, hash={}…)",
            cert_path.display(),
            cert.status,
            &cert.certificate_hash[..16]
        ),
        Err(e) => eprintln!(
            "Warning: failed to persist governance certificate at {}: {e}",
            cert_path.display()
        ),
    }
}

/// Build synthetic `KnowledgeBundleRef`s from CLI-supplied `--business-docs`
/// paths. The object_id is derived from the filename; the source content
/// hash is the SHA-256 of file bytes; mime is sniffed via `infer` with an
/// extension-based fallback.
fn build_cli_bundles(paths: &[PathBuf]) -> std::io::Result<Vec<KnowledgeBundleRef>> {
    let mut out = Vec::with_capacity(paths.len());
    for p in paths {
        let path = p.canonicalize().unwrap_or_else(|_| p.clone());
        let bytes = std::fs::read(&path)?;
        let mut h = Sha256::new();
        h.update(&bytes);
        let source_content_hash = format!("{:x}", h.finalize());
        let filename = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "doc".into());
        let object_id = format!("cli:{}:{}", filename, &source_content_hash[..12]);
        let mime = sniff_mime_or_fallback(&path, None);
        out.push(KnowledgeBundleRef {
            local_path: path,
            object_id,
            source_content_hash,
            mime,
            filename,
        });
    }
    Ok(out)
}


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
    #[arg(long, default_value_t = 100)]
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

    /// Model to use for all agent dispatches (e.g., opus, sonnet, claude-opus-4-6)
    #[arg(long)]
    model: Option<String>,

    /// Use the extended 1M-token context window (appends [1m] to the model)
    #[arg(long, default_value_t = false)]
    extended_context: bool,

    /// Thinking effort level for extended thinking (low, medium, high, max)
    #[arg(long)]
    thinking: Option<ThinkingLevel>,

    /// Base timeout in seconds for Deep-effort steps (Investigate = half, Quick = quarter)
    #[arg(long, default_value_t = 300)]
    step_timeout: u64,

    /// Workspace ID for governed execution context (spec 092)
    #[arg(long)]
    workspace: Option<String>,

    /// Skip the `s-1-extract` typed-extraction stage (spec 120 FR-022).
    /// In standalone CLI use this falls back to passing `--business-docs`
    /// paths verbatim to the LLM agents. The orchestrated pipeline (OPC)
    /// always uses the typed path; only set this for ad-hoc invocations.
    #[arg(long, default_value_t = false)]
    no_pipeline_extract: bool,
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
    if cli.resume.is_none()
        && let Some(ref scaffold_src) = cli.scaffold_source
    {
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

    // Set OPC_WORKSPACE_ID env var early so all child processes inherit it (spec 092).
    // SAFETY: This runs at the start of main before any threads are spawned.
    if let Some(ref ws_id) = cli.workspace {
        unsafe { std::env::set_var("OPC_WORKSPACE_ID", ws_id) };
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
    if let Some(ref model) = cli.model {
        eprintln!("  Model:        {model}");
    }
    if cli.extended_context {
        eprintln!("  Context:      extended (1M)");
    }
    if let Some(ref thinking) = cli.thinking {
        eprintln!("  Thinking:     {}", thinking.as_str());
    }
    if let Some(ref ws) = cli.workspace {
        eprintln!("  Workspace:    {ws}");
    }
    eprintln!(
        "  Timeouts:     deep={}s / investigate={}s / quick={}s",
        cli.step_timeout,
        cli.step_timeout / 2,
        cli.step_timeout / 4,
    );
    eprintln!();

    // ── Initialize engine ───────────────────────────────────────────────
    // Spec 139 Phase 3 — `factory_root` is now a `FactoryRoot` enum; the
    // CLI always wires the Filesystem variant since it operates against
    // an on-disk checkout. The desktop wires the Virtual variant via the
    // `factory-platform-client` materialiser.
    let config = FactoryEngineConfig {
        factory_root: factory_engine::FactoryRoot::Filesystem(factory_root.clone()),
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

    let start = if cli.no_pipeline_extract || cli.business_docs.is_empty() {
        match engine.start_pipeline(&cli.adapter, &cli.business_docs, cli.workspace.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Pipeline start failed: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        let bundles = match build_cli_bundles(&cli.business_docs) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Failed to build typed bundle refs: {e}");
                return ExitCode::FAILURE;
            }
        };
        let store = match factory_engine::artifact_store::LocalArtifactStore::from_env() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to open local artifact store: {e}");
                return ExitCode::FAILURE;
            }
        };
        let cancel = tokio_util::sync::CancellationToken::new();
        let extracting = engine
            .start_pipeline_extracting(
                &cli.adapter,
                &bundles,
                &store,
                None,
                cli.workspace.clone(),
                cancel,
            )
            .await;
        match extracting {
            Ok(r) => {
                eprintln!(
                    "  s-1-extract: {} objects (deterministic={}, agent-yielded={}, failed={})",
                    r.extraction.stored.len() + r.extraction.failed.len(),
                    r.extraction.deterministic_count,
                    r.extraction.agent_yielded_count,
                    r.extraction.failed.len(),
                );
                eprintln!("  s1 context: {}", r.s1_context_path.display());
                factory_engine::PipelineStartResult {
                    run_id: r.run_id,
                    manifest: r.manifest,
                    agent_bridge: r.agent_bridge,
                    pipeline_state: r.pipeline_state,
                }
            }
            Err(e) => {
                eprintln!("s-1-extract failed: {e}");
                return ExitCode::FAILURE;
            }
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

    // Seed Factory pipeline state from the engine's start result and bind it
    // to the resolved run_id (spec 102 FR-003: state must survive across
    // both phases for certificate emission on success or halt).
    let mut pipeline_state = start.pipeline_state;
    pipeline_state.pipeline_id = run_id.to_string();

    // Hash the requirements documents once (spec 102 FR-003 — intent.requirementsHash).
    let requirements_hash = compute_requirements_hash(&cli.business_docs);

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

    if let Err(e) =
        materialize_run_directory_with_phase(&am, run_id, &start.manifest, Some("process"))
    {
        eprintln!("Failed to materialize run directory: {e}");
        return ExitCode::FAILURE;
    }

    // Create executor with agent prompt lookup and standards resolver (spec 055).
    let bridge = Arc::new(start.agent_bridge);
    let lookup = Arc::new(BridgeLookup(bridge.clone()));

    // Load coding standards for prompt injection (spec 055).
    // Standards are pre-loaded once and resolved per-agent using bridge metadata.
    let standards_resolver = match standards_loader::load_all_tiers(&project_path) {
        Ok(tiers) => {
            let resolver = FactoryStandardsResolver::new(
                bridge.clone(),
                tiers,
                standards_loader::FormatOptions::default(),
            );
            Some(Arc::new(resolver))
        }
        Err(e) => {
            eprintln!("  Warning: failed to load coding standards: {e}");
            None
        }
    };

    let mut executor_builder = ClaudeCodeExecutor::new(project_path.clone())
        .with_prompt_lookup(lookup)
        .with_max_turns(cli.max_turns)
        .with_model(cli.model.clone())
        .with_extended_context(cli.extended_context)
        .with_thinking(cli.thinking)
        .with_step_timeout(cli.step_timeout);

    if let Some(resolver) = standards_resolver {
        executor_builder = executor_builder.with_standards_resolver(resolver);
    }

    let executor = Arc::new(executor_builder);

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
        cas: None,
        artifact_metadata: None,
        governance_mode: None,
        sync_tracker: None,
        on_gate_checkpoint: None,
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
            pipeline_state.mark_failed();
            emit_certificate(&am, run_id, &pipeline_state, &requirements_hash);
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
        pipeline_state.mark_failed();
        emit_certificate(&am, run_id, &pipeline_state, &requirements_hash);
        return ExitCode::FAILURE;
    }

    let transition = match engine.transition_to_scaffolding(
        &cli.adapter,
        &build_spec_path,
        &mut pipeline_state,
        cli.org.as_deref(),
        cli.workspace.clone(),
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Phase transition failed: {e}");
            pipeline_state.mark_failed();
            emit_certificate(&am, run_id, &pipeline_state, &requirements_hash);
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
    if let Err(e) =
        materialize_run_directory_with_phase(&am, run_id, &transition.manifest, Some("scaffold"))
    {
        eprintln!("Failed to materialize Phase 2 run directory: {e}");
        pipeline_state.mark_failed();
        emit_certificate(&am, run_id, &pipeline_state, &requirements_hash);
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
        cas: None,
        artifact_metadata: None,
        governance_mode: None,
        sync_tracker: None,
        on_gate_checkpoint: None,
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
            pipeline_state.mark_failed();
            emit_certificate(&am, run_id, &pipeline_state, &requirements_hash);
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

    pipeline_state.add_tokens(total_tokens);
    pipeline_state.mark_complete();

    eprintln!("\n========================================");
    eprintln!("Factory pipeline complete");
    eprintln!("  Total steps:  {total_steps}");
    eprintln!("  Total tokens: {total_tokens}");
    eprintln!(
        "  Artifacts:    {}",
        artifact_dir.join(run_id.to_string()).display()
    );
    eprintln!("========================================");

    // Spec 102 FR-003 / FR-009 — emit governance certificate at the end of
    // every successful pipeline run.
    emit_certificate(&am, run_id, &pipeline_state, &requirements_hash);

    ExitCode::SUCCESS
}
