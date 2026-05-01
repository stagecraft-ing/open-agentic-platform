-- Reverse migration 31 — drops factory_runs.
--
-- Spec 124 §3, T015. CASCADE is intentional: the only inbound FKs are the
-- ones declared inside this table (org/project/users/factory_adapters/
-- factory_processes), so CASCADE here only affects the table itself.
-- All test rows are lost on reversal — there is no other consumer of
-- factory_runs that needs warning.

DROP INDEX IF EXISTS factory_runs_org_client_run_id_uniq;
DROP INDEX IF EXISTS factory_runs_org_in_flight_idx;
DROP INDEX IF EXISTS factory_runs_org_started_at_idx;

DROP TABLE IF EXISTS factory_runs CASCADE;
