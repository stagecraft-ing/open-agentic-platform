-- Spec 137 Phase 1 — reverse migration for environment access gate schema.
--
-- Drops both tables; CASCADE handles the implicit indexes and constraints.
-- The order matters: the allowlist references environment_id but not the
-- gates table directly, so either order works — keeping allowlist first
-- matches the create order in reverse.

BEGIN;

DROP TABLE IF EXISTS environment_access_gate_allowlist_emails;
DROP TABLE IF EXISTS environment_access_gates;

COMMIT;
