-- Spec 142 §2.1 — align column types for adapter/process IDs with the
-- spec 139 substrate cutover.
--
-- Spec 139 Phase 4 (T091) replaced the dropped factory_adapters /
-- factory_processes tables with substrate-projected adapter and
-- process records keyed by synthetic strings of the form
--   `synthetic-adapter-<orgId8>-<name>`
--   `synthetic-process-<orgId8>-<name>-<version>`
--
-- Migration 34 (drop_legacy_factory_tables) dropped the FK constraints
-- on the four dependent columns but left the column TYPE as `uuid`.
-- Every Create against the live cluster therefore fails the
-- scaffold_jobs insert with
--   invalid input syntax for type uuid: "synthetic-adapter-…"
-- surfaced to the UI as Encore's generic "an internal error occurred".
--
-- This migration aligns the column types to `text` so the synthetic
-- IDs the runtime emits round-trip cleanly. Pre-spec-139 UUID rows
-- cast losslessly to their canonical text representation under
-- `USING <col>::text`. NOT NULL is preserved by ALTER COLUMN TYPE.
-- Indexes (if any) are rebuilt automatically.

BEGIN;

ALTER TABLE projects
    ALTER COLUMN factory_adapter_id TYPE text USING factory_adapter_id::text;

ALTER TABLE scaffold_jobs
    ALTER COLUMN factory_adapter_id TYPE text USING factory_adapter_id::text;

ALTER TABLE factory_runs
    ALTER COLUMN adapter_id TYPE text USING adapter_id::text;

ALTER TABLE factory_runs
    ALTER COLUMN process_id TYPE text USING process_id::text;

COMMIT;
