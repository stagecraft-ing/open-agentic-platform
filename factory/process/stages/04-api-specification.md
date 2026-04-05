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
gate: S4-001 through S4-005 (from verification contract)
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
   - Response shape (single, list, paginated, empty)
   - Business rules enforced
   - Use case and test case traceability

3. **System endpoints** — Health checks, CSRF tokens, auth status — non-resource endpoints.

4. **Dual-stack routing** (if dual variant):
   - Determine which operations belong on the public stack (citizen-facing, proxied)
   - Determine which operations belong on the internal stack (staff-facing, direct DB)
   - Operations accessible to both audiences appear on internal; public proxies to them

## Output Format

Populate the `api` section of `.factory/build-spec.yaml`. Also populate `auth`, `project` (with variant), and `business_rules` sections at this stage since we now have all the information.

## What NOT to do

- Do not generate code, OpenAPI specs, or framework-specific route definitions. The adapter does that.
- Do not choose response formats (JSON vs XML). That's adapter-specific.
- Do not specify middleware. That's adapter-specific.

## Gate

S4-001 through S4-005 must pass before Stage 5 begins.
