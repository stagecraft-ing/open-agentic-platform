// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/165-opc-decomposition-pipeline/spec.md — FR-008 / SC-005

//! Promotion: lift a staged draft spec into the project's own spec spine.
//!
//! FR-008: promotion writes the staged `spec.md` into
//! `<project>/specs/<target-slug>/spec.md`, invokes the project's
//! spec-compiler (recompiling `<project>/.derived/spec-registry/`), and
//! runs the project's coupling gate as a sanity check before completing.
//!
//! Design choices:
//! - The spec-compiler is called **in-process** via its library
//!   (`open_agentic_spec_compiler::compile_and_write`) — deterministic, no
//!   binary-path discovery.
//! - The coupling gate is shelled out to the project's `spec-code-coupling-check`
//!   binary in `--paths-from` mode (the binary already orchestrates the
//!   index/registry/diff machinery). It is **optional**: when no binary is
//!   supplied the step is skipped. A non-zero gate result is surfaced but
//!   does not abort promotion — promotion of a pure-spec change is a sanity
//!   check, not a hard gate (the developer reviews the signal).
//! - Promotion refuses to clobber an existing target spec directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::PipelineError;

/// What to promote and where.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionRequest {
    /// The target project's working-tree root.
    pub project_root: PathBuf,
    /// The decomposition run directory (`<output_root>/<run_id>/`).
    pub run_root: PathBuf,
    /// Slug of the staged spec directory under `s6-synthesis/specs/`.
    pub staged_slug: String,
    /// Final `NNN-slug` the developer chose for `<project>/specs/`.
    pub target_slug: String,
}

/// Outcome of one of promotion's sub-steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    pub ran: bool,
    pub ok: bool,
    pub detail: String,
}

impl StepResult {
    fn skipped(detail: impl Into<String>) -> Self {
        Self { ran: false, ok: true, detail: detail.into() }
    }
}

/// Result of a completed promotion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionOutcome {
    /// Path of the promoted spec relative to the project root
    /// (`specs/<target-slug>/spec.md`).
    pub promoted_relpath: String,
    /// spec-compiler recompile of the project registry (FR-008).
    pub compile: StepResult,
    /// coupling-gate sanity check (FR-008).
    pub coupling: StepResult,
}

/// Promote a staged spec into the project's spec spine. `coupling_check_bin`
/// is the path to the project's `spec-code-coupling-check` binary, or `None`
/// to skip the gate sanity check.
pub fn promote_spec(
    req: &PromotionRequest,
    coupling_check_bin: Option<&Path>,
) -> Result<PromotionOutcome, PipelineError> {
    let staged = req
        .run_root
        .join("s6-synthesis")
        .join("specs")
        .join(&req.staged_slug)
        .join("spec.md");
    if !staged.is_file() {
        return Err(PipelineError::Promotion(format!(
            "staged spec not found: {}",
            staged.display()
        )));
    }

    let target_dir = req.project_root.join("specs").join(&req.target_slug);
    if target_dir.exists() {
        return Err(PipelineError::Promotion(format!(
            "target spec already exists, refusing to clobber: {}",
            target_dir.display()
        )));
    }

    // Rewrite the staged spec's id/slug to the chosen target so the
    // promoted spec is self-consistent and compiles under its new dir name.
    let body = fs::read_to_string(&staged).map_err(|e| PipelineError::io(&staged, e))?;
    let rewritten = rewrite_id_slug(&body, &req.target_slug);
    fs::create_dir_all(&target_dir).map_err(|e| PipelineError::io(&target_dir, e))?;
    let target_spec = target_dir.join("spec.md");
    fs::write(&target_spec, rewritten).map_err(|e| PipelineError::io(&target_spec, e))?;

    let promoted_relpath = format!("specs/{}/spec.md", req.target_slug);

    // FR-008: invoke the project's spec-compiler (in-process).
    let compile = match open_agentic_spec_compiler::compile_and_write(&req.project_root) {
        Ok(out) => StepResult {
            ran: true,
            ok: out.validation_passed,
            detail: if out.validation_passed {
                "registry recompiled; validation passed".to_string()
            } else {
                "registry recompiled; validation failures present".to_string()
            },
        },
        Err(e) => StepResult {
            ran: true,
            ok: false,
            detail: format!("spec-compiler error: {e}"),
        },
    };

    // FR-008: run the project's coupling gate as a sanity check.
    let coupling = run_coupling_gate(coupling_check_bin, req, &promoted_relpath)?;

    Ok(PromotionOutcome {
        promoted_relpath,
        compile,
        coupling,
    })
}

