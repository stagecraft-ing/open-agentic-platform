-- Spec 139 Phase 1 — Factory Artifact Substrate.
--
-- Introduces:
--   * factory_artifact_substrate        — content-addressed substrate, one
--                                         row per upstream file (verbatim
--                                         mirror)
--   * factory_artifact_substrate_audit  — per-row lifecycle audit trail
--   * factory_bindings                  — universal pinning (replaces spec
--                                         123 project_agent_bindings; same
--                                         shape, any kind)
--
-- **Naming Note:** the new substrate table is `factory_artifact_substrate`
-- (NOT `factory_artifacts`) because spec 082's per-run artifact provenance
-- registry already owns that name (migration 8, extended by 12/27/28).
-- See spec 139 §2.1 Naming Note for the full rationale.
--
-- Generalises factory_upstreams from a singleton row per org into N rows
-- per org keyed by (org_id, source_id). Legacy columns
-- (factory_source / factory_ref / template_source / template_ref) are
-- retained nullable so syncWorker.ts continues to read them through
-- Phase 1; Phase 4 drops them.
--
-- The existing single row per org migrates to source_id='legacy-mixed'
-- with role='mixed' so spec 108's API surface continues to serve.

-- ---------------------------------------------------------------------
-- 1. factory_upstreams generalisation
-- ---------------------------------------------------------------------

ALTER TABLE factory_upstreams
    ADD COLUMN source_id TEXT,
    ADD COLUMN role      TEXT,
    ADD COLUMN subpath   TEXT,
    ADD COLUMN repo_url  TEXT,
    ADD COLUMN ref       TEXT;

-- Backfill for existing legacy singleton rows. The legacy row carried both
-- factory + template configs; tag it 'legacy-mixed' / role='mixed' so the
-- old PK can be swapped. We materialise repo_url + ref from the legacy
-- factory_source/factory_ref so future sync workers can route through the
-- single row uniformly.
UPDATE factory_upstreams
   SET source_id = 'legacy-mixed',
       role      = 'mixed',
       repo_url  = factory_source,
       ref       = factory_ref
 WHERE source_id IS NULL;

ALTER TABLE factory_upstreams
    ALTER COLUMN source_id SET NOT NULL,
    ALTER COLUMN role      SET NOT NULL,
    ALTER COLUMN repo_url  SET NOT NULL,
    ALTER COLUMN ref       SET NOT NULL,
    ALTER COLUMN ref       SET DEFAULT 'main';

-- Swap PK from (org_id) to (org_id, source_id) so multiple sources per
-- org are addressable. The legacy singleton row continues to read by
-- (org_id, 'legacy-mixed').
ALTER TABLE factory_upstreams DROP CONSTRAINT factory_upstreams_pkey;
ALTER TABLE factory_upstreams
    ADD CONSTRAINT factory_upstreams_pkey PRIMARY KEY (org_id, source_id);

-- Phase 1 transition: legacy column writers may continue producing rows
-- without filling factory_source/template_source if they only carry the
-- new (source_id, role, repo_url) shape. Phase 4 drops these columns.
ALTER TABLE factory_upstreams
    ALTER COLUMN factory_source  DROP NOT NULL,
    ALTER COLUMN template_source DROP NOT NULL;

-- ---------------------------------------------------------------------
-- 2. factory_artifact_substrate — content-addressed substrate
-- ---------------------------------------------------------------------

CREATE TABLE factory_artifact_substrate (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    origin          TEXT NOT NULL,
    path            TEXT NOT NULL,
    kind            TEXT NOT NULL,
    bundle_id       UUID,

    version         INTEGER NOT NULL DEFAULT 1,
    status          TEXT NOT NULL DEFAULT 'active',

    upstream_sha    TEXT,
    upstream_body   TEXT,
    user_body       TEXT,
    user_modified_at  TIMESTAMPTZ,
    user_modified_by  UUID,

    -- Generated stored: consumers select one canonical field. PG14+ supports
    -- COALESCE in generated columns when both inputs are stored.
    effective_body  TEXT GENERATED ALWAYS AS (COALESCE(user_body, upstream_body)) STORED,

    content_hash    TEXT NOT NULL,
    frontmatter     JSONB,

    conflict_state         TEXT,
    conflict_upstream_sha  TEXT,
    conflict_resolved_at   TIMESTAMPTZ,
    conflict_resolved_by   UUID,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Spec 139 §2.1 — versioned addressability per (org, origin, path).
    UNIQUE (org_id, origin, path, version),

    -- Closed-set guards. Misclassification at sync time is a sync-worker
    -- bug fix; the substrate refuses unknown values to surface drift loud.
    CONSTRAINT factory_artifact_substrate_kind_chk
        CHECK (kind IN (
            'agent', 'skill', 'process-stage', 'adapter-manifest',
            'contract-schema', 'pattern', 'page-type-reference',
            'sample-html', 'reference-data', 'invariant',
            'pipeline-orchestrator'
        )),
    CONSTRAINT factory_artifact_substrate_status_chk
        CHECK (status IN ('active', 'retired')),
    CONSTRAINT factory_artifact_substrate_conflict_state_chk
        CHECK (conflict_state IS NULL OR conflict_state IN ('ok', 'diverged'))
);

CREATE INDEX factory_artifact_substrate_org_kind_idx
    ON factory_artifact_substrate (org_id, kind);
CREATE INDEX factory_artifact_substrate_org_origin_path_idx
    ON factory_artifact_substrate (org_id, origin, path);
CREATE INDEX factory_artifact_substrate_bundle_idx
    ON factory_artifact_substrate (bundle_id) WHERE bundle_id IS NOT NULL;
CREATE INDEX factory_artifact_substrate_conflict_idx
    ON factory_artifact_substrate (org_id, conflict_state)
    WHERE conflict_state = 'diverged';

-- ---------------------------------------------------------------------
-- 3. factory_artifact_substrate_audit — per-row lifecycle audit
-- ---------------------------------------------------------------------

CREATE TABLE factory_artifact_substrate_audit (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    artifact_id   UUID NOT NULL,
    org_id        UUID NOT NULL,
    action        TEXT NOT NULL,
    actor_user_id UUID,
    before        JSONB,
    after         JSONB,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT factory_artifact_substrate_audit_action_chk
        CHECK (action IN (
            'artifact.synced',
            'artifact.retired',
            'artifact.overridden',
            'artifact.override_cleared',
            'artifact.conflict_detected',
            'artifact.conflict_resolved',
            -- Spec 139 §6.4 (extended Phase 2): absorbs spec 111
            -- `agent_catalog_audit.action='fork'` via T051 mapping.
            'artifact.forked'
        ))
);

CREATE INDEX factory_artifact_substrate_audit_artifact_idx
    ON factory_artifact_substrate_audit (artifact_id);
CREATE INDEX factory_artifact_substrate_audit_org_idx
    ON factory_artifact_substrate_audit (org_id, created_at DESC);

-- ---------------------------------------------------------------------
-- 4. factory_bindings — universal pinning (replaces project_agent_bindings)
-- ---------------------------------------------------------------------

CREATE TABLE factory_bindings (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id           UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    artifact_id          UUID NOT NULL REFERENCES factory_artifact_substrate(id) ON DELETE RESTRICT,
    pinned_version       INTEGER NOT NULL,
    pinned_content_hash  TEXT NOT NULL,
    bound_by             UUID NOT NULL,
    bound_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (project_id, artifact_id)
);

CREATE INDEX factory_bindings_project_idx
    ON factory_bindings (project_id);
CREATE INDEX factory_bindings_artifact_idx
    ON factory_bindings (artifact_id);
