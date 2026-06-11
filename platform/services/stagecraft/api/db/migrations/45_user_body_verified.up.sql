-- Spec 198 FR-013(c) — override trust class: the verified flag.
--
-- An override (`user_body`) is `unverified` until a privileged human
-- verifies it; every new override revision resets the flag. Runs whose
-- admitted envelope declares `overrides.require_verified: true` refuse
-- unverified override content fail-closed at bundle assembly (AC-11).
ALTER TABLE factory_artifact_substrate
    ADD COLUMN user_body_verified BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE factory_artifact_substrate
    ADD COLUMN verified_by UUID;
ALTER TABLE factory_artifact_substrate
    ADD COLUMN verified_at TIMESTAMPTZ;
