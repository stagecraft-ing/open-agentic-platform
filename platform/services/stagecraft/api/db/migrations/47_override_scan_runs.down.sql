-- Rows carrying the spec 200 FR-008 scan actions must go before the
-- narrower constraint can be restored.
DELETE FROM factory_artifact_substrate_audit
    WHERE action IN (
        'artifact.scan_flagged',
        'artifact.scan_clean',
        'artifact.scan_skipped',
        'artifact.scan_failed'
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
            'artifact.forked',
            'artifact.override_gate_rejected',
            'artifact.override_verified'
        ));

DROP TABLE factory_override_scan_runs;
