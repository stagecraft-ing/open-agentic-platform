# Encore Migration Pattern

## Convention
Each service owns its database. Migrations are numbered SQL files in `api/{service}/migrations/`. Encore auto-runs migrations on startup.

## Template
```sql
-- api/{service}/migrations/{number}_{name}.sql

CREATE TYPE {entity_snake}_status AS ENUM ('{value1}', '{value2}');

CREATE TABLE {entity_snake} (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    {field_snake} TEXT NOT NULL,
    {ref_snake} UUID NOT NULL REFERENCES {ref_table}(id),
    status {entity_snake}_status DEFAULT '{default}',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_{entity_snake}_{ref_snake} ON {entity_snake}({ref_snake});
```

## Database Declaration
```typescript
// api/{service}/encore.service.ts
import { SQLDatabase } from "encore.dev/storage/sqldb";
export const db = new SQLDatabase("{service}", {
  migrations: "./migrations",
});
```

## Rules
1. One database per service — services don't share databases
2. Migrations are sequential: `1_create_users.sql`, `2_add_sessions.sql`
3. Encore manages migration execution — no separate migration runner
4. Use `gen_random_uuid()` for UUID defaults (PostgreSQL built-in)
5. Always create indexes on foreign keys
6. Use PostgreSQL enums for status fields
