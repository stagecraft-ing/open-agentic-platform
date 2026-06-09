-- Spec 198 rollback: drop admission/revocation tables and restore the
-- 11-kind substrate CHECK.
DROP TABLE IF EXISTS factory_revocations;
DROP TABLE IF EXISTS factory_admissions;

ALTER TABLE factory_artifact_substrate
    DROP CONSTRAINT factory_artifact_substrate_kind_chk;
ALTER TABLE factory_artifact_substrate
    ADD CONSTRAINT factory_artifact_substrate_kind_chk
        CHECK (kind IN (
            'agent',
            'skill',
            'process-stage',
            'adapter-manifest',
            'contract-schema',
            'pattern',
            'page-type-reference',
            'sample-html',
            'reference-data',
            'invariant',
            'pipeline-orchestrator'
        ));
