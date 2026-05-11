-- Spec 143 §12 FU-019 down-migration.
--
-- PostgreSQL cannot drop a value from an enum without a full type
-- rebuild (CREATE TYPE ... AS ENUM, swap, drop). Rebuilding here would
-- silently demote any rows already in `unsupported_type` and is not
-- worth the operational risk for a forward-only taxonomy extension.
-- Intentional no-op: rolling back this migration leaves the enum value
-- in place. Any consumer that needs the previous shape should ignore
-- `unsupported_type` rows or rebuild the enum out-of-band.

SELECT 1;
