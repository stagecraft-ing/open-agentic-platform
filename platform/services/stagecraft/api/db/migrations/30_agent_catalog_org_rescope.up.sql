-- Migration 30 — Spec 123: Agent Catalog Org-Rescope.
--
-- Reverts the agent_catalog and agent_catalog_audit `project_id` scoping that
-- spec 119 established (migration 28) back to `org_id`, and introduces the
-- `project_agent_bindings` join table so projects consume org-managed agents
-- as pinned versions instead of authoring their own.
--
-- Spec/reality drift discovered at implementation time:
--   - Spec 123 §3 / §4.3 also calls for migrating `agent_policies` from
--     `project_id` to `org_id`. In reality, `agent_policies` was created in
--     migration 4 with `org_id` and was never project-scoped (migrations 27,
--     28, 29 do not touch it). The agent_policies SQL block from spec §4.3
--     is therefore a no-op and is omitted from this migration. The spec 123
--     closure phase reconciles the spec body with this actual state.
--
--   - Spec 123 §4.1 names indexes `agent_catalog_proj_*` and
--     `agent_catalog_audit_proj_idx`. The actual indexes installed by
--     migration 28 are `agent_catalog_project_name_idx`,
--     `agent_catalog_project_status_idx`, and
--     `agent_catalog_audit_project_idx`. This migration drops those names.
--
-- Binding shape: per implementation decision (spec 123 plan, validated
-- against §6.2 UI semantics), every (project_id, agent_name) pair gets at
-- most ONE binding row pinned to that project's MAX-version row. Repinning
-- via PATCH /api/projects/:projectId/agents/:bindingId updates the same
-- binding atomically (Phase 2 work). Older versions remain in
-- `agent_catalog` as accessible history but are not project-bound.

-- ===========================================================================
-- 1. Add org_id columns with backfill default
-- ===========================================================================

ALTER TABLE agent_catalog
    ADD COLUMN org_id TEXT NOT NULL DEFAULT 'default';

ALTER TABLE agent_catalog_audit
    ADD COLUMN org_id TEXT NOT NULL DEFAULT 'default';

-- ===========================================================================
-- 2. Backfill org_id from projects.org_id (uuid → text cast)
-- ===========================================================================

UPDATE agent_catalog ac
   SET org_id = p.org_id::text
  FROM projects p
 WHERE p.id = ac.project_id;

UPDATE agent_catalog_audit a
   SET org_id = p.org_id::text
  FROM projects p
 WHERE p.id = a.project_id;

-- ===========================================================================
-- 3. Drop defaults (org_id is now authoritative; no fallback writes allowed)
-- ===========================================================================

ALTER TABLE agent_catalog ALTER COLUMN org_id DROP DEFAULT;
ALTER TABLE agent_catalog_audit ALTER COLUMN org_id DROP DEFAULT;

-- ===========================================================================
-- 4. Pre-flight divergence check (spec 123 §4.5 / OQ-1, T015)
--    Detects same-name same-version agents across projects in one org with
--    DIFFERENT content_hash. That cannot be safely deduped — the migration
--    aborts and the operator reconciles by hand (rename one, fork, accept
--    one as canonical) before re-running.
-- ===========================================================================

DO $migration_30_preflight$
DECLARE
    conflict_count INTEGER;
    conflict_summary TEXT;
BEGIN
    WITH conflicts AS (
        SELECT org_id, name, version, count(DISTINCT content_hash) AS variants
          FROM agent_catalog
         GROUP BY org_id, name, version
        HAVING count(DISTINCT content_hash) > 1
    )
    SELECT count(*),
           string_agg(
               org_id || '/' || name || '@v' || version
                   || ' (' || variants || ' divergent hashes)',
               '; ')
      INTO conflict_count, conflict_summary
      FROM conflicts;

    IF conflict_count > 0 THEN
        RAISE EXCEPTION
            'Migration 30 aborted: % (org_id, name, version) tuple(s) have divergent content_hash across projects in the same org. Reconcile by hand (rename one, fork to a new agent, or accept one project''s content as canonical) and re-run migration 30. Conflicts: %',
            conflict_count, conflict_summary;
    END IF;
END $migration_30_preflight$;

-- ===========================================================================
-- 5. Migration log table (spec 123 T017)
--    Records every absorption + binding decision for replay/audit. Survives
--    the migration. Unique per absorbed_id so re-running the migration
--    against a partially-migrated DB would surface a constraint violation
--    rather than silently double-logging.
-- ===========================================================================

