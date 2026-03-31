# 054 Agent Frontmatter Schema — phased implementation plan

> **Non-authoritative.** Planning scratch for agent coordination only. Canonical contract remains `specs/054-agent-frontmatter-schema/spec.md`.

## Goal

Standardize YAML frontmatter for agent and skill markdown definitions, ship JSON Schema for editors and CI, implement Tier 1–3 loading and tool allowlist enforcement, quality linter, and migration of in-repo definitions.

## Pre-implementation decisions (A-001 to A-004)

- **A-001 (schema location):** Canonical JSON Schemas live at repository root `schemas/agent-frontmatter.schema.json` and `schemas/skill-frontmatter.schema.json` (matches spec directory diagram). `$id` uses stable `https://open-agentic-platform.dev/schemas/...` URIs for references.
- **A-002 (forwards compatibility):** JSON Schema sets `additionalProperties: true` on the root object so unknown fields validate and can be preserved in parse → serialize round-trips (NF-003, SC-005). Stricter keys are enforced by the Phase 5 linter where needed (kebab-case, description length).
- **A-003 (Tier 1 performance):** NF-001 target is met by parsing only the YAML between first `---` pairs without slurping full file bodies when scanning catalogs; implementation uses bounded reads or line-delimited extraction in Phase 2–3.
- **A-004 (tool allowlist):** FR-007 enforcement lands in Phase 4 by threading parsed `tools` into the governed execution / bridge path aligned with specs 035 and existing agent registry surfaces (042).

## Implementation slices

### Phase 1 — Schema definition (FR-005)

Deliverables:

- `schemas/agent-frontmatter.schema.json` — required: `name`, `description`, `tools`, `model`; optional: `category`, `color`, `displayName`, `version`, `author`, `tags`, `priority`.
- `schemas/skill-frontmatter.schema.json` — required: `name`, `description`; optional tags/author/version as needed.

Validation:

- Schemas are valid JSON and load under draft-07.
- Spot-check: existing `.claude/agents/architect.md` frontmatter validates against agent schema (tools may need normalizing to array of strings if currently comma-separated — migration Phase 6).

### Phase 2 — Parser (FR-001–FR-003, NF-002, NF-003)

Deliverables:

- Shared or package-local parser (TypeScript or Rust consistent with repo consumers) that splits frontmatter + body, parses YAML, preserves unknown keys, surfaces path + line/col on malformed YAML.

Validation:

- Unit tests: valid file, missing delimiter, bad YAML, duplicate keys behavior.

### Phase 3 — Progressive loader (FR-004)

Deliverables:

- Tier 1: metadata-only scan API for directories of agent/skill files.
- Tier 2: load full body for a single id/path on demand.
- Tier 3: hook for on-demand resource paths (stub or config-driven).

Validation:

- NF-001 benchmark or test harness with synthetic 500-file fixture (frontmatter-only path).

### Phase 4 — Tool allowlist enforcement (FR-007)

Deliverables:

- Wire `tools` from parsed agent metadata into runtime/tool dispatch so disallowed tools fail with a clear error (SC-003).

Validation:

- Integration test: agent with `tools: [Read]` cannot invoke `Write`.

### Phase 5 — Quality linter (FR-008, SC-004)

Deliverables:

- CLI or workspace script: required fields, description length ≥ 50, `name` kebab-case, non-empty `tools` when policy requires tools.

Validation:

- Golden tests for pass/fail cases.

### Phase 6 — Migration (SC-001, SC-006)

Deliverables:

- Convert `.claude/agents/*.md` and any skill stubs to compliant frontmatter; fix `tools` formatting if needed.
- Document contributing criteria in a single reference (short section under spec or `execution/` as appropriate).

Validation:

- All definitions pass JSON Schema + linter; SC-002 catalog timing check documented in `execution/verification.md`.
