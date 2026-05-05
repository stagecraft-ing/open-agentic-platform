-- Spec 139 Phase 4b — drop the spec 111 / 123 family + legacy
-- factory_upstreams per-side columns.
--
-- Drops:
--   * agent_catalog            — substrate-projected by user-authored
--                                rows (origin='user-authored', kind='agent')
--                                since Phase 2 (migration 33).
--                                api/agents/catalog.ts has read+written
--                                substrate-direct since Phase 4 narrow;
--                                api/agents/bindings.ts re-points to
--                                the substrate in this commit (B-1).
--   * agent_catalog_audit      — substrate-projected by
--                                factory_artifact_substrate_audit since
--                                Phase 2 (migration 33).
--   * project_agent_bindings   — replaced by `factory_bindings` since
--                                Phase 2 (migration 33). bindings.ts
--                                writes factory_bindings exclusively
--                                after Phase 4b (B-1).
--   * factory_upstreams.factory_source / factory_ref / template_source /
--     template_ref columns      — the legacy singleton wire shape
--                                composes from two N-per-org rows
--                                (`legacy-mixed` for factory side,
--                                `legacy-template-mixed` for template
--                                side); upstreams.ts re-points to the
--                                substrate-shape columns in B-3.
--
-- Backfill: any existing `legacy-mixed` rows with non-NULL
-- template_source/template_ref get a corresponding
-- `legacy-template-mixed` row inserted so the wire shape stays serving
-- byte-stable across the migration. Idempotent (ON CONFLICT DO NOTHING).
--
-- **Irreversibility note.** The .down.sql is best-effort schema-only:
-- it re-creates empty tables + columns so a developer can roll back the
-- migration in dev without restoring data. Once these tables drop in
-- prod, the row contents are gone forever — Phases 1-4-narrow + 4b's
-- substrate cutover are what made this safe. No prod data loss because
-- the substrate already carries every row that mattered.

BEGIN;

-- 1) Backfill the template-side row from the legacy columns. Skips rows
--    where template_source is NULL (an org configured factory only) and
--    is idempotent under repeat runs.
INSERT INTO factory_upstreams (
    org_id, source_id, role, repo_url, ref, subpath,
    last_synced_at, last_sync_status,
    created_at, updated_at
)
SELECT
    org_id,
    'legacy-template-mixed' AS source_id,
    'scaffold' AS role,
    template_source AS repo_url,
    COALESCE(template_ref, 'main') AS ref,
    NULL AS subpath,
    last_synced_at,
    last_sync_status,
    created_at,
    updated_at
FROM factory_upstreams
WHERE source_id = 'legacy-mixed'
  AND template_source IS NOT NULL
ON CONFLICT (org_id, source_id) DO NOTHING;

-- 2) Drop the legacy per-side columns. The substrate-shape columns
--    (repo_url, ref, role) carry the wire-shape state going forward.
ALTER TABLE factory_upstreams
    DROP COLUMN IF EXISTS factory_source,
    DROP COLUMN IF EXISTS factory_ref,
    DROP COLUMN IF EXISTS template_source,
    DROP COLUMN IF EXISTS template_ref;

-- 3) Drop the spec 111 / 123 family. Bindings → factory_bindings;
--    catalog → factory_artifact_substrate; audit →
--    factory_artifact_substrate_audit. Order matters when FKs are
--    present; today's schema does not enforce FKs into agent_catalog
--    from project_agent_bindings (the FK was modelled at the app layer,
--    spec 123 §4.4 I-B4), so the drop order is informational.
DROP TABLE IF EXISTS project_agent_bindings;
DROP TABLE IF EXISTS agent_catalog_audit;
DROP TABLE IF EXISTS agent_catalog;

COMMIT;
