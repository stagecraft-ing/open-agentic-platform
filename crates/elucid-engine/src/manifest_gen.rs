// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/075-elucid-workflow-engine/spec.md — FR-002, FR-003

//! Manifest generation for Elucid workflows.
//!
//! Two functions:
//! - `generate_process_manifest()` — Build Spec derivation (stages s0–s5)
//! - `generate_scaffold_manifest()` — Dynamic fan-out from Build Spec (s6a–s6g)

use crate::topo_sort::topological_sort_entities;
use crate::ElucidError;
use elucid_contracts::adapter_manifest::AdapterManifest;
use elucid_contracts::build_spec::BuildSpec;
use elucid_contracts::PatternResolver;
use orchestrator::effort::EffortLevel;
use orchestrator::manifest::{StepGateConfig, VerifyCommand, WorkflowManifest, WorkflowStep};
use std::path::Path;

/// Elucid workflow metadata embedded in the manifest (FR-001).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ElucidManifestMetadata {
    /// Always `"elucid"`.
    #[serde(rename = "type")]
    pub workflow_type: String,
    /// Adapter name (e.g., `"next-prisma"`).
    pub adapter: String,
    /// Step ID that produces the frozen Build Spec.
    pub build_spec_step: String,
    /// Output artifact name for the Build Spec.
    pub build_spec_output: String,
}

// ── Stage Definitions ────────────────────────────────────────────────────────

struct StageDefinition {
    id: &'static str,
    agent_role: &'static str,
    instruction: &'static str,
    effort: EffortLevel,
    outputs: Vec<&'static str>,
    gate: Option<StepGateConfig>,
}

fn process_stages() -> Vec<StageDefinition> {
    vec![
        StageDefinition {
            id: "s0-preflight",
            agent_role: "pipeline-orchestrator",
            instruction: "Run pre-flight checks: validate business documents exist, adapter is available, all dependencies are installed.",
            effort: EffortLevel::Quick,
            outputs: vec!["preflight-report.yaml"],
            gate: None,
        },
        StageDefinition {
            id: "s1-business-requirements",
            agent_role: "business-requirements-analyst",
            instruction: "Analyze business documents and produce the business requirements deliverables: entity models, use cases, audiences, and sitemap.",
            effort: EffortLevel::Deep,
            outputs: vec!["entity-model.yaml", "use-cases.yaml", "audiences.yaml", "sitemap.yaml"],
            gate: Some(StepGateConfig::Checkpoint { label: Some("Review business requirements".into()) }),
        },
        StageDefinition {
            id: "s2-service-requirements",
            agent_role: "service-designer",
            instruction: "Design service requirements from business requirements: business rules, integrations, audit requirements, and security requirements.",
            effort: EffortLevel::Deep,
            outputs: vec!["business-rules.yaml", "integrations.yaml", "audit-spec.yaml", "security-spec.yaml"],
            gate: Some(StepGateConfig::Checkpoint { label: Some("Review service requirements".into()) }),
        },
        StageDefinition {
            id: "s3-data-model",
            agent_role: "data-architect",
            instruction: "Generate the data model from entity models and business rules: entity definitions with fields, constraints, relationships, and lifecycle states.",
            effort: EffortLevel::Deep,
            outputs: vec!["data-model.yaml"],
            gate: Some(StepGateConfig::Checkpoint { label: Some("Review data model".into()) }),
        },
        StageDefinition {
            id: "s4-api-specification",
            agent_role: "api-architect",
            instruction: "Generate the API specification from the data model and use cases: resources, operations, request/response schemas, auth requirements.",
            effort: EffortLevel::Deep,
            outputs: vec!["api-spec.yaml"],
            gate: Some(StepGateConfig::Checkpoint { label: Some("Review API specification".into()) }),
        },
        StageDefinition {
            id: "s5-ui-specification",
            agent_role: "ui-architect",
            instruction: "Generate the UI specification from the API spec and sitemap: pages, navigation, data sources, view types. Freeze the Build Spec.",
            effort: EffortLevel::Deep,
            outputs: vec!["ui-spec.yaml", "build-spec.yaml"],
            gate: Some(StepGateConfig::Approval {
                timeout_ms: 3_600_000, // 1 hour
                escalation: None,
            }),
        },
    ]
}

