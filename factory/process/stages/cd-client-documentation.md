---
id: client-documentation
name: Client Documentation (Optional)
sequence: cd
inserts_after: 2
inputs:
  - requirements/brd.md
  - requirements/use-cases.json
  - requirements/entity-model.json
  - requirements/business-rules.json
  - requirements/audiences.json
  - requirements/journeys.json
  - requirements/sitemap.json
outputs:
  - requirements/client/client-document.json
  - requirements/client/client-document.html
  - requirements/client/client-document.docx (optional, when document-formatting tool is available)
  - requirements/client/project-charter.md
  - requirements/client/project-charter.html
  - requirements/client/project-charter.docx (optional)
  - requirements/client/{service-slug}.slides.json (optional)
gate: CD-001 through CD-003 (from verification contract)
agent_role: Client Documentation Orchestrator
optional: true
scheduling:
  values: ["NOW", "SKIP", "DEFERRED"]
  default: "SKIP"
---

# Stage CD: Client Documentation (Optional)

Generate client-facing documentation artefacts: a service document, an ISO 21500-aligned project charter, and optionally a slide deck. This is **not** on the critical path — the 7-stage build pipeline (pre-flight → adapter-handoff) never depends on its output. It exists to serve stakeholders who need readable summaries of what will be built before code is generated.

## When to Run

Client Documentation runs **after Stage 2 and before Stage 3**, but only if the user explicitly schedules it. There are three legal schedulings:

| Value | Effect |
|-------|--------|
| `NOW` | Run immediately after Stage 2 gate passes. Pipeline pauses at Stage CD handoff. User confirms, then Stage 3 begins. |
| `SKIP` | Skip entirely. Default. Pipeline proceeds directly from Stage 2 to Stage 3. |
| `DEFERRED` | Defer to post-build. The pipeline completes through Stage 6, then optionally runs Stage CD against the frozen Build Spec. |

The scheduling is captured in the Stage 2 Handoff Report. Once recorded in pipeline state it is immutable for the current run — to change it, pause the pipeline and update the state manually.

## What Stage CD Does NOT Do

- **Never blocks the 7-stage build.** If Stage CD fails, the pipeline MUST continue. The Client Documentation outputs are advisory artefacts.
- **Never generates code or API/UI specs.** That is Stage 3–5 work.
- **Never back-pressures earlier stages.** A mid-CD discovery that the BRD is incomplete does not re-open Stage 1. The user fixes the BRD and re-runs Stages 1–2 explicitly.

## Three Phases

The Client Documentation Orchestrator runs three phases in order, each with its own gate:

### Phase 1 — Client Document

Produce `client-document.json` (structured), `client-document.html` (styled), and optionally `client-document.docx`. The document summarises:

- Service description (ministry, program, audiences, support channels)
- Business value and capability inventory
- Per-audience summary of journeys and key pages
- Integration inventory (at an intent level, not API contracts)

### Phase 2 — Project Charter

Produce `project-charter.md`, `.html`, and optionally `.docx`. Aligns with ISO 21500 structure: purpose, objectives, success criteria, scope boundaries, stakeholders, assumptions, constraints, risks, preliminary milestones.

### Phase 3 — Slide Deck (Optional)

If the environment has `python-pptx` available (or an equivalent adapter tool), produce `{service-slug}.slides.json` and `{service-slug}.pptx`. Skip with a recorded reason if the tool is missing.

## Output Discipline

- Write to `requirements/client/` — never mix with the 7-stage outputs in `requirements/`.
- Do not write to `.factory/build-spec.yaml`. Stage CD is a read-only consumer of Stages 1–2.
- Every written file is registered in pipeline state under `stages.client-documentation.artifacts[]`.

## Gate

CD-001 through CD-003 must pass (or be marked `skipped-optional`):

- **CD-001**: Required inputs from Stages 1 and 2 are present and parse.
- **CD-002**: At least Phase 1 (client document) produced a readable JSON + HTML pair.
- **CD-003**: No placeholder tokens (`{{…}}`) remain in any generated `.html` or `.md` output.

## Relationship to ACP Pipeline State

Stage CD is **not** one of the seven stages in `pipeline-state.schema.yaml`. When translating from a legacy goa-software-factory manifest (spec 112 §3.4), the `clientDocumentation` section is intentionally discarded — ACP's pipeline state tracks the build, not the documentation. If a downstream consumer needs to know client docs exist, they read `requirements/client/` directly from the repository.
