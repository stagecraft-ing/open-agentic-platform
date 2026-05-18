-- Spec 137 Phase 4↔5 integration — reverse migration.
--
-- Drops the deploy-descriptor secret columns and their CHECK constraint.

BEGIN;

ALTER TABLE environment_access_gates
    DROP CONSTRAINT IF EXISTS environment_access_gates_enabled_requires_secrets;

ALTER TABLE environment_access_gates
    DROP COLUMN IF EXISTS tls_secret_name,
    DROP COLUMN IF EXISTS cookie_secret,
    DROP COLUMN IF EXISTS rauthy_client_secret;

COMMIT;
