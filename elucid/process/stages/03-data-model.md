---
id: data-model
name: Data Model Design
sequence: 3
inputs:
  - requirements/entity-model.json
  - requirements/business-rules.json
  - requirements/audiences.json
outputs:
  - requirements/data-model.json (normalized entity model with constraints)
gate: S3-001 through S3-003 (from verification contract)
agent_role: Data Architect
---

# Stage 3: Data Model Design

Refine the entity model from Stage 1 into a normalized, constraint-complete data model ready for adapter consumption.

## Agent Role

You are a Data Architect. Using the entity model and business rules from Stage 1, produce a refined data model:

1. **Normalize entities** — Apply at least 3NF. Split composite entities into separate tables where appropriate. Identify junction tables for many-to-many relationships.

2. **Define all fields** — For each entity: field name, type (from the Build Spec type enum: string, text, integer, decimal, boolean, uuid, date, datetime, timestamp, enum, json, reference), constraints (required, unique, default), and max length / precision where applicable.

3. **Define relationships** — Every reference field must specify: target entity, target field, and on-delete behavior (cascade, restrict, set-null, no-action).

4. **Map business rules** — Link each BR to the entities it constrains. For state machines: define all states, transitions, and terminal states. For computations: define formula, inputs, output field.

5. **Define indexes** — For fields commonly used in queries (foreign keys, status fields, search fields).

6. **Check constraints** — Express entity-level constraints as named rules (e.g., "fiscal year format YYYY-YYYY").

## Output Format

Write `requirements/data-model.json` following the `data_model` section of the Build Specification schema. This is the authoritative entity model consumed by Stage 4 (API spec) and by the adapter's data scaffolder.

## What NOT to do

- Do not generate DDL/SQL. The adapter's data scaffolder does that using its database dialect.
- Do not choose an ORM or query library. That's adapter-specific.
- Do not create API schemas. Stage 4 derives those from this model.

## Gate

S3-001 through S3-003 must pass before Stage 4 begins.
