-- Spec 139 Phase 4 (T093) — narrow drop of the spec 108 bucket-blob trio.
--
-- Drops:
--   * factory_adapters     — projected at read time from
--                            factory_artifact_substrate via
--                            loadSubstrateForOrg + projectSubstrateToLegacy.
--   * factory_contracts    — same projection.
--   * factory_processes    — same projection.
--
-- Does NOT drop (deferred to Phase 4b):
--   * agent_catalog / agent_catalog_audit — still consumed by
--     `api/agents/bindings.ts` whose substrate re-point is Phase 4b.
--   * project_agent_bindings — same; bindings.ts is the last consumer.
--   * factory_upstreams.factory_source / factory_ref / template_source /
--     template_ref columns — still back the legacy singleton wire shape
--     on GET/POST /api/factory/upstreams. Column drop ships with the
--     wire-shape cutover in Phase 4b.
--
-- **Irreversibility note.** The .down.sql is best-effort schema-only:
-- it re-creates empty tables so a developer can roll back the migration
-- in dev without restoring data. Once these tables drop in prod, the row
-- contents are gone forever — Phases 1-3 reads + Phase 4's substrate
-- cutover are what made this safe. No prod data loss because the
-- substrate already carries every row that mattered.

BEGIN;

-- Drop FK constraints from dependent tables before dropping the legacy
-- ones. Spec 139 cutover: schema.ts now treats these columns as bare
-- UUIDs whose values point into factory_artifact_substrate(id) at the
-- app layer (see `projects.factoryAdapterId`, `scaffoldJobs.factoryAdapterId`,
-- `factoryRuns.adapterId`, `factoryRuns.processId` — none carry
-- `.references()` after the cutover). The columns themselves survive;
-- only the FK constraint goes.
--
-- IF EXISTS guards keep the migration idempotent and tolerate Postgres
-- auto-naming differences across environments.
ALTER TABLE projects
    DROP CONSTRAINT IF EXISTS projects_factory_adapter_id_fkey;

ALTER TABLE scaffold_jobs
    DROP CONSTRAINT IF EXISTS scaffold_jobs_factory_adapter_id_fkey;

ALTER TABLE factory_runs
    DROP CONSTRAINT IF EXISTS factory_runs_adapter_id_fkey;

ALTER TABLE factory_runs
    DROP CONSTRAINT IF EXISTS factory_runs_process_id_fkey;

-- factory_adapters has a unique on (org_id, name).
DROP TABLE IF EXISTS factory_adapters;

-- factory_contracts has a unique on (org_id, name, version); no FKs.
DROP TABLE IF EXISTS factory_contracts;

-- factory_processes has a unique on (org_id, name, version).
DROP TABLE IF EXISTS factory_processes;

COMMIT;
