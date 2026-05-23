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
const TENANT_TOOLCHAIN_TMPL: &str =
    include_str!("../../templates/kernel/toolchain.yaml.tmpl");

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

/// Context for rendering the spec 168 toolchain manifest.
#[derive(Debug, Clone)]
pub struct TenantToolchainContext {
    /// Tenant-resident binaries dir; reuses the gate-context default
    /// when constructed via [`TenantToolchainContext::from`].
    pub binaries_dir: String,
    /// Pinned `factory-engine` semver — the version that emitted the
    /// kernel. Recorded into `.kernel-version` as well so the two stay
    /// in lockstep (FR-008).
    pub factory_engine_version: String,
    /// Distribution mode chosen by the adapter (FR-005 of spec 167).
    /// Serialised as `vendor-binaries` or `pinned-toolchain`.
    pub toolchain_mode: String,
}

impl TenantToolchainContext {
    /// Construct a toolchain context inheriting `binaries_dir` from the
    /// existing tenant gate context.
    pub fn new(
        gate_ctx: &TenantGateContext,
        factory_engine_version: impl Into<String>,
        toolchain_mode: impl Into<String>,
    ) -> Self {
        Self {
            binaries_dir: gate_ctx.binaries_dir.clone(),
            factory_engine_version: factory_engine_version.into(),
            toolchain_mode: toolchain_mode.into(),
        }
    }
}

/// Render the tenant `.factory/toolchain.yaml` (spec 168 FR-001 / §2.2).
pub fn render_tenant_toolchain(
    ctx: &TenantToolchainContext,
) -> Result<String, KernelEmissionError> {
    let rendered = TENANT_TOOLCHAIN_TMPL
        .replace("@@binaries_dir@@", &ctx.binaries_dir)
        .replace("@@factory_engine_version@@", &ctx.factory_engine_version)
        .replace("@@toolchain_mode@@", &ctx.toolchain_mode);
    if let Some(idx) = rendered.find("@@") {
        let snippet: String = rendered[idx..].chars().take(40).collect();
        return Err(KernelEmissionError::Template(format!(
            "un-substituted placeholder near `{snippet}`"
        )));
    }
    Ok(rendered)
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

    // ── spec 168 toolchain template ──

    #[test]
    fn toolchain_renders_for_pinned_mode() {
        let gate = TenantGateContext::default();
        let ctx = TenantToolchainContext::new(&gate, "1.4.2", "pinned-toolchain");
        let out = render_tenant_toolchain(&ctx).unwrap();
        assert!(out.contains("mode: \"pinned-toolchain\""));
        assert!(out.contains("version: \"1.4.2\""));
        assert!(out.contains("--tag v1.4.2"));
        assert!(out.contains("invoke: \"tools/spec-spine/build-certificate\""));
        assert!(out.contains("invoke: \"tools/spec-spine/verify-certificate\""));
        assert!(!out.contains("@@"));
    }

    #[test]
    fn toolchain_renders_for_vendor_mode() {
        let gate = TenantGateContext {
            binaries_dir: "vendor/spine".into(),
            ..Default::default()
        };
        let ctx = TenantToolchainContext::new(&gate, "0.9.0", "vendor-binaries");
        let out = render_tenant_toolchain(&ctx).unwrap();
        assert!(out.contains("mode: \"vendor-binaries\""));
        assert!(out.contains("invoke: \"vendor/spine/build-certificate\""));
        assert!(!out.contains("@@"));
    }
}
