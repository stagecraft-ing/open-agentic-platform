# SQLx Migration Pattern

Migrations are sequential SQL files that define database schema. SQLx runs
them automatically on startup or via `sqlx migrate run`.

## Convention

- File: `migrations/{number}_{name}.sql` (e.g., `001_create_users.sql`)
- Pure PostgreSQL DDL — no ORM abstractions
- Sequential numbering ensures dependency order
- Each migration is idempotent where possible

## Template

```sql
-- migrations/{number}_{name}.sql

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TYPE {entity_snake}_status AS ENUM ('{value1}', '{value2}', '{value3}');

CREATE TABLE {entity_snake} (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    {field_snake} TEXT NOT NULL,
    {ref_snake}   UUID NOT NULL REFERENCES {ref_table}(id) ON DELETE CASCADE,
    status      {entity_snake}_status NOT NULL DEFAULT '{default}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_{entity_snake}_{ref_snake} ON {entity_snake}({ref_snake});
CREATE INDEX idx_{entity_snake}_status ON {entity_snake}(status);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_{entity_snake}_updated_at
    BEFORE UPDATE ON {entity_snake}
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
```

## Example

From `migrations/001_create_sites.sql`:

```sql
CREATE TABLE site (
    id   SERIAL PRIMARY KEY,
    url  TEXT NOT NULL UNIQUE
);

CREATE TABLE "check" (
    id         SERIAL PRIMARY KEY,
    site_id    INTEGER NOT NULL REFERENCES site(id) ON DELETE CASCADE,
    up         BOOLEAN NOT NULL,
    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_check_site_id ON "check"(site_id);
```

## Rules

1. Sequential numbering: `001_`, `002_`, etc. — no gaps.
2. Use `gen_random_uuid()` for UUID defaults (PostgreSQL built-in, requires `pgcrypto`).
3. Always create indexes on foreign keys.
4. Use PostgreSQL enums via `CREATE TYPE ... AS ENUM` for status fields.
5. Include `created_at` and `updated_at` on every table.
6. Name constraints explicitly: `idx_`, `fk_`, `chk_` prefixes.
7. Create `update_updated_at` trigger for automatic timestamp updates.
