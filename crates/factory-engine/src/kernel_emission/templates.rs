// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/167-born-with-spec-spine-kernel/spec.md

//! Tenant-side gate wiring templates (spec 167 §2.1.5, FR-003 + FR-008).
//!
//! The tenant project is emitted with a GitHub Actions workflow and a
//! Makefile `pr-prep` target that invoke the coupling gate against the
//! tenant's own spec spine. Default platform is GitHub Actions; the
//! template strings here are stable enough that per-adapter overrides
//! (GitLab CI, Azure Pipelines) can be added without restructuring.

use super::KernelEmissionError;

/// Embedded template text (built at compile time via include_str!).
const TENANT_WORKFLOW_TMPL: &str =
    include_str!("../../templates/kernel/tenant-ci.yml.tmpl");
const TENANT_MAKEFILE_TMPL: &str =
    include_str!("../../templates/kernel/tenant.makefile.tmpl");

/// Context for rendering the tenant gate templates.
#[derive(Debug, Clone)]
pub struct TenantGateContext {
    /// Path (relative to the tenant project root) where the tenant-resident
    /// spine binaries live. Default: `tools/spec-spine`.
    pub binaries_dir: String,
    /// Path to the tenant codebase index manifest input. Default:
    /// `.derived/codebase-index/index.json`.
    pub index_path: String,
    /// Path to the spec registry the tenant's coupling gate reads.
    pub registry_path: String,
}

impl Default for TenantGateContext {
    fn default() -> Self {
        Self {
            binaries_dir: "tools/spec-spine".into(),
            index_path: ".derived/codebase-index/index.json".into(),
            registry_path: ".derived/spec-registry/registry.json".into(),
        }
    }
}

/// Render the tenant CI workflow (GitHub Actions). Substitutes
/// `{{binaries_dir}}`, `{{index_path}}`, `{{registry_path}}`.
pub fn render_tenant_workflow(ctx: &TenantGateContext) -> Result<String, KernelEmissionError> {
    substitute(TENANT_WORKFLOW_TMPL, ctx)
}

/// Render the tenant Makefile `pr-prep` target.
pub fn render_tenant_makefile(ctx: &TenantGateContext) -> Result<String, KernelEmissionError> {
    substitute(TENANT_MAKEFILE_TMPL, ctx)
}

fn substitute(template: &str, ctx: &TenantGateContext) -> Result<String, KernelEmissionError> {
    // `@@NAME@@` is the kernel-emission placeholder syntax. Chosen over
    // Jinja/Mustache `{{NAME}}` because the emitted CI workflows contain
    // GitHub Actions expressions (`${{ github.event… }}`) that would
    // otherwise collide with the un-substituted-placeholder check below.
    let rendered = template
        .replace("@@binaries_dir@@", &ctx.binaries_dir)
        .replace("@@index_path@@", &ctx.index_path)
        .replace("@@registry_path@@", &ctx.registry_path);
    if let Some(idx) = rendered.find("@@") {
        let snippet: String = rendered[idx..].chars().take(40).collect();
        return Err(KernelEmissionError::Template(format!(
            "un-substituted placeholder near `{snippet}`"
        )));
    }
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_renders_with_defaults() {
        let ctx = TenantGateContext::default();
        let out = render_tenant_workflow(&ctx).unwrap();
        assert!(out.contains("tools/spec-spine"));
        assert!(out.contains("spec-code-coupling-check"));
        assert!(!out.contains("@@"));
    }

    #[test]
    fn makefile_renders_with_defaults() {
        let ctx = TenantGateContext::default();
        let out = render_tenant_makefile(&ctx).unwrap();
        assert!(out.contains("pr-prep:"));
        assert!(out.contains("tools/spec-spine"));
        assert!(out.contains(".derived/codebase-index/index.json"));
        assert!(out.contains(".derived/spec-registry/registry.json"));
        assert!(!out.contains("@@"));
    }

    #[test]
    fn workflow_honours_binaries_dir_override() {
        let ctx = TenantGateContext {
            binaries_dir: "vendor/spine".into(),
            ..Default::default()
        };
        let out = render_tenant_workflow(&ctx).unwrap();
        assert!(out.contains("vendor/spine"));
        assert!(!out.contains("tools/spec-spine"));
    }

    #[test]
    fn makefile_honours_index_and_registry_overrides() {
        let ctx = TenantGateContext {
            binaries_dir: "vendor/spine".into(),
            index_path: "build/index.json".into(),
            registry_path: "build/registry.json".into(),
        };
        let out = render_tenant_makefile(&ctx).unwrap();
        assert!(out.contains("vendor/spine"));
        assert!(out.contains("build/index.json"));
        assert!(out.contains("build/registry.json"));
    }

    #[test]
    fn unsubstituted_placeholder_is_rejected() {
        // Smuggle the placeholder syntax through a context value to
        // simulate a future template carrying a typo'd `@@foo@@`.
        let ctx = TenantGateContext {
            binaries_dir: "@@lurking_placeholder@@".into(),
            ..Default::default()
        };
        let err = render_tenant_workflow(&ctx).unwrap_err();
        assert!(matches!(err, KernelEmissionError::Template(_)));
    }

    #[test]
    fn workflow_template_does_not_collide_with_github_actions_expressions() {
        // GitHub Actions uses `${{ ... }}` expressions; our substitution
        // syntax `@@NAME@@` must coexist with them.
        let ctx = TenantGateContext::default();
        let out = render_tenant_workflow(&ctx).unwrap();
        assert!(out.contains("${{ github.event.pull_request.base.sha }}"));
    }
}
