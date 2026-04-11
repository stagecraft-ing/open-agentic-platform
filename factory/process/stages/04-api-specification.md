---
id: api-specification
name: API Specification
sequence: 4
inputs:
  - requirements/data-model.json
  - requirements/use-cases.json
  - requirements/business-rules.json
  - requirements/audiences.json
  - requirements/sitemap.json
  - requirements/variant.json
outputs:
  - .factory/build-spec.yaml (api section populated)
gate: S4-001 through S4-008 (from verification contract)
agent_role: API Architect
---

# Stage 4: API Specification

Design the complete API surface as a tech-agnostic resource/operation model.

## Agent Role

You are an API Architect. Using the data model, use cases, and audiences from previous stages, define every API resource and operation:

1. **Resources** — Group operations by entity. Each resource: name (kebab-case), primary entity, parent resource (for nested routes).

2. **Operations** — For each use case, define the API operations that implement it:
   - ID (unique, e.g., "list-funding-requests")
   - HTTP method (GET, POST, PUT, PATCH, DELETE)
   - Path (relative to resource, e.g., "/", "/:id", "/:id/transitions")
   - Audience (which user groups can call this)
   - Auth requirement (required, optional, service-only, public)
   - Required roles (empty = any authenticated user)
   - Stack assignment (for dual variant: public, internal, or both)
   - Request shape (params, query, body with entity field references)
   - Response shape (single, list, paginated, empty, binary)
   - Business rules enforced
   - Use case and test case traceability

3. **System endpoints** — Health checks, CSRF tokens, auth status — non-resource endpoints.

4. **Dual-stack routing** (if dual variant):
   - Determine which operations belong on the public stack (citizen-facing, proxied)
   - Determine which operations belong on the internal stack (staff-facing, direct DB)
   - Operations accessible to both audiences appear on internal; public proxies to them

## Output Format

Populate the `api` section of `.factory/build-spec.yaml`. Also populate `auth`, `project` (with variant), and `business_rules` sections at this stage since we now have all the information.

## Cross-Stage Consistency Checks

Before completing this stage, verify the Build Specification is consistent with the data model produced in Stage 3:

### Field-to-Column Traceability

Every field name referenced in the API specification (request shapes, response shapes, filter parameters) MUST trace to a column in the data model's entity definitions. Specifically:

- **Entity field names** in operation request/response shapes must correspond to columns defined in `requirements/entity-model.json`. A field called `status` when the data model defines `application_status` is a defect — the adapter will generate SQL with the wrong column name.
- **Naming convention compliance** — if camelCase is used in the API spec, there must be an unambiguous mapping to the snake_case data model columns (e.g., `applicationStatus` → `application_status`). Shortened or renamed fields (`status` for `application_status`) are not acceptable.

### Enum Value Alignment

Every enumerated field in the API specification (fields with a fixed set of allowed values) must have values that exactly match the constraint definitions in the data model:

- If the data model defines `CHECK (status IN ('draft', 'submitted', 'approved'))`, the Build Spec's allowed values for that field must be exactly `['draft', 'submitted', 'approved']` — no more, no less.
- Enum values derived from Stage 1 business requirements must be reconciled against Stage 3's data model constraints. The data model is authoritative when they conflict.

### Response Shape Consistency

Pagination and response envelope patterns must be consistent:

- If the spec defines paginated responses, every list operation must use the same envelope shape (e.g., `{ data: T[], total: number }` — not some using `items` and others using `data`).

## What NOT to do

- Do not generate code, OpenAPI specs, or framework-specific route definitions. The adapter does that.
- Do not choose response formats (JSON vs XML). That's adapter-specific.
- Do not specify middleware. That's adapter-specific.

## Gate

S4-001 through S4-008 must pass before Stage 5 begins.

- S4-001 through S4-005: existing specification completeness checks
- **S4-006**: Field-to-column traceability — every API field traces to a data model column
- **S4-007**: Enum value alignment — every enumerated API field matches data model constraints exactly
- **S4-008**: Response shape consistency — pagination envelopes are uniform across operations
