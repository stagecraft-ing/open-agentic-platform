-- Rows carrying the spec 198 FR-013 actions must go before the narrow
-- constraint can be restored.
DELETE FROM factory_artifact_substrate_audit
    WHERE action IN (
        'artifact.override_gate_rejected',
        'artifact.override_verified'
    );
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
            'artifact.forked'
        ));
