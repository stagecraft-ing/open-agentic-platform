---
id: "054-agent-frontmatter-schema"
title: "Agent Frontmatter Schema"
feature_branch: "054-agent-frontmatter-schema"
status: draft
kind: platform
created: "2026-03-29"
authors:
  - "open-agentic-platform"
language: en
summary: >
  Standard YAML frontmatter schema for agent and skill definitions. Agent
  frontmatter includes name, description (with trigger condition), tools
  (allowed list), model, category, color, and displayName. Skill definitions
  use name, description, and markdown body. Progressive disclosure loads
  metadata eagerly, instructions on activation, and resources on demand.
  Contributing quality criteria ensure consistency across the agent library.
code_aliases:
  - AGENT_FRONTMATTER
  - SKILL_SCHEMA
sources:
  - claude-code-sub-agents
  - skills
  - developer-cc-commands
  - agents
---

# Feature Specification: Agent Frontmatter Schema

## Purpose

Agent and skill definitions across the platform use inconsistent metadata formats. Some agents declare their capabilities in code, others in unstructured markdown headers, and others in JSON configuration files. Multiple consolidation sources — claude-code-sub-agents (agent metadata), skills (skill definitions with markdown bodies), developer-cc-commands (command declarations), agents (agent catalog with categories) — each define agent/skill metadata differently, making it impossible to build reliable tooling for discovery, validation, and progressive loading.

This feature establishes a standard YAML frontmatter schema for both agent definitions and skill definitions, enabling uniform parsing, validation, catalog generation, and progressive disclosure that loads only what is needed at each stage of agent/skill lifecycle.

## Scope

### In scope

- **Agent frontmatter schema**: A YAML frontmatter block for agent definition files specifying `name`, `description` (including trigger condition), `tools` (allowed tool list), `model`, `category`, `color`, `displayName`, and optional fields.
- **Skill frontmatter schema**: A YAML frontmatter block for skill definition files specifying `name`, `description`, with the skill body in markdown below the frontmatter.
- **Progressive disclosure**: A three-tier loading strategy where metadata is always loaded (for catalogs and discovery), instructions are loaded on activation (when an agent or skill is invoked), and resources are loaded on demand (when specific tools or context is needed).
- **JSON Schema for validation**: Machine-readable JSON Schema files for both agent and skill frontmatter, enabling IDE autocompletion and CI validation.
- **Contributing quality criteria**: Documented standards for agent and skill contributions including required fields, description quality, tool list hygiene, and naming conventions.

### Out of scope

- **Agent runtime behavior**: This feature defines the metadata schema, not how agents execute.
- **Skill execution engine**: How skills are invoked and run is covered by other features.
- **Agent marketplace / distribution**: Publishing agents to a registry or marketplace is a follow-on concern.
- **Visual editor for agent definitions**: No UI for authoring agent frontmatter.

## Requirements

### Functional

- **FR-001**: Agent definition files use YAML frontmatter (delimited by `---`) with the following required fields: `name` (string, unique identifier), `description` (string, including when the agent should be triggered), `tools` (array of allowed tool names), `model` (string, LLM model identifier).
- **FR-002**: Agent definition files support the following optional fields: `category` (string, for catalog grouping), `color` (string, hex color for UI display), `displayName` (string, human-friendly name), `version` (string, semver), `author` (string), `tags` (array of strings), `priority` (number, for trigger ordering).
- **FR-003**: Skill definition files use YAML frontmatter with required fields: `name` (string, unique identifier), `description` (string, what the skill does). The skill body (markdown below the frontmatter) contains the skill's instructions.
- **FR-004**: Progressive disclosure is implemented in three tiers:
  - **Tier 1 (metadata)**: Frontmatter fields are always parsed and available for catalog listing, search, and filtering. This is cheap to load.
  - **Tier 2 (instructions)**: The full markdown body (agent system prompt or skill instructions) is loaded only when the agent or skill is activated.
  - **Tier 3 (resources)**: External resources referenced by the agent (tool schemas, context files, example data) are loaded on demand when the agent requests them during execution.
- **FR-005**: A JSON Schema file is provided for both agent and skill frontmatter, enabling validation in editors and CI pipelines.
- **FR-006**: `description` fields for agents must include a trigger condition clause (e.g., "Activate when the user asks about database migrations") so that automated routing can match user intents to agents.
- **FR-007**: The `tools` array in agent frontmatter is an allowlist; agents may only invoke tools listed in their frontmatter. The runtime enforces this constraint.
- **FR-008**: Contributing quality criteria are documented and enforced via a linter: required fields present, `description` minimum length (50 characters), `name` follows kebab-case convention, `tools` list is non-empty for agents that use tools.

### Non-functional

