-- Reverse spec 205 Phase 0: drop the two-principal attribution columns.

ALTER TABLE audit_log
  DROP COLUMN IF EXISTS nhi_sub,
  DROP COLUMN IF EXISTS on_behalf_of;
