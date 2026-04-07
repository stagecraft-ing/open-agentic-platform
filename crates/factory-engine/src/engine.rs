// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/075-factory-workflow-engine/spec.md

//! Two-phase Factory execution engine.
//!
//! Orchestrates:
//! 1. Phase 1 (Process): stages s0–s5 producing a Build Spec
//! 2. Phase transition: Build Spec freeze → generate Phase 2 manifest
//! 3. Phase 2 (Scaffolding): dynamic fan-out from Build Spec (s6a–s6g)
//!
//! Uses the orchestrator primitives for dispatch, gates, state persistence,
//! and artifact management. Adds Factory-specific verification, retry logic,
//! and pipeline state tracking.

use crate::agent_bridge::FactoryAgentBridge;
use crate::manifest_gen::{generate_process_manifest, generate_scaffold_manifest};
use crate::pipeline_state::{FactoryPipelineState, FailedFeature};
use crate::policy_shard::generate_factory_policy_shard;
use crate::verify_harness::run_factory_gate_check;
use crate::FactoryError;
use factory_contracts::build_spec::BuildSpec;
use factory_contracts::AdapterRegistry;
use orchestrator::manifest::WorkflowManifest;
use policy_kernel::PolicyBundle;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Configuration for the Factory engine.
#[derive(Clone, Debug)]
pub struct FactoryEngineConfig {
    /// Path to the Factory root directory (e.g., `factory/`).
    pub factory_root: PathBuf,
    /// Path to the target project directory.
    pub project_path: PathBuf,
    /// Maximum concurrent scaffolding steps (NF-001).
    pub concurrency_limit: usize,
    /// Maximum total token budget for the pipeline (NF-002).
    pub max_total_tokens: Option<u64>,
}

impl Default for FactoryEngineConfig {
    fn default() -> Self {
        Self {
            factory_root: PathBuf::from("factory"),
            project_path: PathBuf::from("."),
            concurrency_limit: 4,
            max_total_tokens: None,
        }
    }
}

/// The Factory two-phase workflow engine.
///
/// Coordinates process stages, Build Spec freeze, and scaffolding fan-out
/// using OAP orchestrator primitives.
pub struct FactoryEngine {
    config: FactoryEngineConfig,
    adapter_registry: AdapterRegistry,
}

impl FactoryEngine {
    /// Create a new engine with adapter discovery.
    pub fn new(config: FactoryEngineConfig) -> Result<Self, FactoryError> {
        let adapter_registry =
            AdapterRegistry::discover(&config.factory_root).map_err(|e| {
                FactoryError::AdapterNotFound {
                    name: format!("discovery failed: {e}"),
                }
            })?;

        Ok(Self {
            config,
            adapter_registry,
        })
    }

    /// Create an engine with pre-loaded adapters (for testing).
    pub fn with_adapters(
        config: FactoryEngineConfig,
        adapter_registry: AdapterRegistry,
    ) -> Self {
        Self {
            config,
            adapter_registry,
        }
    }

    /// Start a new Factory pipeline.
    ///
    /// Returns the `run_id` and the initial `FactoryPipelineState`.
    ///
    /// This generates the Phase 1 manifest and returns it for dispatch.
    /// The caller is responsible for dispatching via the orchestrator.
    pub fn start_pipeline(
        &self,
        adapter_name: &str,
        business_doc_paths: &[PathBuf],
    ) -> Result<PipelineStartResult, FactoryError> {
        let adapter = self
            .adapter_registry
            .get(adapter_name)
            .ok_or_else(|| FactoryError::AdapterNotFound {
                name: adapter_name.into(),
            })?;

        let run_id = Uuid::new_v4();

        // Generate Phase 1 manifest.
        let phase1_manifest = generate_process_manifest(
            adapter,
            business_doc_paths,
            &self.config.factory_root,
        )?;

        // Create agent bridge.
        let process_agents = factory_contracts::agent_loader::load_process_agents(
            &self.config.factory_root,
        )
        .unwrap_or_default();
        let adapter_path = self
            .config
            .factory_root
            .join("adapters")
            .join(adapter_name);
        let adapter_agents =
            factory_contracts::agent_loader::load_adapter_agents(&adapter_path)
                .unwrap_or_default();
        let agent_bridge = FactoryAgentBridge::new(process_agents, adapter_agents);

        // Create initial pipeline state.
        let pipeline_state = FactoryPipelineState::new(run_id.to_string(), adapter_name);

        Ok(PipelineStartResult {
            run_id,
            manifest: phase1_manifest,
            agent_bridge,
            pipeline_state,
        })
    }

