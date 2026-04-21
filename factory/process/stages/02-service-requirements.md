---
id: service-requirements
name: Service Requirements
sequence: 2
inputs:
  - requirements/brd.md
  - requirements/use-cases.json
  - requirements/entity-model.json
outputs:
  - requirements/audiences.json (audience definitions with roles)
  - requirements/journeys.json (user journey maps)
  - requirements/sitemap.json (page inventory with view types)
  - requirements/variant.json (derived variant + rationale)
gate: S2-001 through S2-003 (from verification contract)
agent_role: Service Designer
---

# Stage 2: Service Requirements

Derive the service shape from business requirements: who uses it, how they interact with it, and what pages they need.

## Agent Role

You are a Service Designer. Using the BRD and use cases from Stage 1, produce:

1. **Audiences** — Distinct user groups. For each: name, description, authentication method (saml, oidc, api-key, mock), roles with permissions.

2. **Journey Maps** — Per audience, the key workflows as step sequences. Each step: action, page reference, emotional state, pain points.

3. **Sitemap** — Every page the application needs. For each: ID, title, URL path, page type (landing, dashboard, list, detail, form, content, help, profile, login, error), audience, view type (public, public-authenticated, private-authenticated).

4. **Variant Derivation** — Analyze the sitemap to determine deployment topology:
   - Only public/public-authenticated pages → `single-public`
   - Only private-authenticated pages → `single-internal`
   - Both → `dual`

## Work Unit Strategy

Stage 2 outputs cluster into three phases with an enforced dependency gate between Phase B and Phase C.

### Phase A — Foundation (1 batch)

Produce `audiences.json` (every distinct user group with roles and auth method). Write to disk before Phase B begins.

### Phase B — Journey Maps (one batch per audience)

For each audience identified in Phase A, produce the journey entry in `journeys.json`. Batch size is one audience at a time. After each batch, append to `journeys.json` on disk and update `.factory/stage-progress.json`. Release the completed journey's content from active context before starting the next batch.

### Phase B → Phase C Dependency Gate

Sitemap and variant derivation are **blocked until every audience in `audiences.json` has a corresponding journey entry on disk in `journeys.json`**. Starting Phase C with incomplete journeys produces an incomplete page inventory.

Before beginning Phase C:
1. Count audiences in `audiences.json`.
2. Count distinct `audience` values in `journeys.json`.
3. If counts do not match, identify the missing audiences and produce their journeys first — do NOT start Phase C.

### Phase C — Synthesis (sequential)

Produce `sitemap.json` by enumerating every page referenced in journeys plus any pages implied by the entity model but not yet in a journey (e.g., admin pages). Derive `variant.json` from the sitemap's `view_type` values. Write each artifact to disk as it is produced.

## Context Budget Awareness

- Write each artifact to disk as it completes (`audiences.json` → per-audience journey append → `sitemap.json` → `variant.json`).
- After writing, release the artifact's content from active context. If Phase C needs journey detail, re-read `journeys.json` from disk.
- Write `.factory/stage-progress.json` after each phase and each Phase B batch. Track a `dependencyGate` object with `totalAudiences`, `journeyMapsWritten`, `journeyMapsRequired` so compaction recovery can tell whether Phase C is unblocked.

## Capability Validation

After variant is determined, check the adapter manifest:
- If `dual` and adapter lacks `dual_stack` capability → STOP, report incompatibility
- If auth method needed and adapter doesn't support it → STOP, report incompatibility

Write validation results to pipeline state.

## What NOT to do

- Do not design API endpoints. Stage 4 does that.
- Do not choose components or layouts. The adapter handles that.
- Page types are abstract categories, not framework-specific patterns.

## Gate

S2-001 through S2-003 must pass before Stage 3 begins.
