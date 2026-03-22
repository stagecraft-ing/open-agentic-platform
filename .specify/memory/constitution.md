# open-agentic-platform Constitution

## Core Principles

### I. Markdown-Only Authored Truth

All human-authored durable truth in this repository is expressed as Markdown (`.md`). Optional YAML may appear **only** as **frontmatter inside** a markdown file. Standalone YAML files are not an authoring channel for platform truth.

### II. Compiler-Owned JSON Machine Truth

Machine-consumable registries, indices, and normalized spec metadata live in JSON **produced only** by the designated spec compiler. Hand-edited JSON in compiler output paths is a workflow violation.

### III. Spec-First Development

Features are specified before implementation; implementation is justified by specs under `specs/`. Feature `000-bootstrap-spec-system` is the constitutional baseline for how specs and compiled registries behave.

### IV. Determinism and Validation

The spec compiler must be deterministic for the same committed inputs. Validation rules (including rejection of forbidden standalone YAML) are product requirements, not suggestions.

### V. Legacy Inputs Are Non-Normative

Repositories used for reverse engineering (for example `opc`, `platform`) are **evidence only**. They are not sources of truth for this repository. Provenance must be declared in feature text when legacy concepts inform a design.

## Additional Constraints

- Feature directories use the pattern `specs/NNN-kebab-case/` with matching `id` in frontmatter. Authoritative specs live under **repo-root `specs/`**, not under `.specify/`.
- The compiled registry format is versioned (`specVersion`) and described by JSON Schema in Feature 000. Deterministic output is **`registry.json`**; **`build-meta.json`** holds non-deterministic wall-clock metadata only.

## Development Workflow

- Use Spec Kit commands (`/speckit.specify`, `/speckit.plan`, `/speckit.tasks`, etc.) with feature branches named `NNN-short-name`.
- Read `.specify/contract.md` and Feature 000 before adding new authoring formats or tooling outputs.

## Governance

This constitution is subordinate to **explicit** normative text in `specs/000-bootstrap-spec-system/spec.md` where stricter rules apply. Amendments to constitutional rules require a spec change (new feature or superseding revision) and review of downstream consumers.

**Version**: 1.0.1 | **Ratified**: 2026-03-22 | **Last Amended**: 2026-03-22
