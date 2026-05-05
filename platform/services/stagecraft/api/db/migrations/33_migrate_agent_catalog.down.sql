-- Spec 139 Phase 2 — rollback for the agent_catalog backfill.
--
-- Removes the rows the up-migration inserted. Since up-migration §1
-- preserved `agent_catalog.id` as `factory_artifact_substrate.id`, the
-- rollback can target rows by id-equality with `agent_catalog`. The
-- `agent_catalog`, `agent_catalog_audit`, and `project_agent_bindings`
-- tables are untouched (Phase 4 drops them).

BEGIN;

-- §3 T052 reverse — drop bindings that were copied from project_agent_bindings.
DELETE FROM factory_bindings fb
 USING project_agent_bindings pab
 WHERE fb.id = pab.id;

-- §2 T051 reverse — drop audit rows that were copied from agent_catalog_audit.
DELETE FROM factory_artifact_substrate_audit fasa
 USING agent_catalog_audit aca
 WHERE fasa.id = aca.id;

-- §1 T050 reverse — drop substrate rows whose id matches an agent_catalog row
-- under origin='user-authored'. The origin filter is a defence-in-depth so we
-- never delete a substrate row that pre-existed under another origin.
DELETE FROM factory_artifact_substrate fas
 USING agent_catalog ac
 WHERE fas.id = ac.id
   AND fas.origin = 'user-authored';

COMMIT;
