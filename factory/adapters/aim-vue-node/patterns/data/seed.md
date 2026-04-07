# Seed Pattern

Database seeding populates reference/lookup tables and development fixtures.
Two separate files, run in order after migrations.

## Convention

- Reference data: `database/seeds/reference-data.sql`
- Dev fixtures: `database/seeds/dev-fixtures.sql`
- Runner script: `scripts/run-seeds.js` (checks NODE_ENV)
- Idempotent — use `INSERT ... ON CONFLICT DO NOTHING`
- Wrap each entity block in a transaction

## Template — Reference Data

```sql
-- Seed: {EntityName}
-- Source: Build Spec data_model.entities["{EntityName}"].hydration
BEGIN;

INSERT INTO app.{table_name} ({columns})
VALUES
  ({row_1_values}),
  ({row_2_values}),
  ({row_N_values})
ON CONFLICT ({unique_column}) DO NOTHING;

COMMIT;
```

## Template — Dev Fixtures

```sql
-- Fixture: {EntityName} — {profile_name}
-- Profile: {profile_description}
BEGIN;

INSERT INTO app.{table_name} ({columns})
VALUES
  ('{deterministic_uuid}', {field_values})
ON CONFLICT ({pk_column}) DO NOTHING;

COMMIT;
```

## Template — Seed Runner Script

```js
// scripts/run-seeds.js
import { execSync } from 'node:child_process';

if (process.env.NODE_ENV === 'production') {
  console.error('ERROR: Seed scripts cannot run in production.');
  process.exit(1);
}

const dbUrl = process.env.DATABASE_URL;
if (!dbUrl) { console.error('DATABASE_URL not set'); process.exit(1); }

// Migrations first, then seeds
execSync(`psql "${dbUrl}" --set ON_ERROR_STOP=1 -f database/seeds/reference-data.sql`, { stdio: 'inherit' });

if (process.env.LOAD_FIXTURES !== 'false') {
  execSync(`psql "${dbUrl}" --set ON_ERROR_STOP=1 -f database/seeds/dev-fixtures.sql`, { stdio: 'inherit' });
}

console.log('Seed complete.');
```

## Rules

1. `ON CONFLICT DO NOTHING` for idempotency.
2. Insert in FK dependency order (parent tables first).
3. One `BEGIN/COMMIT` block per entity.
4. Dev fixtures use deterministic UUIDs (`00000000-0000-0000-0000-00000000NNNN`).
5. Dev fixture user references MUST match mock auth driver user IDs.
6. No `random()`, `gen_random_uuid()`, or `now()` in VALUES — all literal.
7. Comments before each entity block: entity name, source, profile name.
