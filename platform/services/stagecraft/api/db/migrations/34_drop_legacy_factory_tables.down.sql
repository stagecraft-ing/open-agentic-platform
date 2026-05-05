-- Spec 139 Phase 4 (T093) — best-effort rollback for the legacy factory
-- table drop. **Schema-only restoration; the original row contents are
-- not recoverable from this script** (substrate-projected reads stay
-- working post-rollback because browse.ts / opcBundle.ts / runs.ts
-- query the substrate directly, not these tables).
--
-- This .down.sql exists so a developer can re-run migration 34 in dev
-- after a partial rollback test without DROPping their entire local DB.

BEGIN;

-- Re-create factory_adapters with the same schema as migration 18 §27.
CREATE TABLE IF NOT EXISTS factory_adapters (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    version     TEXT NOT NULL,
    manifest    JSONB NOT NULL,
    source_sha  TEXT NOT NULL,
    synced_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, name)
);
CREATE INDEX IF NOT EXISTS idx_factory_adapters_org
    ON factory_adapters(org_id);

-- Re-create factory_contracts with the same schema as migration 18 §40.
CREATE TABLE IF NOT EXISTS factory_contracts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    version     TEXT NOT NULL,
    schema      JSONB NOT NULL,
    source_sha  TEXT NOT NULL,
    synced_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, name, version)
);

-- Re-create factory_processes with the same schema as migration 18.
CREATE TABLE IF NOT EXISTS factory_processes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    version     TEXT NOT NULL,
    definition  JSONB NOT NULL,
    source_sha  TEXT NOT NULL,
    synced_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, name, version)
);

COMMIT;