/// Generate Phase 1 (process stages) manifest from adapter and document paths (FR-002).
pub fn generate_process_manifest(
    adapter: &AdapterManifest,
    business_doc_paths: &[impl AsRef<Path>],
    _elucid_root: &Path,
) -> Result<WorkflowManifest, ElucidError> {
    let stages = process_stages();
    let mut steps = Vec::with_capacity(stages.len());

    for (i, stage) in stages.iter().enumerate() {
        let agent_id = format!("elucid-{}", stage.agent_role);

        // First stage takes business documents as inputs; subsequent stages
        // take outputs from the previous stage.
        let inputs: Vec<String> = if i == 0 {
            business_doc_paths
                .iter()
                .map(|p| p.as_ref().to_string_lossy().into_owned())
                .collect()
        } else {
            let prev = &stages[i - 1];
            prev.outputs
                .iter()
                .map(|o| format!("{}/{o}", prev.id))
                .collect()
        };

        let mut instruction = stage.instruction.to_string();
        instruction.push_str(&format!("\n\nAdapter: {}", adapter.adapter.name));

        steps.push(WorkflowStep {
            id: stage.id.into(),
            agent: agent_id,
            effort: stage.effort,
            inputs,
            outputs: stage.outputs.iter().map(|s| (*s).to_string()).collect(),
            instruction,
            gate: stage.gate.clone(),
            post_verify: None,
            max_retries: None,
        });
    }

    Ok(WorkflowManifest { steps })
}

