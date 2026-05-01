-- T019 — Canonical-case test: FIXTURES ONLY.
-- Run sequence: BEGIN → this file → migration body → assertions file → ROLLBACK.

\set ON_ERROR_STOP on
\echo === Loading T019 canonical fixtures ===

INSERT INTO organizations (id, name, slug, created_by)
VALUES ('a1111111-1111-1111-1111-111111111111', 'Test Org', 'test-org',
        '00000000-0000-0000-0000-000000000000');

INSERT INTO projects (id, org_id, name, slug, object_store_bucket, created_by)
VALUES
  ('a0000000-0000-0000-0000-00000000000a', 'a1111111-1111-1111-1111-111111111111',
   'Project A', 'project-a', 'bucket-a',
   '00000000-0000-0000-0000-000000000000'),
  ('b0000000-0000-0000-0000-00000000000b', 'a1111111-1111-1111-1111-111111111111',
   'Project B', 'project-b', 'bucket-b',
   '00000000-0000-0000-0000-000000000000'),
  ('c0000000-0000-0000-0000-00000000000c', 'a1111111-1111-1111-1111-111111111111',
   'Project C', 'project-c', 'bucket-c',
   '00000000-0000-0000-0000-000000000000');

INSERT INTO agent_catalog
    (id, project_id, name, version, status, frontmatter, body_markdown,
     content_hash, created_by)
VALUES
  ('e0000000-0000-0000-0000-00000000aaaa',
   'a0000000-0000-0000-0000-00000000000a',
   'extractor', 1, 'published', '{"name":"extractor","model":"opus"}'::jsonb,
   '# extractor body', 'sha256_extractor_v1',
   '00000000-0000-0000-0000-000000000000'),
  ('e0000000-0000-0000-0000-00000000bbbb',
   'b0000000-0000-0000-0000-00000000000b',
   'extractor', 1, 'published', '{"name":"extractor","model":"opus"}'::jsonb,
   '# extractor body', 'sha256_extractor_v1',
   '00000000-0000-0000-0000-000000000000'),
  ('e0000000-0000-0000-0000-00000000cccc',
   'c0000000-0000-0000-0000-00000000000c',
   'summarizer', 1, 'published', '{"name":"summarizer","model":"sonnet"}'::jsonb,
   '# summarizer body', 'sha256_summarizer_v1',
   '00000000-0000-0000-0000-000000000000');

INSERT INTO agent_catalog_audit
    (id, agent_id, project_id, action, actor_user_id, before, after)
VALUES
  ('aa000000-0000-0000-0000-000000000001',
   'e0000000-0000-0000-0000-00000000aaaa',
   'a0000000-0000-0000-0000-00000000000a',
   'create', '00000000-0000-0000-0000-000000000000',
   NULL, '{}'::jsonb),
  ('aa000000-0000-0000-0000-000000000002',
   'e0000000-0000-0000-0000-00000000bbbb',
   'b0000000-0000-0000-0000-00000000000b',
   'create', '00000000-0000-0000-0000-000000000000',
   NULL, '{}'::jsonb),
  ('aa000000-0000-0000-0000-000000000003',
   'e0000000-0000-0000-0000-00000000cccc',
   'c0000000-0000-0000-0000-00000000000c',
   'create', '00000000-0000-0000-0000-000000000000',
   NULL, '{}'::jsonb);
