-- Spec 137 Phase 4↔5 integration — deploy descriptor secrets on the gate row.
--
-- Phase 4 (deployd-api) consumes an `AccessGateDescriptor` carrying the
-- Rauthy client secret, the oauth2-proxy cookie secret, and the TLS
-- secret name used by the tenant + gate Ingresses. Migration 40 left
-- those off the descriptor row because the original architectural
-- aspiration (plan.md §Risk register, 2026-05-06) was "schema absence" —
-- keep secrets exclusively in K8s Secrets.
--
-- The pragmatic v1 path persists them here on the descriptor row:
--   * Rauthy 0.35 admin GET does not return the client secret (T003
--     smoke; 14-field readback excludes it). The secret is only surfaced
--     once, at create/PUT time. Stagecraft needs to retain it across
--     subsequent deploy calls.
--   * The oauth2-proxy cookie secret must remain stable across deploys
--     (rotating it invalidates every session). Generating per-deploy
--     would force a fresh secret each `helm upgrade --install`.
--
-- This does NOT violate FR-007 — those are infrastructure credentials
-- (OIDC client secret, cookie-signing key), not user passwords, password
-- hashes, or upstream IdP user tokens. Plan.md risk register text is
-- amended in the same PR to reflect this pragmatic v1; KMS-backed
-- encryption-at-rest is filed as a follow-up.

BEGIN;

ALTER TABLE environment_access_gates
    ADD COLUMN rauthy_client_secret TEXT,
    ADD COLUMN cookie_secret        TEXT,
    ADD COLUMN tls_secret_name      TEXT NOT NULL DEFAULT 'tenants-wildcard-tls';

-- CHECK: an enabled gate carries the secrets it needs to render. Both
-- secrets must be NOT NULL when `enabled = true`. The existing
-- `enabled_requires_ref` constraint already enforces `rauthy_client_ref
-- IS NOT NULL` — this one extends the pattern to the deploy secrets.
ALTER TABLE environment_access_gates
    ADD CONSTRAINT environment_access_gates_enabled_requires_secrets
    CHECK (
        enabled = false
        OR (rauthy_client_secret IS NOT NULL AND cookie_secret IS NOT NULL)
    );

COMMIT;
