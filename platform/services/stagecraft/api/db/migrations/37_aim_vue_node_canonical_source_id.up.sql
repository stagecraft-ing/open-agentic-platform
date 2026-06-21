-- Spec 141 §2.2 — align aim-vue-node `scaffold_source_id` with the
-- upstream's self-declared `template.json::templateName` ("aim-vue-node"),
-- amending spec 140 §2.1 which fixed the literal as
-- `aim-vue-node-template`.
--
-- Two effects, both idempotent:
--
--   1. UPDATE the migration-36 synthetic substrate row(s) to carry
--      `aim-vue-node` (instead of `aim-vue-node-template`) in
--      `upstream_body` (the YAML literal) and `frontmatter` (the JSONB
--      field). Filtered by current value so a re-run is a no-op.
--
--   2. INSERT a sibling `factory_upstreams` row per org keyed
--      `(org_id, source_id='aim-vue-node')`, role `'scaffold'`, copying
--      `repo_url` / `ref` / `subpath` from the existing
--      `legacy-template-mixed` row written by the legacy two-row form
--      (`upstreams.ts::upsertUpstreams`). Guarded by `ON CONFLICT
--      (org_id, source_id) DO NOTHING`.
--
-- **Migration 36 immutability:** migration 36 SQL is not edited.
-- Migration 36 continues to insert `aim-vue-node-template`; migration
-- 37 forward-migrates the value to `aim-vue-node` in the same
-- transaction sequence on every cluster. Per repo convention the SQL
-- of a once-applied migration must not change (the runner tracks
-- version only — `scripts/migrate.mjs:58-61`).
--
-- **Legacy row preservation:** the `legacy-template-mixed` row stays in
-- place. The legacy two-row UI form continues to read/write it for the
-- singleton compat path (`upstreams.ts:152-153`). Only the
-- source-id-keyed lookup uses the new sibling.

BEGIN;

-- Step 1 — rewrite the migration-36 synthetic substrate row.
UPDATE factory_artifact_substrate
   SET upstream_body = replace(
           upstream_body,
           'scaffold_source_id: aim-vue-node-template',
           'scaffold_source_id: aim-vue-node'),
       frontmatter = jsonb_set(
           frontmatter,
           '{scaffold_source_id}',
           '"aim-vue-node"'::jsonb,
           false)
 WHERE origin = 'oap-self'
   AND path = 'adapters/aim-vue-node/manifest.yaml'
   AND upstream_sha = 'oap-self/aim-vue-node/spec-140-migration-36'
   AND frontmatter->>'scaffold_source_id' = 'aim-vue-node-template';
-- `content_hash` is intentionally not updated. Migration 36 chose a
-- marker-style sha256 of `spec-140-migration-36-aim-vue-node-v1`
-- (deliberately not body-derived) so migration 37's text rewrite does
-- not invalidate it. The runtime substrate convention for new rows
-- (sha256 of body — `api/factory/substrate.ts:112`) does not retroactively
-- bind already-written rows.

-- Audit row marking the rewrite. Idempotent guard via NOT EXISTS on the
-- spec-141 reason marker.
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
    jsonb_build_object('scaffold_source_id', 'aim-vue-node-template'),
    jsonb_build_object(
        'scaffold_source_id', 'aim-vue-node',
        'reason', 'spec-141-migration-37'
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
          AND a.after->>'reason' = 'spec-141-migration-37'
  );

-- Step 2 — promote the legacy template upstream to a sibling row keyed
-- by the canonical scaffold source-id.
INSERT INTO factory_upstreams (
    org_id, source_id, role, repo_url, ref, subpath
)
SELECT
    org_id,
    'aim-vue-node' AS source_id,
    'scaffold'     AS role,
    repo_url,
    ref,
    subpath
  FROM factory_upstreams
 WHERE source_id = 'legacy-template-mixed'
ON CONFLICT (org_id, source_id) DO NOTHING;

COMMIT;
