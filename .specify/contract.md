# `.specify/` contract (open-agentic-platform)

This file is a **short normative summary** of Feature `000-bootstrap-spec-system`. If this file disagrees with `specs/000-bootstrap-spec-system/spec.md`, **the feature spec wins**.

## Authoring

- Human durable truth: **markdown files** (`.md`) only.
- Structured metadata for a document: **YAML inside markdown frontmatter** (`---` blocks) when required by the document grammar.
- **No** standalone `.yaml` / `.yml` files as authoritative inputs (see feature spec invariant **V-004**).

## Machine layer

- Machine durable truth: **JSON only**, emitted by the **spec compiler** into `build/spec-registry/` (default; see feature spec for exact filename).
- Humans do not hand-edit compiler JSON.

## Spec Kit layout

- Feature work lives under `specs/NNN-kebab-case/` with `spec.md`, `plan.md`, `tasks.md`, and optional `contracts/`, `research.md`, `quickstart.md`.
- Workflow scripts live under `.specify/scripts/bash/`; templates under `.specify/templates/`.

## Next step

Read the full contract: `specs/000-bootstrap-spec-system/spec.md`.
