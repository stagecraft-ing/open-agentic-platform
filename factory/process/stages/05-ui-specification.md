---
id: ui-specification
name: UI Specification
sequence: 5
inputs:
  - .factory/build-spec.yaml (api section)
  - requirements/sitemap.json
  - requirements/audiences.json
  - requirements/journeys.json
outputs:
  - .factory/build-spec.yaml (ui section populated — Build Spec now complete)
gate: S5-001 through S5-003 (from verification contract)
agent_role: UI Architect
---

# Stage 5: UI Specification

Define every page the application needs, linking each to its data sources and navigation position.

## Agent Role

You are a UI Architect. Using the sitemap, API specification, and journey maps, define every page:

1. **Pages** — For each sitemap entry, produce a page specification:
   - ID, title, URL path
   - Page type (landing, dashboard, list, detail, form, content, help, profile, login, error)
   - Audience and view type
   - Auth requirements and role restrictions
   - Data sources (which API operations this page calls, and when — on-load, on-action, on-submit)
   - For forms: which operation the form submits to
   - Navigation section and order
   - Use case and test case traceability

2. **Navigation structure** — Define nav sections (e.g., "main", "admin") with their audience and labels.

3. **Data source completeness** — Verify every API operation is reachable from at least one page. If an operation has no page, either add a page or flag it as API-only (background/service use).

## Output Format

Populate the `ui` and remaining sections (integrations, notifications, audit, traceability) of `.factory/build-spec.yaml`. After this stage, the Build Specification is **complete**.

## What NOT to do

- Do not choose components, frameworks, or design systems. The adapter does that.
- Do not specify layouts, CSS, or responsive behavior. The adapter handles that via its page-type patterns.
- Page types are abstract categories. "form" means "a page where users input data" — not "a Vue SFC with GoA form components."

## Gate

S5-001 through S5-003 must pass. After this gate, the Build Specification is frozen and handed to the adapter.
