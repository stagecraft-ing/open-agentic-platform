# Data model: Spec compiler MVP (implementation view)

**Feature**: `001-spec-compiler-mvp`

This feature **consumes** the registry data model defined in Feature **000** (`specs/000-bootstrap-spec-system/data-model.md`). No parallel schema.

## Internal types (suggested, non-normative)

| Internal struct | Maps to |
|-----------------|---------|
| `ParsedSpec` | Raw path + frontmatter map + body + extracted headings |
| `NormalizedFeature` | `FeatureRecord` in `registry.json` |
| `BuildFingerprint` | Inputs contributing to `contentHash` |

## Outputs

| File | Schema |
|------|--------|
| `build/spec-registry/registry.json` | `specs/000-bootstrap-spec-system/contracts/registry.schema.json` |
| `build/spec-registry/build-meta.json` | `specs/000-bootstrap-spec-system/contracts/build-meta.schema.json` |

## Heading extraction

- **MVP:** Collect **level-1** (`#`) and **level-2** (`##`) ATX headings only, in source order, **excluding** the first H1 if it duplicates `title` (implementation choice documented in code comments; must be stable across runs).
