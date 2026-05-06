-- Spec 142 rollback — best-effort dev-only revert from `text` back to
-- `uuid`. Synthetic-id strings written by the spec 139 runtime
-- (`synthetic-adapter-…`, `synthetic-process-…`) are NOT valid UUIDs,
-- so this rollback rejects them on NOT NULL columns and nulls them on
-- the nullable column. Do NOT apply on a production database that
-- contains any synthetic-id rows — data loss is the only path back to
-- a pure-uuid type.
--
-- The CASE expression below recognises the canonical 36-char dashed
-- UUID form and casts; everything else is converted to NULL. On
-- scaffold_jobs.factory_adapter_id and factory_runs.{adapter,process}_id
-- (NOT NULL columns) a non-UUID row would fail the cast; an operator
-- running this rollback on a populated dev DB MUST DELETE / TRUNCATE
-- the offending rows first.

BEGIN;

ALTER TABLE projects
    ALTER COLUMN factory_adapter_id TYPE uuid
    USING (
        CASE
            WHEN factory_adapter_id ~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                THEN factory_adapter_id::uuid
            ELSE NULL
        END
    );

ALTER TABLE scaffold_jobs
    ALTER COLUMN factory_adapter_id TYPE uuid
    USING factory_adapter_id::uuid;

ALTER TABLE factory_runs
    ALTER COLUMN adapter_id TYPE uuid
    USING adapter_id::uuid;

ALTER TABLE factory_runs
    ALTER COLUMN process_id TYPE uuid
    USING process_id::uuid;

COMMIT;
