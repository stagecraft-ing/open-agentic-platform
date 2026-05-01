-- T019 — Canonical-case test: ASSERTIONS.
-- Runs after migration body has applied. Caller rolls back regardless.

\set ON_ERROR_STOP on

DO $assertions$
DECLARE
    catalog_count INTEGER;
    binding_count INTEGER;
    log_count INTEGER;
    extractor_id UUID;
    audit_redirected_count INTEGER;
BEGIN
    SELECT count(*) INTO catalog_count FROM agent_catalog
     WHERE org_id = 'a1111111-1111-1111-1111-111111111111';
    IF catalog_count <> 2 THEN
        RAISE EXCEPTION 'T019 FAIL: expected 2 agent_catalog rows, got %', catalog_count;
    END IF;

    SELECT count(*) INTO binding_count FROM project_agent_bindings
     WHERE project_id IN (
         'a0000000-0000-0000-0000-00000000000a',
         'b0000000-0000-0000-0000-00000000000b',
         'c0000000-0000-0000-0000-00000000000c');
    IF binding_count <> 3 THEN
        RAISE EXCEPTION 'T019 FAIL: expected 3 bindings, got %', binding_count;
    END IF;

    SELECT id INTO extractor_id FROM agent_catalog
     WHERE org_id = 'a1111111-1111-1111-1111-111111111111'
       AND name = 'extractor';
    IF extractor_id <> 'e0000000-0000-0000-0000-00000000aaaa' THEN
        RAISE EXCEPTION 'T019 FAIL: expected canonical extractor id to be A''s (e0000000-...aaaa), got %',
            extractor_id;
    END IF;

    SELECT count(*) INTO log_count FROM agent_catalog_migration_30_log;
    IF log_count <> 1 THEN
        RAISE EXCEPTION 'T019 FAIL: expected 1 absorption log row, got %', log_count;
    END IF;

    SELECT count(*) INTO audit_redirected_count
      FROM agent_catalog_audit
     WHERE id = 'aa000000-0000-0000-0000-000000000002'
       AND agent_id = 'e0000000-0000-0000-0000-00000000aaaa';
    IF audit_redirected_count <> 1 THEN
        RAISE EXCEPTION 'T019 FAIL: B''s audit row was not re-pointed onto canonical extractor';
    END IF;

    RAISE NOTICE 'T019 PASS: 2 catalog rows, 3 bindings, 1 absorption logged, audit re-pointed';
END $assertions$;