    /// Perform the phase transition from Process to Scaffolding.
    ///
    /// Called after stage s5 completes and the Build Spec is frozen.
    /// Reads the Build Spec artifact, hashes it, generates the Phase 2 manifest.
    ///
    /// `org_override` injects `project.org` when the agent-produced spec omits it.
    pub fn transition_to_scaffolding(
        &self,
        adapter_name: &str,
        build_spec_path: &Path,
        pipeline_state: &mut FactoryPipelineState,
        org_override: Option<&str>,
    ) -> Result<PhaseTransitionResult, FactoryError> {
        let adapter = self
            .adapter_registry
            .get(adapter_name)
            .ok_or_else(|| FactoryError::AdapterNotFound {
                name: adapter_name.into(),
            })?;

        // Read and parse the Build Spec.
        let build_spec_yaml = std::fs::read_to_string(build_spec_path).map_err(|e| {
            FactoryError::InvalidBuildSpec {
                reason: format!("read {}: {e}", build_spec_path.display()),
            }
        })?;

        let mut build_spec: BuildSpec =
            serde_yaml::from_str(&build_spec_yaml).map_err(|e| {
                FactoryError::InvalidBuildSpec {
                    reason: format!("parse: {e}"),
                }
            })?;

        // Inject org if the agent omitted it and the operator supplied one.
        if build_spec.project.org.is_empty() {
            if let Some(org) = org_override {
                build_spec.project.org = org.to_string();
            }
        }

        // Hash the frozen Build Spec.
        let hash = {
            let mut hasher = Sha256::new();
            hasher.update(build_spec_yaml.as_bytes());
            format!("{:x}", hasher.finalize())
        };

        // Transition pipeline state.
        pipeline_state.transition_to_scaffolding(hash);

        // Generate policy shard (FR-008).
        let policy_bundle = generate_factory_policy_shard(adapter, &build_spec);

        // Generate Phase 2 manifest (FR-003).
        let phase2_manifest = generate_scaffold_manifest(
            &build_spec,
            adapter,
            &self.config.factory_root,
        )?;

        Ok(PhaseTransitionResult {
            manifest: phase2_manifest,
            policy_bundle,
        })
    }

    /// Run gate checks for a completed stage (FR-010).
    pub async fn run_gate_checks(
        &self,
        stage_id: &str,
    ) -> Result<crate::verify_harness::GateCheckResult, FactoryError> {
        run_factory_gate_check(
            stage_id,
            &self.config.project_path,
            &self.config.factory_root,
        )
        .await
    }

    /// Get the engine configuration.
    pub fn config(&self) -> &FactoryEngineConfig {
        &self.config
    }

    /// Get the adapter registry.
    pub fn adapter_registry(&self) -> &AdapterRegistry {
        &self.adapter_registry
    }
}

/// Result of starting a pipeline.
pub struct PipelineStartResult {
    /// Unique run identifier.
    pub run_id: Uuid,
    /// Phase 1 (process) manifest ready for dispatch.
    pub manifest: WorkflowManifest,
    /// Agent bridge for the pipeline.
    pub agent_bridge: FactoryAgentBridge,
    /// Initial pipeline state.
    pub pipeline_state: FactoryPipelineState,
}

/// Result of the Phase 1→2 transition (Build Spec freeze → scaffolding).
pub struct PhaseTransitionResult {
    /// Phase 2 (scaffolding) manifest ready for dispatch.
    pub manifest: WorkflowManifest,
    /// Policy bundle generated from the adapter manifest and Build Spec (FR-008).
    /// Callers should apply this to the axiomregent permission runtime.
    pub policy_bundle: PolicyBundle,
}

