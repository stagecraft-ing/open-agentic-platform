# Migration Pattern

## Convention

One SQL migration file per entity: `database/migrations/{timestamp}_{entity_name}.sql`.
Raw DDL, no ORM. Idempotent where possible. All tables live in the `public` schema.

## Template

```sql
-- Migration: Create {entity} table
-- Created: {timestamp}

CREATE TABLE IF NOT EXISTS {table_name} (
    id UUID DEFAULT gen_random_uuid(),
    {-- columns --}
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT pk_{table_name} PRIMARY KEY (id)
);

-- Foreign keys
ALTER TABLE {table_name}
    ADD CONSTRAINT fk_{table_name}_{ref_table}
    FOREIGN KEY ({ref_col}_id) REFERENCES {ref_table}(id)
    ON DELETE {CASCADE|SET NULL|RESTRICT};

-- Indexes
CREATE INDEX ix_{table_name}_{col} ON {table_name} ({col});

-- Enum check constraints
ALTER TABLE {table_name}
    ADD CONSTRAINT ck_{table_name}_{col}
    CHECK ({col} IN ('{value1}', '{value2}'));

-- Unique constraints
ALTER TABLE {table_name}
    ADD CONSTRAINT uq_{table_name}_{col1}_{col2}
    UNIQUE ({col1}, {col2});
```

## Example

```sql
-- Migration: Create funding_request table
-- Created: 2025-03-15T10:00:00Z

CREATE TABLE IF NOT EXISTS funding_request (
    id UUID DEFAULT gen_random_uuid(),
    applicant_id UUID NOT NULL,
    program_id UUID NOT NULL,
    title VARCHAR(255) NOT NULL,
    amount_requested NUMERIC(12,2) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    description TEXT,
    submitted_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT pk_funding_request PRIMARY KEY (id)
);

ALTER TABLE funding_request
    ADD CONSTRAINT fk_funding_request_applicant
    FOREIGN KEY (applicant_id) REFERENCES applicant(id)
    ON DELETE CASCADE;

ALTER TABLE funding_request
    ADD CONSTRAINT fk_funding_request_program
    FOREIGN KEY (program_id) REFERENCES program(id)
    ON DELETE RESTRICT;

ALTER TABLE funding_request
    ADD CONSTRAINT ck_funding_request_status
    CHECK (status IN ('draft', 'submitted', 'under_review',
                      'approved', 'denied', 'withdrawn'));

CREATE INDEX ix_funding_request_applicant_id
    ON funding_request (applicant_id);

CREATE INDEX ix_funding_request_program_id
    ON funding_request (program_id);

CREATE INDEX ix_funding_request_status
    ON funding_request (status);
```

## Rules

1. **snake_case** for all table and column names.
2. **UUID primary keys** with `DEFAULT gen_random_uuid()` -- never serial/integer.
3. Every table gets `created_at` and `updated_at` TIMESTAMP columns.
4. Constraint naming: `pk_`, `fk_`, `uq_`, `ck_`, `ix_` prefixes.
5. Foreign keys always specify `ON DELETE` behavior explicitly.
6. Index every foreign key column and any column used in WHERE filters.
7. Use `VARCHAR(n)` for bounded strings, `TEXT` for unbounded.
8. Use `NUMERIC(p,s)` for monetary values -- never float/real.
9. One migration file per entity -- never bundle multiple tables.
10. File naming: `{YYYYMMDDHHMMSS}_{entity_name}.sql`.
