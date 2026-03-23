# `.specify/` contract (open-agentic-platform)

This file is a **short normative summary** of Feature `000-bootstrap-spec-system`.

**Precedence:** `specs/000-bootstrap-spec-system/spec.md` > `.specify/memory/constitution.md` > **this file**. The constitution filename does **not** override Feature 000.

## Authoring

- Human durable truth: **markdown files** (`.md`) only.
- Structured metadata for a document: **YAML inside markdown frontmatter** (`---` blocks) when required by the document grammar. Frontmatter is **not** an independent authoring format and must not become an escape hatch for bulk YAML semantics—see the feature spec.
- **No** standalone `.yaml` / `.yml` files as authoritative inputs (see feature spec invariant **V-004**).

## Machine layer

- Machine durable truth: **JSON only**, emitted by the **spec compiler** into `build/spec-registry/`.
- **`registry.json`**: deterministic, canonical for consumers (e.g. future featuregraph).
- **`build-meta.json`**: ephemeral compiler-owned JSON (wall-clock `builtAt`); **not** part of golden-file determinism checks.
- Humans do not hand-edit compiler JSON.

## Spec Kit layout

- **Canonical feature specs** live at **`specs/NNN-kebab-case/`** (repository root), with `spec.md`, `plan.md`, `tasks.md`, and optional `contracts/`, `research.md`, `quickstart.md`. **Not** under `.specify/specifications/`.
- `.specify/` holds **templates, scripts, and constitution**—workflow support, not the authoritative spec library.
- Workflow scripts live under `.specify/scripts/bash/`; templates under `.specify/templates/`.

## Next step

Read the full contract: `specs/000-bootstrap-spec-system/spec.md`.