/// Generate Phase 2 (scaffolding) manifest from a frozen Build Spec (FR-003).
///
/// Produces steps:
/// - s6a: Scaffold initialization
/// - s6b-*: One per entity (dependency-ordered)
/// - s6c-*: One per API operation (grouped by resource)
/// - s6d-*: One per UI page
/// - s6e: Configuration
/// - s6f: Trim
/// - s6g: Final validation
pub fn generate_scaffold_manifest(
    build_spec: &BuildSpec,
    adapter: &AdapterManifest,
    elucid_root: &Path,
) -> Result<WorkflowManifest, ElucidError> {
    let adapter_root = elucid_root.join("adapters").join(&adapter.adapter.name);
    let resolver = PatternResolver::new(&adapter_root, adapter.clone());

    let verify_commands = build_verify_commands(adapter);
    let compile_only = compile_verify_commands(adapter);

    let mut steps: Vec<WorkflowStep> = Vec::new();

    // ── s6a: Scaffold initialization ─────────────────────────────────────
    steps.push(WorkflowStep {
        id: "s6a-scaffold-init".into(),
        agent: format!("elucid-data-scaffolder-{}", adapter.adapter.name),
        effort: EffortLevel::Investigate,
        inputs: vec![],
        instruction: format!(
            "Initialize scaffold: copy adapter base project from {}, install dependencies, verify compilation.\n\
             Adapter: {}\n\
             Setup commands: {}",
            adapter.scaffold.source,
            adapter.adapter.name,
            adapter.scaffold.setup_commands.join("; "),
        ),
        outputs: vec!["scaffold-init-report.yaml".into()],
        gate: None,
        post_verify: Some(vec![
            VerifyCommand {
                command: adapter.commands.install.clone(),
                working_dir: ".".into(),
                timeout_ms: 120_000,
            },
            VerifyCommand {
                command: adapter.commands.compile.clone(),
                working_dir: ".".into(),
                timeout_ms: 60_000,
            },
        ]),
        max_retries: Some(3),
    });

    // ── s6b-*: Data entities (dependency-ordered, FR-004) ────────────────
    let entity_order = topological_sort_entities(&build_spec.data_model.entities)?;
    let mut prev_entity_ids: Vec<String> = vec!["s6a-scaffold-init".into()];

    for entity_name in &entity_order {
        let entity = build_spec
            .data_model
            .entities
            .iter()
            .find(|e| &e.name == entity_name)
            .ok_or_else(|| ElucidError::ManifestGeneration {
                reason: format!("entity {entity_name} not found after topo sort"),
            })?;

        let step_id = format!("s6b-data-{}", entity_name);

        // Inputs: depend on the previous entity steps (to respect ordering).
        let inputs: Vec<String> = prev_entity_ids
            .iter()
            .map(|prev| format!("{prev}/scaffold-init-report.yaml"))
            .take(1) // Only depend on scaffold init (topo order handles the rest)
            .collect();

        let data_pattern = resolver
            .resolve_data_pattern("migration")
            .map(|p| format!("\nData pattern: {}", p.display()))
            .unwrap_or_default();

        steps.push(WorkflowStep {
            id: step_id.clone(),
            agent: format!("elucid-data-scaffolder-{}", adapter.adapter.name),
            effort: EffortLevel::Investigate,
            inputs,
            outputs: vec![format!("{entity_name}-entity-report.yaml")],
            instruction: format!(
                "Generate data model for entity: {}\n\
                 Description: {}\n\
                 Fields: {}\n\
                 Adapter: {}{data_pattern}",
                entity.name,
                entity.description.as_deref().unwrap_or(""),
                entity
                    .fields
                    .iter()
                    .map(|f| format!("{}: {:?}", f.name, f.field_type))
                    .collect::<Vec<_>>()
                    .join(", "),
                adapter.adapter.name,
            ),
            gate: None,
            post_verify: Some(compile_only.clone()),
            max_retries: Some(3),
        });

        prev_entity_ids.push(step_id);
    }

    // ── s6c-*: API operations (grouped by resource) ──────────────────────
    for resource in &build_spec.api.resources {
        for operation in &resource.operations {
            let step_id = format!("s6c-api-{}-{}", resource.name, operation.id);

            let api_pattern = resolver
                .resolve_api_pattern("service")
                .map(|p| format!("\nAPI pattern: {}", p.display()))
                .unwrap_or_default();

            steps.push(WorkflowStep {
                id: step_id,
                agent: format!("elucid-api-scaffolder-{}", adapter.adapter.name),
                effort: EffortLevel::Investigate,
                inputs: vec![],
                outputs: vec![format!("{}-{}-report.yaml", resource.name, operation.id)],
                instruction: format!(
                    "Generate API operation: {} {}\n\
                     Resource: {} (entity: {})\n\
                     Operation: {} — {}\n\
                     Auth: {:?}\n\
                     Adapter: {}{api_pattern}",
                    operation.method.as_str(),
                    operation.path,
                    resource.name,
                    resource.entity,
                    operation.id,
                    operation.description.as_deref().unwrap_or(""),
                    operation.auth,
                    adapter.adapter.name,
                ),
                gate: None,
                post_verify: Some(verify_commands.clone()),
                max_retries: Some(3),
            });
        }
    }

    // ── s6d-*: UI pages ──────────────────────────────────────────────────
    for page in &build_spec.ui.pages {
        let step_id = format!("s6d-ui-{}", page.id);

        let page_type_str = format!("{:?}", page.page_type).to_lowercase();
        let page_pattern = resolver
            .resolve_page_type_pattern(&page_type_str)
            .map(|p| format!("\nPage type pattern: {}", p.display()))
            .unwrap_or_default();

        steps.push(WorkflowStep {
            id: step_id,
            agent: format!("elucid-ui-scaffolder-{}", adapter.adapter.name),
            effort: EffortLevel::Investigate,
            inputs: vec![],
            outputs: vec![format!("{}-page-report.yaml", page.id)],
            instruction: format!(
                "Generate UI page: {} ({})\n\
                 Path: {}\n\
                 Type: {:?}, View: {:?}\n\
                 Auth required: {}\n\
                 Data sources: {}\n\
                 Adapter: {}{page_pattern}",
                page.title,
                page.id,
                page.path,
                page.page_type,
                page.view_type,
                page.requires_auth,
                page.data_sources
                    .as_ref()
                    .map(|ds| ds
                        .iter()
                        .map(|d| d.operation_id.as_str())
                        .collect::<Vec<_>>()
                        .join(", "))
                    .unwrap_or_default(),
                adapter.adapter.name,
            ),
            gate: None,
            post_verify: Some(compile_only.clone()),
            max_retries: Some(3),
        });
    }

    // ── s6e: Configuration ───────────────────────────────────────────────
    steps.push(WorkflowStep {
        id: "s6e-configure".into(),
        agent: format!("elucid-configurer-{}", adapter.adapter.name),
        effort: EffortLevel::Investigate,
        inputs: vec![],
        outputs: vec!["configure-report.yaml".into()],
        instruction: format!(
            "Apply project configuration: set project identity ({}), wire auth ({} audiences), \
             configure environment variables, apply deployment variant ({:?}).\n\
             Adapter: {}",
            build_spec.project.name,
            build_spec.auth.audiences.len(),
            build_spec.project.variant,
            adapter.adapter.name,
        ),
        gate: None,
        post_verify: Some(compile_only.clone()),
        max_retries: Some(3),
    });

    // ── s6f: Trim ────────────────────────────────────────────────────────
    steps.push(WorkflowStep {
        id: "s6f-trim".into(),
        agent: format!("elucid-trimmer-{}", adapter.adapter.name),
        effort: EffortLevel::Investigate,
        inputs: vec![],
        outputs: vec!["trim-report.yaml".into()],
        instruction: format!(
            "Remove unused scaffold artifacts. Clean up placeholder files, unused imports, \
             and template remnants.\nAdapter: {}",
            adapter.adapter.name,
        ),
        gate: None,
        post_verify: Some(compile_only.clone()),
        max_retries: Some(3),
    });

    // ── s6g: Final validation ────────────────────────────────────────────
    steps.push(WorkflowStep {
        id: "s6g-final-validation".into(),
        agent: format!("elucid-pipeline-orchestrator-{}", adapter.adapter.name),
        effort: EffortLevel::Investigate,
        inputs: vec![],
        outputs: vec!["final-validation-report.yaml".into()],
        instruction: format!(
            "Run full project validation: build, all tests, lint, type check, invariants.\n\
             Adapter: {}",
            adapter.adapter.name,
        ),
        gate: Some(StepGateConfig::Checkpoint {
            label: Some("Review final validation results".into()),
        }),
        post_verify: Some(build_full_verify_commands(adapter)),
        max_retries: Some(3),
    });

    Ok(WorkflowManifest { steps })
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Compile + test verification commands (for API scaffolding steps).
fn build_verify_commands(adapter: &AdapterManifest) -> Vec<VerifyCommand> {
    let mut cmds = vec![VerifyCommand {
        command: adapter.commands.compile.clone(),
        working_dir: ".".into(),
        timeout_ms: 60_000,
    }];
    // Add feature_verify commands.
    for cmd in &adapter.commands.feature_verify {
        cmds.push(VerifyCommand {
            command: cmd.clone(),
            working_dir: ".".into(),
            timeout_ms: 60_000,
        });
    }
    cmds
}

/// Compile-only verification (for data/UI/config steps).
fn compile_verify_commands(adapter: &AdapterManifest) -> Vec<VerifyCommand> {
    vec![VerifyCommand {
        command: adapter.commands.compile.clone(),
        working_dir: ".".into(),
        timeout_ms: 60_000,
    }]
}

/// Full verification for final validation step.
fn build_full_verify_commands(adapter: &AdapterManifest) -> Vec<VerifyCommand> {
    let mut cmds = vec![
        VerifyCommand {
            command: adapter.commands.compile.clone(),
            working_dir: ".".into(),
            timeout_ms: 120_000,
        },
        VerifyCommand {
            command: adapter.commands.test.clone(),
            working_dir: ".".into(),
            timeout_ms: 300_000,
        },
        VerifyCommand {
            command: adapter.commands.lint.clone(),
            working_dir: ".".into(),
            timeout_ms: 60_000,
        },
    ];
    if let Some(ref tc) = adapter.commands.type_check {
        cmds.push(VerifyCommand {
            command: tc.clone(),
            working_dir: ".".into(),
            timeout_ms: 60_000,
        });
    }
    cmds
}

/// Helper: HttpMethod to string.
trait HttpMethodStr {
    fn as_str(&self) -> &'static str;
}

impl HttpMethodStr for elucid_contracts::build_spec::HttpMethod {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_manifest_has_six_stages() {
        let adapter = test_adapter();
        let manifest = generate_process_manifest(
            &adapter,
            &["/docs/business-doc.md"],
            Path::new("/tmp/elucid"),
        )
        .unwrap();

        assert_eq!(manifest.steps.len(), 6);
        assert_eq!(manifest.steps[0].id, "s0-preflight");
        assert_eq!(manifest.steps[5].id, "s5-ui-specification");

        // Stage 5 should have approval gate.
        assert!(matches!(
            manifest.steps[5].gate,
            Some(StepGateConfig::Approval { .. })
        ));

        // Stage 1-4 should have checkpoint gates.
        for step in &manifest.steps[1..5] {
            assert!(matches!(step.gate, Some(StepGateConfig::Checkpoint { .. })));
        }
    }

    #[test]
    fn process_manifest_agent_ids_prefixed() {
        let adapter = test_adapter();
        let manifest = generate_process_manifest(
            &adapter,
            &["/docs/input.md"],
            Path::new("/tmp/elucid"),
        )
        .unwrap();

        for step in &manifest.steps {
            assert!(
                step.agent.starts_with("elucid-"),
                "agent {} should start with elucid-",
                step.agent
            );
        }
    }

    fn test_adapter() -> AdapterManifest {
        use elucid_contracts::adapter_manifest::*;
        use std::collections::HashMap;

        AdapterManifest {
            schema_version: "1.0.0".into(),
            adapter: AdapterIdentity {
                name: "test-adapter".into(),
                display_name: "Test Adapter".into(),
                version: "1.0.0".into(),
                description: None,
            },
            stack: StackSpec {
                language: "typescript".into(),
                runtime: "node".into(),
                backend: BackendSpec {
                    framework: "express".into(),
                    description: "Express.js".into(),
                },
                frontend: FrontendSpec {
                    framework: "react".into(),
                    state_management: "zustand".into(),
                    design_system: "tailwind".into(),
                    description: "React".into(),
                },
                database: None,
            },
            capabilities: Capabilities {
                dual_stack: false,
                bff_pattern: false,
                single_stack: true,
                session_auth: false,
                token_auth: true,
                api_key_auth: false,
                module_system: false,
                file_uploads: false,
                background_jobs: false,
                realtime: false,
                email_notifications: false,
                audit_logging: false,
                direct_sql: false,
                orm_based: true,
                api_proxy: false,
                extra: HashMap::new(),
            },
            supported_auth: vec![],
            supported_session_stores: None,
            commands: Commands {
                install: "npm install".into(),
                compile: "npx tsc --noEmit".into(),
                test: "npm test".into(),
                lint: "npm run lint".into(),
                dev: "npm run dev".into(),
                format_check: None,
                type_check: Some("npx tsc --noEmit".into()),
                feature_verify: vec!["npx tsc --noEmit".into()],
                extra: HashMap::new(),
            },
            directory_conventions: DirectoryConventions::default(),
            patterns: Patterns {
                api: None,
                ui: None,
                data: None,
                page_types: None,
            },
            agents: Agents {
                api_scaffolder: "agents/api-scaffolder.md".into(),
                ui_scaffolder: "agents/ui-scaffolder.md".into(),
                data_scaffolder: "agents/data-scaffolder.md".into(),
                configurer: "agents/configurer.md".into(),
                trimmer: "agents/trimmer.md".into(),
                reviewer: None,
                security_auditor: None,
            },
            scaffold: Scaffold {
                source: "scaffold/".into(),
                description: "Base project".into(),
                modules: HashMap::new(),
                setup_commands: vec!["npm install".into()],
            },
            validation: Validation {
                invariants: vec![],
            },
            dual_stack: None,
        }
    }
}
