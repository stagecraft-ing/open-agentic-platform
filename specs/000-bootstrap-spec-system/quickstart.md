# Quickstart: Bootstrap spec system

**Feature**: `000-bootstrap-spec-system`

## What exists today

- **Authoritative contract**: [spec.md](./spec.md)
- **Deterministic machine contract**: [contracts/registry.schema.json](./contracts/registry.schema.json)
- **Ephemeral build metadata**: [contracts/build-meta.schema.json](./contracts/build-meta.schema.json)
- **Data shapes**: [data-model.md](./data-model.md)

The **spec compiler binary** is not yet implemented; this quickstart describes the **intended** validation loop once `tasks.md` is executed.

## Validate emitted JSON (schema only)

When `registry.json` and optionally `build-meta.json` exist under `build/spec-registry/`:

```bash
# Deterministic registry (golden tests use this file only)
npx --yes ajv-cli validate -s specs/000-bootstrap-spec-system/contracts/registry.schema.json -d build/spec-registry/registry.json

# Wall-clock metadata (optional; changes every run)
npx --yes ajv-cli validate -s specs/000-bootstrap-spec-system/contracts/build-meta.schema.json -d build/spec-registry/build-meta.json
```

Adjust command after the compiler implementation chooses its validation stack.

## Author a new feature spec

1. Create `specs/NNN-my-feature/spec.md` where `NNN` is the next three-digit number.
2. Copy structure from `.specify/templates/spec-template.md` and add required **YAML frontmatter** per [spec.md](./spec.md) (“Markdown document grammar”).
3. Ensure `id` in frontmatter matches the directory name exactly.

## Clarify → Plan → Tasks workflow

This repo uses Spec Kit commands under `.cursor/commands/`. After editing `spec.md`, run checklist validation in `checklists/requirements.md`, then proceed `/speckit.clarify` (if needed) and `/speckit.plan` for downstream features.

Feature 000 is **constitutional**: changes that weaken markdown-only or JSON-only rules require explicit supersession in spec text.
