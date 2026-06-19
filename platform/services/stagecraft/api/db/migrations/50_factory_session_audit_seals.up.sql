-- Spec 207 AC-4: session-scoped audit segment countersign seals.
--
-- One row per countersigned segment head. The local side (OPC) submits a
-- segment HEAD (hash + metadata) over the duplex channel; stagecraft validates
-- and countersigns it, persisting this row. Offline-first: the local side
-- accumulates unanchored heads and submits at reconnect.
--
-- The unique constraint on (org_id, session_id, segment_id) makes the upsert
-- idempotent on re-submission (reconnect / at-least-once delivery).

CREATE TABLE factory_session_audit_seals (
  id                   UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id               UUID          NOT NULL,
  session_id           TEXT          NOT NULL,
  segment_id           TEXT          NOT NULL,
  segment_head_hash    TEXT          NOT NULL,
  segment_record_count INTEGER       NOT NULL,
  first_record_at      TIMESTAMPTZ,
  last_record_at       TIMESTAMPTZ,
  -- A row exists only once countersigned, so unanchored is false at insert.
  -- The local side tracks the unanchored state locally.
  unanchored           BOOLEAN       NOT NULL DEFAULT false,
  countersign_jws      TEXT,
  countersign_kid      TEXT,
  countersigned_at     TIMESTAMPTZ,
  created_at           TIMESTAMPTZ   NOT NULL DEFAULT now(),

  CONSTRAINT factory_session_audit_seals_org_session_segment_uniq
    UNIQUE (org_id, session_id, segment_id)
);
