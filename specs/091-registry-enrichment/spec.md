---
id: "091-registry-enrichment"
title: "Spec Registry Enrichment"
status: approved
implementation: complete
owner: bart
created: "2026-04-11"
risk: low
depends_on:
  - "039"
summary: >
  Promote depends_on, owner, and risk from extra frontmatter to first-class spec
  compiler fields. Validate risk enum values. Update featuregraph to read enriched
  fields from the compiled registry, enabling downstream spec-driven gating.
code_aliases: ["REGISTRY_ENRICHMENT"]
implements:
  - path: tools/spec-compiler
  - path: crates/featuregraph
---

# 091 — Spec Registry Enrichment

Parent plan: [089 Governed Convergence Plan](../089-governed-convergence-plan/spec.md)

## Problem

The spec registry is the compiled truth, but it only carries `id`, `title`, `status`,
`created`, `summary`, `authors`, `kind`, `feature_branch`, and `code_aliases` as
first-class fields. Fields like `depends_on`, `owner`, and `risk` are either lost or
buried in `extraFrontmatter` where no consumer reads them.

Without enriched registry data, downstream consumers (featuregraph, policy-kernel,
preflight) cannot derive execution boundaries from specs.

## Implementation Slices

### 1. Promote depends_on to first-class compiler field (1 day)
- Add `depends_on` to `KNOWN_KEYS` in spec compiler
- Emit as `dependsOn: Vec<String>` in registry JSON features array
- Files: `tools/spec-compiler/src/lib.rs`

### 2. Promote owner to first-class compiler field (0.5 day)
- Add `owner` to `KNOWN_KEYS`
- Emit as `owner: Option<String>` in registry JSON
- Files: `tools/spec-compiler/src/lib.rs`

### 3. Add risk as a new frontmatter field (0.5 day)
- Define risk levels: `low`, `medium`, `high`, `critical`
- Add to `KNOWN_KEYS`, validate enum values, emit in registry
- Files: `tools/spec-compiler/src/lib.rs`

### 4. Featuregraph reads enriched fields (1 day)
- Update `RegistryFeatureRecord` to deserialize `dependsOn`, `owner`, `risk`
- Populate `FeatureNode.depends_on`, `.owner`, `.governance` from registry
- Files: `crates/featuregraph/src/registry_source.rs`, `crates/featuregraph/src/scanner.rs`

### 5. Recompile registry and validate (0.5 day)
- Run `spec-compiler compile` to regenerate `build/spec-registry/registry.json`
- Verify enriched fields appear for specs that use them (087, 089, etc.)
- Add `risk` frontmatter to 5+ specs as initial population

## Acceptance Criteria

- SC-091-1: `registry.json` features carry `dependsOn`, `owner`, `risk` when present in frontmatter
- SC-091-2: `featuregraph` loads dependency graph from registry (non-empty `depends_on` on 087)
- SC-091-3: Spec compiler rejects invalid `risk` values (not in `[low, medium, high, critical]`)

## Dependencies

| Spec | Relationship |
|------|-------------|
| 000-spec-system | Spec compiler foundation |
| 001-spec-frontmatter | Frontmatter schema |
| 034-featuregraph-registry-scanner-fix | Feature graph scanning |
| 039-spec-compiler-known-keys | KNOWN_KEYS system |
