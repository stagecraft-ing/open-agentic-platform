-- Migration 30 down — Spec 123 reverse migration (BEST-EFFORT).
--
-- WARNING: This down migration cannot perfectly reverse migration 30. The
-- forward migration:
--   1. Dedupes agent_catalog rows by (org_id, name, version, content_hash);
--      when N projects authored identical content, only one canonical row
--      survives. The down migration cannot reconstruct the absorbed rows
--      because their original ids are gone (audit re-pointing collapsed
--      them onto the canonical id).
--   2. Materialises bindings as one-per-(project, name); the forward
--      migration loses information about which specific version each
--      project authored if multiple versions existed (only MAX is bound).
--
-- What this script DOES recover:
--   - Adds back project_id columns and reconstructs project ownership for
--     the surviving canonical rows by reading project_agent_bindings.
--   - For agent_catalog rows that have multiple bindings post-migration
--     (one row → many projects), the down migration picks ONE project_id
--     arbitrarily (lowest project UUID) — there is no way to "re-fan out"
--     into N rows because the migration log doesn't preserve the absorbed
--     row contents (only ids and pointers).
--   - For agent_catalog rows with NO bindings (e.g. drafts authored after
--     migration 30 ran), project_id falls back to a placeholder UUID
--     populated from `created_by`'s first project membership; if no such
--     project exists the script aborts.
--
-- Use this only for local dev DB rollback to test forward-migration
-- iterations. Production rollback is not supported; restore from a
-- pre-migration snapshot instead.

-- ===========================================================================
-- 1. Add project_id columns back
-- ===========================================================================

ALTER TABLE agent_catalog
    ADD COLUMN project_id UUID;
ALTER TABLE agent_catalog_audit
    ADD COLUMN project_id UUID;

-- ===========================================================================
-- 2. Best-effort backfill of agent_catalog.project_id from bindings.
--    Pick the lowest project_id (UUID order) when multiple projects bind
--    the same row.
-- ===========================================================================

UPDATE agent_catalog ac
   SET project_id = sub.project_id
  FROM (
      SELECT DISTINCT ON (org_agent_id)
             org_agent_id, project_id
        FROM project_agent_bindings
       ORDER BY org_agent_id, project_id
  ) sub
 WHERE sub.org_agent_id = ac.id;

-- ===========================================================================
-- 3. For unbound rows (drafts), fall back to the creator's first project.
-- ===========================================================================

UPDATE agent_catalog ac
   SET project_id = (
       SELECT pm.project_id
         FROM project_members pm
        WHERE pm.user_id = ac.created_by
        ORDER BY pm.project_id
        LIMIT 1
   )
 WHERE ac.project_id IS NULL;

-- Hard fail if any rows remain unresolved — the down migration cannot
-- proceed without a project to point at.
DO $migration_30_down_check$
DECLARE
    orphan_count INTEGER;
BEGIN
    SELECT count(*) INTO orphan_count
      FROM agent_catalog
     WHERE project_id IS NULL;
    IF orphan_count > 0 THEN
        RAISE EXCEPTION
            'Migration 30 down aborted: % agent_catalog rows have no resolvable project_id (no bindings, creator has no projects). Restore from a pre-migration snapshot instead of using this script.',
            orphan_count;
    END IF;
END $migration_30_down_check$;

ALTER TABLE agent_catalog ALTER COLUMN project_id SET NOT NULL;

-- ===========================================================================
-- 4. Backfill agent_catalog_audit.project_id from agent_catalog
-- ===========================================================================

UPDATE agent_catalog_audit a
   SET project_id = ac.project_id
  FROM agent_catalog ac
 WHERE ac.id = a.agent_id;

-- For audit rows orphaned by the dedup re-pointing (shouldn't exist post-
-- migration but defensive), fall back to actor's first project.
UPDATE agent_catalog_audit a
   SET project_id = (
       SELECT pm.project_id
         FROM project_members pm
        WHERE pm.user_id = a.actor_user_id
        ORDER BY pm.project_id
        LIMIT 1
   )
 WHERE a.project_id IS NULL;

ALTER TABLE agent_catalog_audit ALTER COLUMN project_id SET NOT NULL;

-- ===========================================================================
-- 5. Drop project_agent_bindings + log table + org_id indexes/columns
-- ===========================================================================

DROP TABLE IF EXISTS project_agent_bindings;
DROP TABLE IF EXISTS agent_catalog_migration_30_log;

ALTER TABLE agent_catalog
    DROP CONSTRAINT IF EXISTS agent_catalog_org_id_name_version_key;
DROP INDEX IF EXISTS agent_catalog_org_name_idx;
DROP INDEX IF EXISTS agent_catalog_org_status_idx;
ALTER TABLE agent_catalog DROP COLUMN org_id;

DROP INDEX IF EXISTS agent_catalog_audit_org_idx;
ALTER TABLE agent_catalog_audit DROP COLUMN org_id;

-- ===========================================================================
-- 6. Restore old constraints + indexes
-- ===========================================================================

ALTER TABLE agent_catalog
    ADD CONSTRAINT agent_catalog_project_id_name_version_key
        UNIQUE (project_id, name, version);
CREATE INDEX agent_catalog_project_name_idx
    ON agent_catalog (project_id, name);
CREATE INDEX agent_catalog_project_status_idx
    ON agent_catalog (project_id, status);

CREATE INDEX agent_catalog_audit_project_idx
    ON agent_catalog_audit (project_id, created_at DESC);
