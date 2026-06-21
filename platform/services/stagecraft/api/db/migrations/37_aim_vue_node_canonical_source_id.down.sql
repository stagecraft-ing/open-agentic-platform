-- Spec 141 rollback — revert the migration-36 substrate row to the
-- pre-amendment literal `aim-vue-node-template` and drop the canonical
-- sibling `factory_upstreams` row migration 37 added.
--
-- The rollback is best-effort: it restores the pre-141 wire shape but
-- does not delete the spec-141 audit row (kept as a permanent record
-- that the rewrite happened, in case roll-forward is reapplied later).

BEGIN;

UPDATE factory_artifact_substrate
   SET upstream_body = replace(
           upstream_body,
           'scaffold_source_id: aim-vue-node',
           'scaffold_source_id: aim-vue-node-template'),
       frontmatter = jsonb_set(
           frontmatter,
           '{scaffold_source_id}',
           '"aim-vue-node-template"'::jsonb,
           false)
 WHERE origin = 'oap-self'
   AND path = 'adapters/aim-vue-node/manifest.yaml'
   AND upstream_sha = 'oap-self/aim-vue-node/spec-140-migration-36'
   AND frontmatter->>'scaffold_source_id' = 'aim-vue-node';

DELETE FROM factory_upstreams
 WHERE source_id = 'aim-vue-node'
   AND role = 'scaffold';

COMMIT;
