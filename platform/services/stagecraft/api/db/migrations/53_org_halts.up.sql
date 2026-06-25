-- Spec 208 FR-001/FR-004: org-wide agent kill-switch quarantine record.
-- One row per halt scope (org | project | agent-profile). Consulted
-- fail-closed at run-grant issuance/renewal, new-session registration, and
-- serve/bind (the spec 198 FR-010 revocation check sites the switch reuses
-- rather than adding a parallel mechanism). A scope is "active" exactly when
-- it has no non-'lifted' row.
--
-- pulled_by / lifted_by store the human actor uuid bare, matching the
-- factory_revocations.actor convention; no SQL FK to users.id (the factory
-- tables omit it, and a service identity can never reach the write path: the
-- factory:configure + human-actor gate enforces that at the API boundary).
-- FIPS note: no md5() here (the stagecraft migration rule); no hashing at all.
CREATE TABLE org_halts (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id      UUID NOT NULL,
    scope       TEXT NOT NULL
        CONSTRAINT org_halts_scope_chk
        CHECK (scope IN ('org', 'project', 'agent-profile')),
    scope_key   TEXT NOT NULL,
    state       TEXT NOT NULL DEFAULT 'halted'
        CONSTRAINT org_halts_state_chk
        CHECK (state IN ('halted', 'reintegrating', 'lifted')),
    reason      TEXT NOT NULL,
    pulled_by   UUID NOT NULL,
    pulled_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    lifted_by   UUID,
    lifted_at   TIMESTAMPTZ,
    -- Per-engine acknowledgment timestamps (FR-003): each entry
    -- {clientId, ackedAt, kind:'halt'|'lift'}. Populated in Phase 2.
    acks        JSONB NOT NULL DEFAULT '[]'::jsonb
);

-- Hot-path liveness lookup AND the at-most-one-active-halt-per-scope
-- invariant in one partial UNIQUE index. The FR-001 fail-closed consult is an
-- indexed lookup on (org_id, scope, scope_key) WHERE state != 'lifted'; the
-- UNIQUE constraint stops a repeated pull from accumulating duplicate active
-- rows (which would let "lift the halt" leave the scope still halted). The
-- predicate excludes only 'lifted': a 'reintegrating' scope still refuses new
-- sessions and grants until its staged re-admission completes (FR-004), so it
-- must remain in the active set and counts toward the uniqueness constraint.
CREATE UNIQUE INDEX idx_org_halts_active
    ON org_halts (org_id, scope, scope_key)
    WHERE state != 'lifted';
