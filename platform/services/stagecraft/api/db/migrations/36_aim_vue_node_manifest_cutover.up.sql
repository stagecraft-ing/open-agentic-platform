-- Spec 140 §2.4 — backfill synthetic oap-self adapter-manifest row per
-- org so existing substrate emits aim-vue-node with the §7.2
-- `scaffold_source_id` manifest shape without waiting on /factory-sync.
--
-- For every distinct `org_id` already present in
-- `factory_artifact_substrate`, insert one row at
--   (org_id, origin='oap-self', path='adapters/aim-vue-node/manifest.yaml',
--    kind='adapter-manifest', version=1)
-- carrying the canonical §7.2-compliant manifest body. The projection's
-- de-dup priority (spec 140 §2.4 / projection.ts:118 post-§2.1) prefers
-- the `oap-self` row over the template-origin synthetic, so this row
-- becomes the authoritative manifest for aim-vue-node.
--
-- **Idempotence:** safe to re-run.
--   * Substrate insert: `ON CONFLICT (org_id, origin, path, version) DO NOTHING`.
--   * Audit insert: guarded by `NOT EXISTS` against an `after->>'reason'`
--     marker so a second run does not write duplicate audit rows.
--
-- **Scope:** writes a substrate row only. Does NOT touch
-- `factory_upstream_pats`, `factory_upstreams`, or any secret store.
-- Migration 36 is a backfill, not a credential rotation.
--
-- **`orchestration_source_id` value:** spec 140 §2.1 directs that the
-- canonical Factory Agent upstream `source_id` is used; per
-- `OAP_NATIVE_ADAPTERS["aim-vue-node"].orchestrationSourceId` and spec
-- 139 §7.1 row 1 that value is `goa-software-factory`. The literal
-- `aim-vue-node-orchestration` shown in spec 140 §2.1's example block is
-- the placeholder its inline annotation already disambiguates.

BEGIN;

INSERT INTO factory_artifact_substrate (
    org_id,
    origin,
    path,
    kind,
    version,
    status,
    upstream_sha,
    upstream_body,
    content_hash,
    frontmatter,
    conflict_state
)
SELECT DISTINCT
    s.org_id,
    'oap-self'                                  AS origin,
    'adapters/aim-vue-node/manifest.yaml'       AS path,
    'adapter-manifest'                          AS kind,
    1                                            AS version,
    'active'                                     AS status,
    'oap-self/aim-vue-node/spec-140-migration-36' AS upstream_sha,
    -- Canonical §7.2 manifest body. The shape mirrors what the projection
    -- emits at read time (see `api/factory/projection.ts::buildAdapter`)
    -- so the de-dup pick is consistent regardless of which side wins.
    E'adapter:\n  name: aim-vue-node\norchestration_source_id: goa-software-factory\nscaffold_source_id: aim-vue-node-template\nscaffold_runtime: node-24\n'
                                                AS upstream_body,
    md5('spec-140-migration-36-aim-vue-node-v1') AS content_hash,
    jsonb_build_object(
        'adapter', jsonb_build_object('name', 'aim-vue-node'),
        'orchestration_source_id', 'goa-software-factory',
        'scaffold_source_id', 'aim-vue-node-template',
        'scaffold_runtime', 'node-24'
    )                                            AS frontmatter,
    'ok'                                         AS conflict_state
FROM factory_artifact_substrate s
ON CONFLICT (org_id, origin, path, version) DO NOTHING;

-- Audit row per inserted substrate row. The `after->>'reason'` marker
-- is the idempotence anchor: a re-run finds the existing audit and skips.
INSERT INTO factory_artifact_substrate_audit (
    artifact_id,
    org_id,
    action,
    actor_user_id,
    before,
    after
)
SELECT
    s.id,
    s.org_id,
    'artifact.synced',
    NULL,
    NULL,
    jsonb_build_object(
        'origin', 'oap-self',
        'path', 'adapters/aim-vue-node/manifest.yaml',
        'kind', 'adapter-manifest',
        'reason', 'spec-140-migration-36'
    )
FROM factory_artifact_substrate s
WHERE s.origin = 'oap-self'
  AND s.path = 'adapters/aim-vue-node/manifest.yaml'
  AND s.upstream_sha = 'oap-self/aim-vue-node/spec-140-migration-36'
  AND NOT EXISTS (
        SELECT 1
        FROM factory_artifact_substrate_audit a
        WHERE a.artifact_id = s.id
          AND a.action = 'artifact.synced'
          AND a.after->>'reason' = 'spec-140-migration-36'
  );

COMMIT;
