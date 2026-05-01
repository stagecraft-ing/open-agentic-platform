-- Migration 31 — factory_runs table (spec 124 §3, A-7).
--
-- Reserved slot. Phase 1 (T010..T016) fills the body. The ordering guard
-- runs first and aborts loudly if spec 123's migration 30
-- (agent_catalog_org_rescope) has not yet been applied: this spec depends
-- on `agent_catalog.org_id` and on the rescoped binding tables.
--
-- Spec 124 hard-claims slot 31; future migrations MUST NOT reuse it.

-- Body filled in Phase 1.
