# Data model: Compiled spec registry (MVP)

**Feature**: `000-bootstrap-spec-system`

## RegistryDocument (root)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `specVersion` | string | yes | Format version for `registry.schema.json`. |
| `build` | BuildInfo | yes | Provenance of this JSON artifact. |
| `features` | FeatureRecord[] | yes | All compiled features from markdown inputs. |
| `validation` | ValidationSummary | yes | Aggregate validation result. |

## BuildInfo

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `compilerId` | string | yes | Stable identifier of the compiler implementation (e.g. `open-agentic-spec-compiler`). |
| `compilerVersion` | string | yes | Semantic version of the compiler. |
| `builtAt` | string (ISO 8601) | yes | UTC timestamp of emission. |
| `inputRoot` | string | yes | Repository-relative root scanned (e.g. `.`). |
| `contentHash` | string | yes | SHA-256 hex per `research.md` D2. |

## FeatureRecord

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | yes | Matches `specs/<id>/` and frontmatter `id`. |
| `title` | string | yes | From frontmatter. |
| `status` | string | yes | From frontmatter. |
| `specPath` | string | yes | Relative path to authoritative `spec.md`. |
| `created` | string | yes | From frontmatter ISO date. |
| `sectionHeadings` | string[] | yes | Level-1 and level-2 headings in document order (for TOC / navigation); exact depth fixed in compiler. |
| `frontmatter` | object | yes | Parsed frontmatter as a JSON object (keys sorted on emit). |

## ValidationSummary

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `passed` | boolean | yes | True iff zero violations with severity `error`. |
| `violations` | Violation[] | yes | Stable list, sorted by `code` then `message`. |

## Violation

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `code` | string | yes | e.g. `V-001`, `V-004`. |
| `severity` | string | yes | `error` \| `warning` (MVP may emit only `error`). |
| `message` | string | yes | Human-readable description. |
| `path` | string | no | File path if applicable. |

## Out of scope for MVP JSON payload

- Full markdown body text (may be added in a later registry version).
- Graph edges between features (future **featuregraph** feature).
- xray scan results or axiomregent policy payloads.
