-- Spec 119 Phase C — clean up the workspace_id columns left orphan after
-- migration 27 dropped the workspaces table.
--
-- Migration 27 left six tables with workspace_id columns (no FK because
-- DROP TABLE workspaces CASCADE removed them). The data in those columns
-- is now orphan UUIDs — there is no workspace to resolve back to. This
-- migration finishes the collapse:
--
--   - scaffold_jobs.workspace_id        : DROP (project_id is the canonical scope)
--   - desktop_refresh_tokens.workspace_id: DROP (was a JWT claim that no longer exists)
--   - factory_artifacts.workspace_id    : RENAME -> project_id (orphan UUIDs preserved)
--   - promotions.workspace_id           : RENAME -> project_id (orphan UUIDs preserved)
--   - agent_catalog.workspace_id        : RENAME -> project_id (orphan UUIDs preserved)
--   - agent_catalog_audit.workspace_id  : RENAME -> project_id (orphan UUIDs preserved)
--
-- Existing UUID values in the renamed columns no longer reference a real
-- row anywhere — pre-collapse this was a workspace UUID; post-collapse no
-- foreign key is added because the data isn't backfillable (workspaces
-- table is gone, projects.workspace_id is gone). Data is retained for
-- traceability; new writes carry real project UUIDs.

-- ===========================================================================
-- 1. scaffold_jobs: drop the redundant workspace_id (project_id already exists)
-- ===========================================================================

DROP INDEX IF EXISTS idx_scaffold_jobs_workspace_status;
ALTER TABLE scaffold_jobs DROP COLUMN workspace_id;

-- ===========================================================================
-- 2. desktop_refresh_tokens: drop workspace_id (was a JWT claim — spec 119
--    removed oap_workspace_id from the Rauthy custom-attribute set)
-- ===========================================================================

ALTER TABLE desktop_refresh_tokens DROP COLUMN workspace_id;

-- ===========================================================================
-- 3. factory_artifacts: rename workspace_id -> project_id, refresh indexes
-- ===========================================================================

DROP INDEX IF EXISTS idx_factory_artifacts_workspace;
DROP INDEX IF EXISTS idx_factory_artifacts_workspace_hash;

ALTER TABLE factory_artifacts RENAME COLUMN workspace_id TO project_id;

CREATE INDEX idx_factory_artifacts_project ON factory_artifacts(project_id);
CREATE INDEX idx_factory_artifacts_project_hash
    ON factory_artifacts(project_id, content_hash);

-- ===========================================================================
-- 4. promotions: rename workspace_id -> project_id, refresh indexes
-- ===========================================================================

DROP INDEX IF EXISTS idx_promotions_workspace;

ALTER TABLE promotions RENAME COLUMN workspace_id TO project_id;

CREATE INDEX idx_promotions_project ON promotions(project_id);

-- ===========================================================================
-- 5. agent_catalog: rename workspace_id -> project_id, rebuild unique +
--    indexes that referenced the old column name
-- ===========================================================================

DROP INDEX IF EXISTS agent_catalog_ws_name_idx;
DROP INDEX IF EXISTS agent_catalog_ws_status_idx;

ALTER TABLE agent_catalog
    DROP CONSTRAINT IF EXISTS agent_catalog_workspace_id_name_version_key;
ALTER TABLE agent_catalog RENAME COLUMN workspace_id TO project_id;
ALTER TABLE agent_catalog
    ADD CONSTRAINT agent_catalog_project_id_name_version_key
    UNIQUE (project_id, name, version);

CREATE INDEX agent_catalog_project_name_idx
    ON agent_catalog (project_id, name);
CREATE INDEX agent_catalog_project_status_idx
    ON agent_catalog (project_id, status);

-- ===========================================================================
-- 6. agent_catalog_audit: rename workspace_id -> project_id, refresh index
-- ===========================================================================

DROP INDEX IF EXISTS agent_catalog_audit_workspace_idx;

ALTER TABLE agent_catalog_audit RENAME COLUMN workspace_id TO project_id;

CREATE INDEX agent_catalog_audit_project_idx
    ON agent_catalog_audit (project_id, created_at DESC);
