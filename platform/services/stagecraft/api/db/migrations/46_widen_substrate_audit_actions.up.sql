-- Spec 198 FR-013 correction (discovered during spec 201 phase 1): the
-- migration-32 check constraint predates FR-013 and never admitted the two
-- audit actions its writers emit — `artifact.override_gate_rejected` (a)
-- and `artifact.override_verified` (c). On any database built from these
-- migrations, every gate-refusal and verify-override audit INSERT violated
-- `factory_artifact_substrate_audit_action_chk` (the DB-bound tests
-- covering them are encore-test-gated and not part of the CI vitest run,
-- so the violation never surfaced). Widen the check to the full action
-- vocabulary `api/db/schema.ts::ArtifactAuditAction` declares.
ALTER TABLE factory_artifact_substrate_audit
    DROP CONSTRAINT factory_artifact_substrate_audit_action_chk;
ALTER TABLE factory_artifact_substrate_audit
    ADD CONSTRAINT factory_artifact_substrate_audit_action_chk
        CHECK (action IN (
            'artifact.synced',
            'artifact.retired',
            'artifact.overridden',
            'artifact.override_cleared',
            'artifact.conflict_detected',
            'artifact.conflict_resolved',
            'artifact.forked',
            'artifact.override_gate_rejected',
            'artifact.override_verified'
        ));
