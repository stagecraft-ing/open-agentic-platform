-- Spec 119 — Collapse workspace into project.
--
-- Drops the workspaces / workspace_grants / document_bindings tables and
-- promotes project to the top-level governance unit under organization.
-- Knowledge corpus, source connectors, sync runs, extraction runs, and
-- clone runs all become project-scoped. Permission grants merge from
-- workspace_grants into a new project_grants table per spec 119 §6.4.
--
-- Pre-flight (asserted at deploy time, not enforced by this file):
--   - no inflight knowledge_extraction_runs / project_clone_runs
--   - every workspace contains at most one project (the cardinality
--     assumption under which the "first project per workspace" backfill
--     is unambiguous)
--   - every project's workspace_id resolves to an existing workspace
--
-- Backfill rule for moving tables: workspace_id resolves to "the oldest
-- project in that workspace" via a DISTINCT ON (workspace_id) ... ORDER
-- BY created_at projection over `projects`. Single-project-per-workspace
-- pre-flight makes this deterministic.
--
-- Out of scope (deferred to spec 119 Phase C code-side rename):
--   - factory_artifacts.workspace_id, promotions.workspace_id,
--     desktop_refresh_tokens.workspace_id (no FK to workspaces, columns
--     stay populated with now-orphan UUIDs until Phase C)
--   - scaffold_jobs.workspace_id, agent_catalog.workspace_id,
--     agent_catalog_audit.workspace_id (FK to workspaces is dropped here
--     via DROP TABLE ... CASCADE; columns stay populated until Phase C)

-- ===========================================================================
-- 1. projects: promote object_store_bucket from workspaces
-- ===========================================================================

ALTER TABLE projects
    ADD COLUMN object_store_bucket TEXT;

UPDATE projects p
   SET object_store_bucket = w.object_store_bucket
  FROM workspaces w
 WHERE p.workspace_id = w.id;

ALTER TABLE projects
    ALTER COLUMN object_store_bucket SET NOT NULL;

-- ===========================================================================
-- 2. knowledge_objects: add project_id, backfill via document_bindings
-- ===========================================================================
--
-- Step 2a — most-recent binding wins. Covers the typical 1:1 case and
-- resolves the rare N:1 case deterministically by bound_at.
-- Step 2b — zero-binding fallback: assign to the first project in the
-- knowledge_object's workspace so I-2 ("no orphans post-cutover") holds.

ALTER TABLE knowledge_objects
    ADD COLUMN project_id UUID REFERENCES projects(id);

UPDATE knowledge_objects ko
   SET project_id = b.project_id
  FROM (
      SELECT DISTINCT ON (knowledge_object_id)
             knowledge_object_id,
             project_id
        FROM document_bindings
       ORDER BY knowledge_object_id, bound_at DESC
  ) b
 WHERE ko.id = b.knowledge_object_id;

UPDATE knowledge_objects ko
   SET project_id = p.id
  FROM (
      SELECT DISTINCT ON (workspace_id) workspace_id, id
        FROM projects
       ORDER BY workspace_id, created_at
  ) p
 WHERE ko.project_id IS NULL
   AND ko.workspace_id = p.workspace_id;

ALTER TABLE knowledge_objects
    ALTER COLUMN project_id SET NOT NULL;

-- ===========================================================================
-- 3. Drop document_bindings (project ownership lives on knowledge_objects)
-- ===========================================================================

DROP INDEX IF EXISTS idx_document_bindings_project;
DROP INDEX IF EXISTS idx_document_bindings_knowledge;
DROP TABLE document_bindings;

-- ===========================================================================
-- 4. knowledge_objects: drop workspace_id, recreate indexes on project_id
-- ===========================================================================

DROP INDEX IF EXISTS idx_knowledge_objects_workspace;
DROP INDEX IF EXISTS idx_knowledge_objects_state;

ALTER TABLE knowledge_objects DROP COLUMN workspace_id;

CREATE INDEX idx_knowledge_objects_project ON knowledge_objects(project_id);
CREATE INDEX idx_knowledge_objects_state ON knowledge_objects(project_id, state);

-- ===========================================================================
-- 5. source_connectors: rename workspace_id → project_id
-- ===========================================================================

ALTER TABLE source_connectors
    ADD COLUMN project_id UUID REFERENCES projects(id);

UPDATE source_connectors sc
   SET project_id = p.id
  FROM (
      SELECT DISTINCT ON (workspace_id) workspace_id, id
        FROM projects
       ORDER BY workspace_id, created_at
  ) p
 WHERE sc.workspace_id = p.workspace_id;

ALTER TABLE source_connectors ALTER COLUMN project_id SET NOT NULL;

DROP INDEX IF EXISTS idx_source_connectors_workspace;
ALTER TABLE source_connectors DROP COLUMN workspace_id;

