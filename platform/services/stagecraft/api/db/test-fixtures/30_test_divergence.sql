-- T020 — Divergence-case test for migration 30.
--
-- Fixture: 1 org, 2 projects.
--   project A authors "extractor" v1 with content_hash X
--   project B authors "extractor" v1 with content_hash Y (different content)
--
-- Expected: migration aborts with RAISE EXCEPTION listing the divergent
-- (org_id, name, version) tuple. No schema changes persist (transactional
-- rollback by the caller).
--
-- Run inside a transaction; the wrapper rolls back regardless of outcome.

\set ON_ERROR_STOP on
\echo === Loading T020 divergence fixtures ===

INSERT INTO organizations (id, name, slug, created_by)
VALUES ('d2222222-2222-2222-2222-222222222222', 'Divergence Org', 'div-org',
        '00000000-0000-0000-0000-000000000000');

INSERT INTO projects (id, org_id, name, slug, object_store_bucket, created_by)
VALUES
  ('d0000000-0000-0000-0000-00000000000a', 'd2222222-2222-2222-2222-222222222222',
   'Div A', 'div-a', 'bucket-da',
   '00000000-0000-0000-0000-000000000000'),
  ('d0000000-0000-0000-0000-00000000000b', 'd2222222-2222-2222-2222-222222222222',
   'Div B', 'div-b', 'bucket-db',
   '00000000-0000-0000-0000-000000000000');

INSERT INTO agent_catalog
    (id, project_id, name, version, status, frontmatter, body_markdown,
     content_hash, created_by)
VALUES
  ('df000000-0000-0000-0000-00000000aaaa',
   'd0000000-0000-0000-0000-00000000000a',
   'extractor', 1, 'published', '{}'::jsonb,
   '# A extractor body', 'sha256_DIVERGENT_X',
   '00000000-0000-0000-0000-000000000000'),
  ('df000000-0000-0000-0000-00000000bbbb',
   'd0000000-0000-0000-0000-00000000000b',
   'extractor', 1, 'published', '{}'::jsonb,
   '# B extractor body', 'sha256_DIVERGENT_Y',
   '00000000-0000-0000-0000-000000000000');

\echo === Divergence fixtures loaded; the migration body that follows MUST raise an exception ===
