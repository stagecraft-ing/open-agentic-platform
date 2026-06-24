-- Spec 205 Phase 0 (FR-005): two-principal audit attribution.
--
-- Today every audit_log row attributes an action to a single human via
-- actor_user_id. When an agent acts on a human's behalf (spec 205's
-- non-human identity), the row must record BOTH principals: the NHI that
-- acted (nhi_sub) and the human it acted on behalf of (on_behalf_of).
--
-- Both columns are nullable (PD-A): every pre-NHI write stays valid with
-- both NULL, and actor_user_id keeps pointing at the human. A dyn-client
-- NHI is not a users row, so its subject can never ride actor_user_id.

ALTER TABLE audit_log
  ADD COLUMN IF NOT EXISTS nhi_sub TEXT,
  ADD COLUMN IF NOT EXISTS on_behalf_of TEXT;
