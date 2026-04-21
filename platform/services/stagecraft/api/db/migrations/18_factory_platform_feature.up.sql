-- Spec 108 Phase 2: Factory as a First-Class Platform Feature.
--
-- Promotes the repo-rooted `factory/` tree into four org-scoped tables:
--   * factory_upstreams  — replaces upstream-map.yaml (one config per org)
--   * factory_adapters   — derived from the factory source repo
--   * factory_contracts  — derived JSON schemas
--   * factory_processes  — derived process definitions
--
-- Adapters / contracts / processes are replaced on every sync; only the latest
-- snapshot per (org, name[, version]) is retained. factory_upstreams is the
-- single writable config — everything else is produced by the sync worker.

CREATE TABLE factory_upstreams (
    org_id           UUID PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
    factory_source   TEXT NOT NULL,
    factory_ref      TEXT NOT NULL DEFAULT 'main',
    template_source  TEXT NOT NULL,
    template_ref     TEXT NOT NULL DEFAULT 'main',
    last_synced_at   TIMESTAMPTZ,
    last_sync_sha    JSONB,
    last_sync_status TEXT,
    last_sync_error  TEXT,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE factory_adapters (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    version     TEXT NOT NULL,
    manifest    JSONB NOT NULL,
    source_sha  TEXT NOT NULL,
    synced_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, name)
);

CREATE INDEX idx_factory_adapters_org ON factory_adapters(org_id);

CREATE TABLE factory_contracts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    version     TEXT NOT NULL,
    schema      JSONB NOT NULL,
    source_sha  TEXT NOT NULL,
    synced_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, name, version)
);

CREATE INDEX idx_factory_contracts_org ON factory_contracts(org_id);

CREATE TABLE factory_processes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    version     TEXT NOT NULL,
    definition  JSONB NOT NULL,
    source_sha  TEXT NOT NULL,
    synced_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (org_id, name, version)
);

CREATE INDEX idx_factory_processes_org ON factory_processes(org_id);
