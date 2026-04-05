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
