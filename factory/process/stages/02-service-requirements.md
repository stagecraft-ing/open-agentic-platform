---
id: service-requirements
name: Service Requirements
sequence: 2
inputs:
  - requirements/brd.md
  - requirements/use-cases.json
  - requirements/entity-model.json
outputs:
  - requirements/service-description.json
  - requirements/audiences.json (audience definitions with roles)
  - requirements/journeys/{audience-slug}.json (one journey map per audience)
  - requirements/future-state.json
  - requirements/sitemap.json (page inventory with view types)
  - requirements/variant.json (derived variant + rationale)
gate: S2-001 through S2-005 (from verification contract)
agent_role: Service Designer
phase_model:
  phase_a:
    name: "Foundation"
    produces: ["service-description.json", "audiences.json"]
  phase_b:
    name: "Journey Maps"
    produces: ["journeys/{audience-slug}.json (one per audience)"]
    gate: "Phase-B-complete: every audience in audiences.json has a journey map on disk"
  phase_c:
    name: "Synthesis"
    depends_on: "phase_b gate"
    produces: ["future-state.json", "sitemap.json", "variant.json"]
---

# Stage 2: Service Requirements

Derive the service shape from business requirements: who uses it, how they interact with it, and what pages they need. Stage 2 runs in **three phases** with a hard gate between Phase B and Phase C.

## Why Phases?

Synthesis (Phase C — future-state and sitemap) depends on having every audience journey mapped. Earlier versions of this stage collapsed all five sub-agents into one linear flow; that let the synthesis agent proceed before all journey maps were written, producing sitemaps that silently dropped under-specified audiences. The Phase B→C gate prevents that regression.

## Phase A — Foundation

Produce two artefacts:

1. **`requirements/service-description.json`** — the service's GoA context:
   - Ministry
   - Program area
   - Service name and summary
   - Support channels (phone, email, in-person hours, accessibility)
   - Publicly-stated service hours (if any)

2. **`requirements/audiences.json`** — distinct user groups. For each audience:
   - Stable slug (used as the journey-map filename)
   - Name and human-readable description
   - Authentication method (abstract: `saml`, `oidc`, `api-key`, `mock`)
   - Auth provider identifier (abstract: `alberta-ca-account`, `entra-id`, `custom`)
   - Roles with permission scopes

Audiences must partition user space — an individual user cannot simultaneously belong to two audiences with different auth methods. If that would be necessary, split the workflow across two signed-in audiences (e.g., `citizen` vs. `staff`) and use role permissions within an audience for finer distinctions.

**Phase A gate:** S2-001 (service-description schema valid) and S2-002 (audiences schema valid + no duplicate slugs).

## Phase B — Journey Maps

For **each audience** in `audiences.json`, produce **one** journey map at `requirements/journeys/{audience-slug}.json`:

```json
{
  "audience": "citizen",
  "journeys": [
    {
      "name": "Submit Funding Application",
      "steps": [
        { "action": "Sign in", "page_hint": "login", "use_cases": [] },
        { "action": "View dashboard", "page_hint": "dashboard", "use_cases": [] },
        { "action": "Start new application", "page_hint": "application-form", "use_cases": [] },
        { "action": "Submit application", "page_hint": "application-form", "use_cases": ["UC-001"] }
      ]
    }
  ]
}
```

**Write each journey map to disk as it is produced.** Do not hold them in memory across the batch — the pipeline orchestrator's context recovery protocol depends on disk-resident state.

**Phase B gate — enforced mechanically, not on trust:**

The verification harness walks `requirements/journeys/` and asserts that, for every `audience.slug` in `audiences.json`, the file `requirements/journeys/{slug}.json` exists and parses as the journey schema. If any audience is missing its journey map, Phase C MUST NOT begin. This is verification check **S2-003** and it is a hard block — there is no retry-without-all-journeys path.

## Phase C — Synthesis

Once the Phase B gate passes, produce:

1. **`requirements/future-state.json`** — consolidated digital transformation opportunities derived from journey pain points across all audiences. Not a wish list; only includes opportunities that are (a) reachable from journey steps and (b) traceable to at least one business rule or use case.

2. **`requirements/sitemap.json`** — every page the application needs:
   ```json
   {
     "pages": [
       {
         "id": "dashboard",
         "title": "My Applications",
         "path": "/dashboard",
         "page_type": "dashboard",
         "audience": "citizen",
         "view_type": "public-authenticated",
         "requires_auth": true,
         "data_sources": ["list-funding-requests"],
         "linked_journey_steps": [{"audience": "citizen", "journey": "Submit Funding Application", "step_index": 1}]
       }
     ]
   }
   ```
   Page IDs are stable and used for cross-referencing in Stages 4 and 5. `page_type` is drawn from the canonical catalog (landing, dashboard, list, detail, form, content, help, profile, login, wizard, wizard-step-selection, wizard-step-review, report-results, section-hub, settings-form, user-management, permission-editor, contact, directory, information, start, public-form-step, public-form-review, public-form-confirmation).

3. **`requirements/variant.json`** — deployment topology derivation:
   ```json
   {
     "variant": "dual",
     "rationale": "Both public (citizen) and private-authenticated (staff) view types present",
     "surfaces": {
       "public-site": ["citizen"],
       "staff-portal": ["staff"]
     }
   }
   ```
   Derivation rule:
   - All pages `public` or `public-authenticated` → `single-public`
   - All pages `private-authenticated` → `single-internal`
   - Both present → `dual`

**Phase C gate:** S2-004 (sitemap schema valid + every audience has at least one page) and S2-005 (variant derivation matches sitemap view types).

## Capability Validation

After `variant.json` is written, cross-check the adapter manifest:

- If `variant` = `dual` but the adapter declares `dual_stack: false` → STOP.
- If any audience's auth method is not in the adapter's `supported_auth` → STOP.

Record the validation result in pipeline state. The Pipeline Orchestrator enforces the halt.

## What NOT to Do

- Do **not** design API endpoints. Stage 4 owns that.
- Do **not** choose components or layouts. The adapter does that.
- Do **not** collapse Phase B into Phase A. The gate exists for a reason.
- Do **not** skip the capability check. A `dual` variant on a single-stack adapter is a factory failure, not a runtime problem to discover later.

## Page-Type Catalog Reference

Valid `page_type` values (closed set):

| Public                                                | Authenticated                                                          |
| ----------------------------------------------------- | ---------------------------------------------------------------------- |
| `landing`, `information`, `start`, `contact`,         | `dashboard`, `wizard`, `wizard-step-selection`, `wizard-step-review`,  |
| `directory`, `public-form-step`,                      | `list`, `detail`, `form`, `report-results`,                            |
| `public-form-review`, `public-form-confirmation`      | `section-hub`, `settings-form`, `user-management`, `permission-editor` |

Three auxiliary types apply to both view modes: `content`, `help`, `profile`, `login`.

If a page does not fit any of these, introduce a new type only through an explicit update to this catalog and the sitemap schema — do not silently extend.