CREATE INDEX idx_source_connectors_project ON source_connectors(project_id);

-- ===========================================================================
-- 6. sync_runs: rename workspace_id → project_id
-- ===========================================================================

ALTER TABLE sync_runs
    ADD COLUMN project_id UUID REFERENCES projects(id);

UPDATE sync_runs sr
   SET project_id = p.id
  FROM (
      SELECT DISTINCT ON (workspace_id) workspace_id, id
        FROM projects
       ORDER BY workspace_id, created_at
  ) p
 WHERE sr.workspace_id = p.workspace_id;

ALTER TABLE sync_runs ALTER COLUMN project_id SET NOT NULL;

DROP INDEX IF EXISTS idx_sync_runs_workspace;
ALTER TABLE sync_runs DROP COLUMN workspace_id;

CREATE INDEX idx_sync_runs_project ON sync_runs(project_id);

-- ===========================================================================
-- 7. knowledge_extraction_runs: rename workspace_id → project_id
-- ===========================================================================

ALTER TABLE knowledge_extraction_runs
    ADD COLUMN project_id UUID REFERENCES projects(id) ON DELETE CASCADE;

UPDATE knowledge_extraction_runs ker
   SET project_id = p.id
  FROM (
      SELECT DISTINCT ON (workspace_id) workspace_id, id
        FROM projects
       ORDER BY workspace_id, created_at
  ) p
 WHERE ker.workspace_id = p.workspace_id;

ALTER TABLE knowledge_extraction_runs ALTER COLUMN project_id SET NOT NULL;

DROP INDEX IF EXISTS idx_knowledge_extraction_runs_workspace_status;
DROP INDEX IF EXISTS idx_knowledge_extraction_runs_workspace_completed;

ALTER TABLE knowledge_extraction_runs DROP COLUMN workspace_id;

CREATE INDEX idx_knowledge_extraction_runs_project_status
    ON knowledge_extraction_runs (project_id, status);
CREATE INDEX idx_knowledge_extraction_runs_project_completed
    ON knowledge_extraction_runs (project_id, completed_at DESC)
    WHERE completed_at IS NOT NULL;

-- ===========================================================================
-- 8. project_clone_runs: drop workspace_id (project_id col already holds
--    the destination project; org_id captures listing scope post-collapse)
-- ===========================================================================

DROP INDEX IF EXISTS idx_project_clone_runs_workspace_queued;
ALTER TABLE project_clone_runs DROP COLUMN workspace_id;

CREATE INDEX idx_project_clone_runs_org_queued
    ON project_clone_runs (org_id, queued_at DESC);

-- ===========================================================================
-- 9. workspace_grants → project_grants (rename + retype + expand by project)
-- ===========================================================================
--
-- workspace_grants.workspace_id is TEXT today (legacy seam C from migration
-- 3); project_grants.project_id must be UUID and FK projects(id). One
-- workspace_grants row expands to one project_grants row per project under
-- that workspace. ON CONFLICT DO NOTHING is defensive — pre-flight asserts
-- the cardinality assumption that prevents real conflicts.

CREATE TABLE project_grants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    enable_file_read BOOLEAN NOT NULL DEFAULT true,
    enable_file_write BOOLEAN NOT NULL DEFAULT true,
    enable_network BOOLEAN NOT NULL DEFAULT true,
    max_tier INTEGER NOT NULL DEFAULT 2,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, project_id)
);

INSERT INTO project_grants (
    user_id, project_id,
    enable_file_read, enable_file_write, enable_network, max_tier,
    created_at, updated_at
)
SELECT wg.user_id,
       p.id,
       wg.enable_file_read,
       wg.enable_file_write,
       wg.enable_network,
       wg.max_tier,
       wg.created_at,
       wg.updated_at
  FROM workspace_grants wg
  JOIN projects p ON p.workspace_id::TEXT = wg.workspace_id
 ON CONFLICT (user_id, project_id) DO NOTHING;

DROP TABLE workspace_grants;

-- ===========================================================================
-- 10. projects: drop workspace_id, flatten unique to (org_id, slug)
-- ===========================================================================

ALTER TABLE projects DROP CONSTRAINT IF EXISTS projects_workspace_id_slug_key;
ALTER TABLE projects DROP COLUMN workspace_id;
ALTER TABLE projects ADD CONSTRAINT projects_org_id_slug_key UNIQUE (org_id, slug);

-- ===========================================================================
-- 11. Drop workspaces. CASCADE drops the remaining FK constraints on
--     scaffold_jobs / agent_catalog / agent_catalog_audit (their
--     workspace_id columns are retained as orphaned UUIDs for Phase C
--     cleanup; data is preserved).
-- ===========================================================================

DROP TABLE workspaces CASCADE;
