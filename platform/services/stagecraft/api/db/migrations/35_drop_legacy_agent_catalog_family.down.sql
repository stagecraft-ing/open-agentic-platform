-- Spec 139 Phase 4b — best-effort rollback for the spec 111/123 family
-- drop. **Schema-only restoration; the original row contents are not
-- recoverable from this script** (substrate-projected reads stay
-- working post-rollback because catalog.ts / bindings.ts query the
-- substrate directly, not these tables).
--
-- This .down.sql exists so a developer can re-run migration 35 in dev
-- after a partial rollback test without DROPping their entire local DB.

BEGIN;

-- Re-add factory_upstreams legacy columns (nullable, default 'main' on
-- the ref columns) — matches the schema as of migration 32 §post-Phase 1.
ALTER TABLE factory_upstreams
    ADD COLUMN IF NOT EXISTS factory_source TEXT,
    ADD COLUMN IF NOT EXISTS factory_ref TEXT DEFAULT 'main',
    ADD COLUMN IF NOT EXISTS template_source TEXT,
    ADD COLUMN IF NOT EXISTS template_ref TEXT DEFAULT 'main';

-- Re-create agent_catalog with the same schema as migration 21 +
-- migration 30's org-rescope. UNIQUE on (org_id, name, version).
CREATE TABLE IF NOT EXISTS agent_catalog (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id       TEXT NOT NULL,
    name         TEXT NOT NULL,
    version      INTEGER NOT NULL DEFAULT 1,
    status       TEXT NOT NULL DEFAULT 'draft',
    frontmatter  JSONB NOT NULL,
    body_markdown TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_by   UUID NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, name, version)
);

CREATE TABLE IF NOT EXISTS agent_catalog_audit (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id      UUID NOT NULL,
    org_id        TEXT NOT NULL,
    action        TEXT NOT NULL,
    actor_user_id UUID NOT NULL,
    before        JSONB,
    after         JSONB,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS project_agent_bindings (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id          UUID NOT NULL,
    org_agent_id        UUID NOT NULL,
    pinned_version      INTEGER NOT NULL,
    pinned_content_hash TEXT NOT NULL,
    bound_by            UUID NOT NULL,
    bound_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (project_id, org_agent_id)
);

COMMIT;