CREATE TABLE agent_catalog_migration_30_log (
    id                          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    absorbed_id                 UUID NOT NULL UNIQUE,
    kept_id                     UUID NOT NULL,
    project_id_added_as_binding UUID NOT NULL,
    content_hash                TEXT NOT NULL,
    org_id                      TEXT NOT NULL,
    name                        TEXT NOT NULL,
    absorbed_version            INTEGER NOT NULL,
    kept_version                INTEGER NOT NULL,
    decided_at                  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ===========================================================================
-- 6. Create project_agent_bindings (spec 123 §4.4)
--    One binding per (project_id, org_agent_id). Phase 2 API enforces the
--    stronger logical invariant of "one binding per (project, agent name)"
--    by deleting the old binding before inserting a new one on repin.
-- ===========================================================================

CREATE TABLE project_agent_bindings (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id          UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    org_agent_id        UUID NOT NULL REFERENCES agent_catalog(id) ON DELETE RESTRICT,
    pinned_version      INTEGER NOT NULL,
    pinned_content_hash TEXT NOT NULL,
    bound_by            UUID NOT NULL REFERENCES users(id),
    bound_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (project_id, org_agent_id)
);

CREATE INDEX project_agent_bindings_project_idx
    ON project_agent_bindings (project_id);
CREATE INDEX project_agent_bindings_agent_idx
    ON project_agent_bindings (org_agent_id);

-- ===========================================================================
-- 7. Build dedup plan in a temp table (canonical = lowest id per
--    (org_id, name, version), since divergence has been ruled out by §4
--    above; same-version rows are guaranteed identical content_hash).
-- ===========================================================================

CREATE TEMP TABLE migration_30_plan AS
SELECT
    ac.id           AS row_id,
    ac.project_id,
    ac.org_id,
    ac.name,
    ac.version,
    ac.content_hash,
    ac.created_by,
    (SELECT id FROM agent_catalog ac2
      WHERE ac2.org_id  = ac.org_id
        AND ac2.name    = ac.name
        AND ac2.version = ac.version
      ORDER BY id
      LIMIT 1) AS kept_id
FROM agent_catalog ac;

-- ===========================================================================
-- 8. Materialise bindings: ONE row per (project_id, agent_name) pair
--    pinned to that project's MAX-version absorbed row's canonical
--    counterpart. Implementation choice (A) per spec 123 §6.2 UI semantics
--    and §5.2 PATCH-by-bindingId model.
-- ===========================================================================

INSERT INTO project_agent_bindings
    (project_id, org_agent_id, pinned_version, pinned_content_hash, bound_by)
SELECT DISTINCT ON (p.project_id, p.name)
    p.project_id,
    p.kept_id,
    p.version,
    p.content_hash,
    p.created_by
FROM migration_30_plan p
ORDER BY p.project_id, p.name, p.version DESC, p.row_id;

-- ===========================================================================
-- 9. Log every absorption (rows whose row_id != kept_id were absorbed).
-- ===========================================================================

INSERT INTO agent_catalog_migration_30_log
    (absorbed_id, kept_id, project_id_added_as_binding,
     content_hash, org_id, name, absorbed_version, kept_version)
SELECT
    p.row_id,
    p.kept_id,
    p.project_id,
    p.content_hash,
    p.org_id,
    p.name,
    p.version,
    (SELECT version FROM agent_catalog WHERE id = p.kept_id)
FROM migration_30_plan p
WHERE p.row_id <> p.kept_id;

-- ===========================================================================
-- 10. Re-point audit rows from absorbed → canonical so the audit chain
--     survives the dedup.
-- ===========================================================================

UPDATE agent_catalog_audit a
   SET agent_id = p.kept_id
  FROM migration_30_plan p
 WHERE a.agent_id = p.row_id
   AND p.row_id <> p.kept_id;

-- ===========================================================================
-- 11. Drop absorbed catalog rows (canonical rows survive).
-- ===========================================================================

DELETE FROM agent_catalog ac
 WHERE EXISTS (
     SELECT 1
       FROM migration_30_plan p
      WHERE p.row_id = ac.id
        AND p.row_id <> p.kept_id
 );

DROP TABLE migration_30_plan;

-- ===========================================================================
-- 12. Drop project_id columns + rebuild unique constraints and indexes
-- ===========================================================================

ALTER TABLE agent_catalog
    DROP CONSTRAINT IF EXISTS agent_catalog_project_id_name_version_key;
DROP INDEX IF EXISTS agent_catalog_project_name_idx;
DROP INDEX IF EXISTS agent_catalog_project_status_idx;

ALTER TABLE agent_catalog DROP COLUMN project_id;
ALTER TABLE agent_catalog
    ADD CONSTRAINT agent_catalog_org_id_name_version_key
        UNIQUE (org_id, name, version);

CREATE INDEX agent_catalog_org_name_idx
    ON agent_catalog (org_id, name);
CREATE INDEX agent_catalog_org_status_idx
    ON agent_catalog (org_id, status);

DROP INDEX IF EXISTS agent_catalog_audit_project_idx;
ALTER TABLE agent_catalog_audit DROP COLUMN project_id;
CREATE INDEX agent_catalog_audit_org_idx
    ON agent_catalog_audit (org_id, created_at DESC);
