// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/167-born-with-spec-spine-kernel/spec.md

//! Adapter-seeded initial spec drafts (spec 167 §2.1.6, FR-004).
//!
//! When the kernel is emitted, the adapter contributes spec drafts
//! capturing what it scaffolded. The MVP cut emits one canonical
//! `scaffold-claim` spec per adapter run. The spec body uses spec 147's
//! `kind: capability` grammar and spec 154's `establishes:` unit grammar.
//! When the adapter manifest is available, the draft also carries a
//! `references:` edge with `role: knowledge-source` (spec 156 / 161).

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::version::AdapterIdentity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterSeededSpec {
    /// Three-digit id (e.g. `001`). The tenant numbers their own corpus
    /// starting at 001; spec 000 is OAP's kernel bootstrap and is
    /// reserved.
    pub spec_id: String,
    /// Slug used in the directory name (`specs/<id>-<slug>/`).
    pub slug: String,
    /// Full markdown body, including YAML frontmatter.
    pub markdown: String,
}

impl AdapterSeededSpec {
    /// Relative path (under the tenant project root) where the draft
    /// should land: `specs/<id>-<slug>/spec.md`.
    pub fn relative_path(&self) -> String {
        format!("specs/{}-{}/spec.md", self.spec_id, self.slug)
    }
}

/// Build the adapter's `scaffold-claim` spec.
///
/// FR-004: the emitted draft must satisfy spec 147 (`kind:` grammar),
/// spec 154 (`establishes:` unit grammar with concrete units), and
/// — when `adapter_manifest_uri` is provided — spec 161 (`references:`
/// edge with `role: knowledge-source` and the manifest URI).
pub fn build_scaffold_claim_spec(
    adapter: &AdapterIdentity,
    scaffolded_paths: &[String],
    adapter_manifest_uri: Option<&str>,
) -> AdapterSeededSpec {
    let slug = format!("{}-scaffold-claim", adapter.id);
    let spec_id = "001".to_string();
    let today = Utc::now().format("%Y-%m-%d").to_string();

    let mut establishes_block = String::new();
    if scaffolded_paths.is_empty() {
        // Spec 154 still requires at least one unit; use the scaffold
        // root as a directory unit. The tenant refines this on first
        // edit (the spec is a draft).
        establishes_block.push_str("  - unit: { kind: directory, path: ./ }\n");
    } else {
        for path in scaffolded_paths {
            establishes_block.push_str(&format!(
                "  - unit: {{ kind: directory, path: {path} }}\n"
            ));
        }
    }

    let references_block = match adapter_manifest_uri {
        Some(uri) => format!(
            "references:\n  - role: knowledge-source\n    unit: {{ kind: file, path: {uri} }}\n"
        ),
        None => String::new(),
    };

    let body = format!(
        r#"---
id: "{spec_id}-{slug}"
slug: {slug}
title: "Scaffold claim — {adapter_id}"
status: draft
implementation: pending
owner: tenant
created: "{today}"
kind: capability
risk: low
establishes:
{establishes_block}{references_block}summary: >
  Adapter-seeded scaffold claim emitted by OAP's born-with kernel
  (spec 167). Records the paths the {adapter_id} adapter (v{adapter_version})
  brought into existence at project birth. The tenant should refine
  this spec — split per-unit concerns, name capabilities, and grow
  the relationship graph — as the project evolves.
---

# {spec_id}-{slug}

This spec was emitted at project birth by OAP's factory-engine
(spec 167 — born-with spec-spine kernel emission). It is the first
authored truth in this corpus and serves two purposes:

1. **Provenance.** Records the adapter that scaffolded this project,
   the adapter version, and the paths it established.
2. **Discoverability.** Gives the coupling gate (specs 127/130/133)
   a starting authority to attach to the scaffolded paths so the
   tenant's first PR doesn't fail on empty authority.

Tenants are expected to refine this spec on their first
substantive change. See the kernel's `specs/000-bootstrap-spec-system/spec.md`
for the authoring contract.
"#,
        spec_id = spec_id,
        slug = slug,
        adapter_id = adapter.id,
        adapter_version = adapter.version,
        today = today,
        establishes_block = establishes_block,
        references_block = references_block,
    );

    AdapterSeededSpec { spec_id, slug, markdown: body }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter() -> AdapterIdentity {
        AdapterIdentity {
            id: "aim-vue-node".into(),
            version: "0.1.0".into(),
            manifest_hash: "deadbeef".into(),
        }
    }

    #[test]
    fn relative_path_is_specs_dir() {
        let spec = build_scaffold_claim_spec(&adapter(), &["apps/".into()], None);
        assert_eq!(
            spec.relative_path(),
            "specs/001-aim-vue-node-scaffold-claim/spec.md"
        );
    }

    #[test]
    fn body_includes_required_frontmatter_keys() {
        let spec = build_scaffold_claim_spec(
            &adapter(),
            &["apps/".into(), "packages/".into()],
            Some("file://adapter-scopes.json#aim-vue-node"),
        );
        let md = &spec.markdown;
        // Spec 147 — kind grammar.
        assert!(md.contains("kind: capability"));
        // Spec 154 — unit grammar with concrete units.
        assert!(md.contains("kind: directory, path: apps/"));
        assert!(md.contains("kind: directory, path: packages/"));
        // Spec 161 — provenance edge when manifest URI is supplied.
        assert!(md.contains("role: knowledge-source"));
        assert!(md.contains("adapter-scopes.json"));
        // Spec 000 lifecycle fields.
        assert!(md.contains("status: draft"));
        assert!(md.contains("implementation: pending"));
    }

    #[test]
    fn body_omits_references_block_when_uri_absent() {
        let spec = build_scaffold_claim_spec(&adapter(), &["apps/".into()], None);
        assert!(!spec.markdown.contains("references:"));
        assert!(!spec.markdown.contains("knowledge-source"));
    }

    #[test]
    fn empty_scaffold_paths_fall_back_to_root_unit() {
        let spec = build_scaffold_claim_spec(&adapter(), &[], None);
        // Spec 154 requires at least one unit. Empty input must not
        // produce a spec that violates the unit grammar.
        assert!(spec.markdown.contains("kind: directory, path: ./"));
    }
}
