-- Spec 198 phase 4 — revert signing authority + run-grant storage.

ALTER TABLE factory_runs DROP COLUMN IF EXISTS countersigned_at;
ALTER TABLE factory_runs DROP COLUMN IF EXISTS certificate_sha256;

DROP TABLE IF EXISTS factory_run_grants;

ALTER TABLE factory_admissions DROP COLUMN IF EXISTS sealed_at;
ALTER TABLE factory_admissions DROP COLUMN IF EXISTS seal_jws;