/// Rewrite the first `id:` and `slug:` lines inside the YAML frontmatter to
/// the target slug. Leaves the body untouched.
fn rewrite_id_slug(body: &str, target_slug: &str) -> String {
    let mut out = String::with_capacity(body.len() + 16);
    let mut in_frontmatter = false;
    let mut frontmatter_done = false;
    let mut id_done = false;
    let mut slug_done = false;

    for (i, line) in body.lines().enumerate() {
        if i == 0 && line.trim() == "---" {
            in_frontmatter = true;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_frontmatter && !frontmatter_done && line.trim() == "---" {
            frontmatter_done = true;
            in_frontmatter = false;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_frontmatter && !frontmatter_done {
            if !id_done && line.trim_start().starts_with("id:") {
                out.push_str(&format!("id: \"{target_slug}\"\n"));
                id_done = true;
                continue;
            }
            if !slug_done && line.trim_start().starts_with("slug:") {
                out.push_str(&format!("slug: {target_slug}\n"));
                slug_done = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn run_coupling_gate(
    bin: Option<&Path>,
    req: &PromotionRequest,
    promoted_relpath: &str,
) -> Result<StepResult, PipelineError> {
    let Some(bin) = bin else {
        return Ok(StepResult::skipped("coupling gate skipped (no binary supplied)"));
    };

    // The gate's --paths-from mode takes a newline-delimited file of repo
    // paths; write it into the run scratch dir.
    let paths_file = req.run_root.join("promotion-coupling-paths.txt");
    fs::write(&paths_file, format!("{promoted_relpath}\n"))
        .map_err(|e| PipelineError::io(&paths_file, e))?;

    let output = Command::new(bin)
        .arg("--repo")
        .arg(&req.project_root)
        .arg("--paths-from")
        .arg(&paths_file)
        .output();

    match output {
        Ok(o) => {
            let code = o.status.code().unwrap_or(-1);
            let detail = format!(
                "coupling gate exit {code}: {}",
                String::from_utf8_lossy(if o.stderr.is_empty() { &o.stdout } else { &o.stderr })
                    .trim()
            );
            Ok(StepResult {
                ran: true,
                ok: o.status.success(),
                detail,
            })
        }
        Err(e) => Ok(StepResult {
            ran: true,
            ok: false,
            detail: format!("coupling gate spawn failed: {e}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PipelineConfig, PipelineRunner};

    fn fixture_project(root: &Path) {
        let crates_a = root.join("crates").join("alpha");
        fs::create_dir_all(&crates_a).unwrap();
        fs::write(crates_a.join("lib.rs"), "fn alpha() {}\n").unwrap();
    }

    /// Run the pipeline and return (run_root, first staged slug).
    fn stage_a_spec(project_root: &Path) -> (PathBuf, String) {
        let output_root = project_root.join(".opc").join("decomposition");
        let cfg = PipelineConfig {
            project_root: project_root.to_path_buf(),
            knowledge_bundle: None,
            output_root: output_root.clone(),
            embeddings_enabled: false,
        };
        let run = PipelineRunner::new(cfg).run().unwrap();
        let slug = run.emitted_specs[0].slug.clone();
        (output_root.join(run.run_id.as_str()), slug)
    }

    #[test]
    fn promotes_staged_spec_and_recompiles_registry() {
        let project = tempfile::tempdir().unwrap();
        fixture_project(project.path());
        let (run_root, staged_slug) = stage_a_spec(project.path());

        let req = PromotionRequest {
            project_root: project.path().to_path_buf(),
            run_root,
            staged_slug,
            target_slug: "001-promoted-demo".to_string(),
        };
        let outcome = promote_spec(&req, None).unwrap();

        // Spec written into the project's spine with rewritten id/slug.
        let promoted = project.path().join("specs/001-promoted-demo/spec.md");
        assert!(promoted.is_file());
        let body = fs::read_to_string(&promoted).unwrap();
        assert!(body.contains("id: \"001-promoted-demo\""), "id not rewritten:\n{body}");
        assert!(body.contains("slug: 001-promoted-demo"), "slug not rewritten");
        assert_eq!(outcome.promoted_relpath, "specs/001-promoted-demo/spec.md");

        // FR-008: the project's registry was recompiled and now exists.
        assert!(outcome.compile.ran);
        assert!(
            project.path().join(".derived/spec-registry/registry.json").is_file(),
            "compile step should have written the project registry; compile detail: {}",
            outcome.compile.detail
        );
        // SC-005: the promoted spec is valid (no error-severity violations;
        // V-013 warnings for the capability required-field set are allowed
        // per SC-002 "warning severity or better").
        assert!(outcome.compile.ok, "promoted spec failed validation: {}", outcome.compile.detail);
        // Coupling gate skipped (no binary supplied).
        assert!(!outcome.coupling.ran);
    }

    #[test]
    fn refuses_to_clobber_existing_target() {
        let project = tempfile::tempdir().unwrap();
        fixture_project(project.path());
        let (run_root, staged_slug) = stage_a_spec(project.path());
        fs::create_dir_all(project.path().join("specs/001-taken")).unwrap();

        let req = PromotionRequest {
            project_root: project.path().to_path_buf(),
            run_root,
            staged_slug,
            target_slug: "001-taken".to_string(),
        };
        let err = promote_spec(&req, None).unwrap_err();
        assert!(matches!(err, PipelineError::Promotion(_)));
    }

    #[test]
    fn missing_staged_spec_errors() {
        let project = tempfile::tempdir().unwrap();
        fixture_project(project.path());
        let (run_root, _slug) = stage_a_spec(project.path());
        let req = PromotionRequest {
            project_root: project.path().to_path_buf(),
            run_root,
            staged_slug: "does-not-exist".to_string(),
            target_slug: "002-x".to_string(),
        };
        assert!(matches!(promote_spec(&req, None).unwrap_err(), PipelineError::Promotion(_)));
    }
}
