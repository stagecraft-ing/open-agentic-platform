---
id: ui-specification
name: UI Specification
sequence: 5
inputs:
  - .factory/build-spec.yaml (api section populated by Stage 4)
  - requirements/sitemap.json
  - requirements/audiences.json
  - requirements/journeys/*.json
outputs:
  - .factory/build-spec.yaml (ui section populated — Build Spec now complete)
gate: S5-001 through S5-006 (from verification contract)
agent_role: UI Architect
internal_batching:
  size: "2–3 pages per batch"
  rationale: >
    Keeps the UI Architect's context budget within bounds on large sitemaps
    while letting the verification harness run per-batch schema and
    cross-reference checks.
---

# Stage 5: UI Specification

Define every page the application needs, linking each to its data sources, navigation position, and traceability anchors. This is the last stage before the Build Specification is frozen and handed to the adapter.

## Agent Role

You are a UI Architect. Using the sitemap, API specification, audiences, and journeys, produce the `ui` (and remaining) sections of `.factory/build-spec.yaml`.

## Internal Pipeline

Stage 5 runs a six-phase internal pipeline per batch of pages. A batch is 2–3 pages; the orchestrator walks the sitemap in chunks.

1. **Input validation** — every sitemap page in the batch has a `page_type`, `audience`, `view_type`, and (if interactive) at least one `data_sources` entry.
2. **Page specification** — produce the page object (title, path, page_type, audience, view_type, auth requirements, data sources, form bindings, nav membership, traceability).
3. **Data-source consistency check** — each page's `data_sources[].operation_id` exists in `build_spec.api.resources[].operations[].id`.
4. **Assembly** — splice the batch into `build_spec.ui.pages[]` in sitemap order.
5. **Cross-batch consistency** — no duplicate page IDs across batches; nav sections accumulate deterministically.
6. **UC/TC traceability** — every use case referenced in a journey step appears on at least one page; every page has a declared test-case owner (`tc_ref`).

Verification runs between Phase 4 and Phase 5 of each batch, not just at the end of the stage.

## Output Shape

For each page in `build_spec.ui.pages[]`:

```yaml
- id: dashboard
  title: My Applications
  path: /dashboard
  page_type: dashboard
  audience: citizen
  view_type: public-authenticated
  requires_auth: true
  required_roles: [applicant]
  stack: public                  # derived from view_type + variant
  data_sources:
    - operation_id: list-funding-requests
      trigger: on-load
  nav:
    section: main
    order: 1
  traceability:
    use_cases: [UC-001]
    tc_ref: TC-UI-001
```

Rules:
- `id` is stable and drawn from `sitemap.json` — never renamed in Stage 5.
- `page_type` is drawn from the canonical catalog defined in `02-service-requirements.md`.
- `stack` derives mechanically from `view_type` and `variant`: public pages → `public`, private-authenticated → `internal`. You do not choose.
- `data_sources[].trigger` is one of: `on-load`, `on-action`, `on-submit`.

## Navigation Completeness

After all pages are assembled, produce `build_spec.ui.nav[]`:

```yaml
nav:
  - section: main
    audience: citizen
    label: Main
    pages: [dashboard, application-form]
  - section: admin
    audience: staff
    label: Admin
    pages: [user-management, role-management]
```

Every page with a `nav.section` value appears in exactly one nav entry. Pages without `nav.section` are reachable only via direct link or programmatic navigation and must declare a non-empty `reachable_from` list in their traceability block.

## What NOT to Do

- Do not choose components, frameworks, or design systems. The adapter does that via its `page_types/*.md` patterns.
- Do not specify layouts, CSS, or responsive behaviour. The adapter handles that.
- Do not introduce new `page_type` values. Extend the canonical catalog in `02-service-requirements.md` via an explicit spec change.

## Gate

S5-001 through S5-006 must pass:

- **S5-001**: Every sitemap page has a corresponding Build Spec page with the same id.
- **S5-002**: Every Build Spec page's `data_sources[].operation_id` resolves to an API operation id.
- **S5-003**: No page's `stack` conflicts with any of its `data_sources`' operation `stack` field.
- **S5-004**: Every use case from `use-cases.json` is referenced by at least one page's `traceability.use_cases`.
- **S5-005**: Every page has a `tc_ref` (traceability owner).
- **S5-006**: `build_spec.ui.nav[]` covers every page with a `nav.section` value exactly once.

After the S5 gate passes, **the Build Specification is frozen**. The Scaffolding Orchestrator takes over from here.