- **NF-001**: Tier 1 metadata loading for 500 agents completes in < 200ms (frontmatter-only parsing, no body loading).
- **NF-002**: Frontmatter parsing is resilient to malformed YAML; parse errors include file path and line number.
- **NF-003**: The schema is forwards-compatible: unknown frontmatter fields are preserved (not rejected) to allow experimentation.

## Architecture

### Agent frontmatter example

```markdown
---
name: database-migration-agent
displayName: "Database Migration Agent"
description: >
  Activate when the user asks about database schema changes, migrations, or
  data model updates. Generates migration files, validates schema compatibility,
  and produces rollback scripts.
model: claude-sonnet-4-20250514
category: database
color: "#4A90D9"
tools:
  - Read
  - Write
  - Bash
  - Glob
  - Grep
tags:
  - database
  - migration
  - schema
version: "1.0.0"
author: "open-agentic-platform"
priority: 10
---

# Database Migration Agent

You are a database migration specialist. When the user needs to modify
database schemas, you generate migration files following the project's
migration framework conventions.

## Guidelines

- Always generate both up and down migrations
- Validate foreign key constraints before applying changes
- Produce a rollback script alongside every migration
...
```

### Skill frontmatter example

```markdown
---
name: lint-fix
description: >
  Run project linters and automatically fix all auto-fixable issues.
  Report remaining issues that require manual intervention.
tags:
  - code-quality
  - lint
---

Run the project's configured linter with auto-fix enabled:

1. Detect the project's linter (eslint, biome, ruff, etc.)
2. Run with `--fix` flag
3. Report any remaining issues that could not be auto-fixed
4. Stage the fixed files
```

### Progressive disclosure architecture

```
Agent/Skill catalog request
  |
  v
Tier 1: Parse frontmatter only (all files)
  |
  +---> Return catalog: [{name, description, category, tags}, ...]
  |
  v
User selects / routing activates an agent
  |
  v
Tier 2: Load full markdown body for the selected agent
  |
  +---> System prompt / skill instructions now available
  |
  v
Agent requests a resource during execution
  |
  v
Tier 3: Load referenced resource on demand
  |
  +---> Tool schemas, context files, example data
```

### Directory structure

```
agents/
  database-migration-agent.md
  code-review-agent.md
  security-audit-agent.md
  ...
skills/
  lint-fix.md
  test-runner.md
  dependency-update.md
  ...
schemas/
  agent-frontmatter.schema.json
  skill-frontmatter.schema.json
```

## Implementation approach

1. **Phase 1 -- schema definition**: Define the YAML frontmatter schema for agents and skills. Produce JSON Schema files for both.
2. **Phase 2 -- parser**: Implement a frontmatter parser that extracts Tier 1 metadata without loading the full file body. Handle malformed YAML gracefully with file path and line number in errors.
3. **Phase 3 -- progressive loader**: Implement the three-tier loading strategy with lazy body loading (Tier 2) and on-demand resource loading (Tier 3).
4. **Phase 4 -- tool allowlist enforcement**: Integrate the `tools` frontmatter field with the agent runtime so that agents can only invoke their declared tools.
5. **Phase 5 -- quality linter**: Build a linter that validates agent and skill definitions against the contributing quality criteria (required fields, description quality, naming conventions).
6. **Phase 6 -- migration**: Convert existing agent and skill definitions to the new frontmatter schema. Validate all definitions pass the linter.

## Success criteria

- **SC-001**: All agent definition files in the repository have valid YAML frontmatter that passes JSON Schema validation.
- **SC-002**: The catalog loader parses Tier 1 metadata for all agents in < 200ms without loading markdown bodies.
- **SC-003**: An agent attempting to use a tool not listed in its `tools` frontmatter is blocked by the runtime with a clear error.
- **SC-004**: The quality linter catches missing required fields, descriptions shorter than 50 characters, and non-kebab-case names.
- **SC-005**: Unknown frontmatter fields are preserved through parse-serialize round-trips.
- **SC-006**: Skill definitions with frontmatter and markdown body are correctly parsed into separate metadata and instruction components.

## Dependencies

| Spec | Relationship |
|------|-------------|
| 035-agent-governed-execution | Tool allowlist from frontmatter feeds into governed execution constraints |
| 036-safety-tier-governance | Agent safety classification may be derived from tools listed in frontmatter |
| 042-multi-provider-agent-registry | The `model` field in frontmatter determines which provider handles the agent |

## Risk

- **R-001**: Frontmatter schema may not capture all metadata needs, requiring frequent schema changes. Mitigation: schema is forwards-compatible (unknown fields preserved); additions are non-breaking.
- **R-002**: Progressive disclosure adds complexity to the loading path. Mitigation: Tier 1 is a simple frontmatter parse; Tier 2 and 3 are standard file reads. The complexity is in the orchestration, not the I/O.
- **R-003**: Migrating existing agent definitions to the new schema may surface inconsistencies. Mitigation: the quality linter identifies issues; migration is phased with a grace period for non-compliant definitions.
