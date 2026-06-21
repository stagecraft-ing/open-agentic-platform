-- Spec 140 §2.4 rollback — drop the synthetic oap-self adapter-manifest
-- rows + their audit trail. After rollback, aim-vue-node manifest is
-- emitted exclusively by the template-origin synthetic
-- (`projection.ts::buildAdapter`), which post-spec-140 still emits the
-- §7.2 manifest shape — i.e. rolling back this migration does NOT
-- restore the legacy `template_remote` field, only deletes the duplicate
-- `oap-self` row migration 36 inserted.
--
-- The legacy `template_remote` field is gone from production code
-- regardless of migration state (see spec 140 §2.1 + §2.2). Down is
-- best-effort schema-state cleanup.

BEGIN;

DELETE FROM factory_artifact_substrate_audit
 WHERE action = 'artifact.synced'
   AND after->>'reason' = 'spec-140-migration-36';

DELETE FROM factory_artifact_substrate
 WHERE origin = 'oap-self'
   AND path = 'adapters/aim-vue-node/manifest.yaml'
   AND upstream_sha = 'oap-self/aim-vue-node/spec-140-migration-36';

COMMIT;
