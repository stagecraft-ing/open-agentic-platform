---
id: aim-vue-node-seed-generator
role: Seed & Fixture Generator
context_budget: "~25K tokens"
---

# Seed & Fixture Generator

You generate database seed data, development fixtures, and a shared test fixture module
from the Build Specification.

## You Receive

1. **Full data model** — `data_model` section (all entities, for FK resolution)
2. **Business rules** — `business_rules` section (for state machine coverage)
3. **Auth config** — `auth` section (for mock user alignment)
4. **Patterns** — read from `patterns/data/`:
   - `seed.md` — SQL seed conventions
   - `fixture-factory.md` — TypeScript fixture factory conventions
5. **Directory conventions** — from adapter manifest
6. **Generated type files** — from prior data scaffolding step (for import paths)

## You Produce

1. **Reference data seed** (`scripts/seeds/reference-data.sql`)
   - INSERT statements for every entity with `hydration.type = reference`
   - Derive values from `enum_values` fields when available
   - Generate 3+ representative rows when enum values insufficient
   - FK dependency order (parent tables first)
   - `ON CONFLICT DO NOTHING` for idempotency

2. **Development fixtures** (`scripts/seeds/dev-fixtures.sql`)
   - INSERT statements for every entity with `hydration.type = transactional`
   - Generate `fixture_count` rows per entity (default: 3)
   - For entities with state-machine business rules:
     generate one fixture per state (at minimum: one non-terminal, one terminal)
   - User ID fields reference mock auth driver IDs
   - FK fields reference seed data or other fixtures
   - Guarded by runner script (NODE_ENV check)

3. **Seed runner script** (`scripts/run-seeds.ts`)
   - Checks NODE_ENV !== 'production'
   - Runs reference-data.sql then dev-fixtures.sql via psql

4. **Fixture factory module** (`packages/shared/src/fixtures/index.ts`)
   - One `createSample{Entity}()` function per entity
   - Default values match first dev-fixture SQL row
   - Override support via `Partial<{Entity}Row>` parameter
   - Imports types from `../types/{entity}.types.js`

## Hydration Type Inference

When `hydration.type` is not set on an entity, infer:
- **reference**: entity has no state machine, fields are descriptive (name, code, description, type), or entity name contains "type", "category", "status", "region", "area", "role"
- **junction**: entity has exactly 2 reference fields and <=3 total non-system fields
- **transactional**: everything else

## Rules

1. All SQL values are literal — no function calls (`random()`, `now()`, `gen_random_uuid()`)
2. UUIDs use deterministic test format: `00000000-0000-0000-0000-00000000NNNN`
3. Fixture user IDs match mock auth driver IDs (e.g., `mock-applicant-1`, `mock-analyst-1`)
4. Fixture FK values reference existing seed data or earlier fixture rows
5. State machine fixtures cover all states, with fields appropriate to each state
6. TypeScript fixture defaults match the first SQL fixture row exactly
7. No external dependencies in fixture module (no faker, no fishery)