/// Classify a scaffolding step ID to determine what it produced.
pub fn classify_scaffold_step(step_id: &str) -> ScaffoldStepKind {
    if step_id.starts_with("s6b-data-") {
        ScaffoldStepKind::Entity(step_id.strip_prefix("s6b-data-").unwrap().into())
    } else if step_id.starts_with("s6c-api-") {
        ScaffoldStepKind::Operation(step_id.strip_prefix("s6c-api-").unwrap().into())
    } else if step_id.starts_with("s6d-ui-") {
        ScaffoldStepKind::Page(step_id.strip_prefix("s6d-ui-").unwrap().into())
    } else if step_id == "s6a-scaffold-init" {
        ScaffoldStepKind::Init
    } else if step_id == "s6e-configure" {
        ScaffoldStepKind::Configure
    } else if step_id == "s6f-trim" {
        ScaffoldStepKind::Trim
    } else if step_id == "s6g-review" {
        ScaffoldStepKind::Review
    } else if step_id == "s6h-final-validation" {
        ScaffoldStepKind::FinalValidation
    } else {
        ScaffoldStepKind::Unknown(step_id.into())
    }
}

/// Classification of scaffold step types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScaffoldStepKind {
    Init,
    Entity(String),
    Operation(String),
    Page(String),
    Configure,
    Trim,
    Review,
    FinalValidation,
    Unknown(String),
}

/// Update pipeline state from a completed scaffolding step.
pub fn record_scaffold_completion(
    state: &mut FactoryPipelineState,
    step_id: &str,
    tokens: u64,
) {
    state.add_tokens(tokens);
    match classify_scaffold_step(step_id) {
        ScaffoldStepKind::Entity(name) => state.entity_completed(name),
        ScaffoldStepKind::Operation(name) => state.operation_completed(name),
        ScaffoldStepKind::Page(name) => state.page_completed(name),
        _ => {}
    }
}

/// Update pipeline state from a failed scaffolding step.
pub fn record_scaffold_failure(
    state: &mut FactoryPipelineState,
    step_id: &str,
    retries: u32,
    error: &str,
) {
    let feature = FailedFeature {
        name: step_id.into(),
        step_id: step_id.into(),
        retries,
        last_error: error.into(),
    };
    match classify_scaffold_step(step_id) {
        ScaffoldStepKind::Entity(_) => state.entity_failed(feature),
        ScaffoldStepKind::Operation(_) => state.operation_failed(feature),
        ScaffoldStepKind::Page(_) => state.page_failed(feature),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_step_ids() {
        assert_eq!(
            classify_scaffold_step("s6b-data-Organization"),
            ScaffoldStepKind::Entity("Organization".into())
        );
        assert_eq!(
            classify_scaffold_step("s6c-api-orgs-list"),
            ScaffoldStepKind::Operation("orgs-list".into())
        );
        assert_eq!(
            classify_scaffold_step("s6d-ui-dashboard"),
            ScaffoldStepKind::Page("dashboard".into())
        );
        assert_eq!(
            classify_scaffold_step("s6a-scaffold-init"),
            ScaffoldStepKind::Init
        );
        assert_eq!(
            classify_scaffold_step("s6g-review"),
            ScaffoldStepKind::Review
        );
        assert_eq!(
            classify_scaffold_step("s6h-final-validation"),
            ScaffoldStepKind::FinalValidation
        );
    }

    #[test]
    fn record_completion_updates_state() {
        let mut state = FactoryPipelineState::new("test", "adapter");
        state.transition_to_scaffolding("hash".into());

        record_scaffold_completion(&mut state, "s6b-data-Org", 1000);
        record_scaffold_completion(&mut state, "s6c-api-orgs-list", 2000);
        record_scaffold_completion(&mut state, "s6d-ui-dashboard", 500);

        assert_eq!(state.total_tokens, 3500);
        let sp = state.scaffolding.as_ref().unwrap();
        assert_eq!(sp.entities_completed, vec!["Org"]);
        assert_eq!(sp.operations_completed, vec!["orgs-list"]);
        assert_eq!(sp.pages_completed, vec!["dashboard"]);
    }

    #[test]
    fn record_failure_updates_state() {
        let mut state = FactoryPipelineState::new("test", "adapter");
        state.transition_to_scaffolding("hash".into());

        record_scaffold_failure(&mut state, "s6b-data-Widget", 3, "compile error");

        let sp = state.scaffolding.as_ref().unwrap();
        assert_eq!(sp.entities_failed.len(), 1);
        assert_eq!(sp.entities_failed[0].retries, 3);
    }
}
