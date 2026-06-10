// Spec 124 §4 / §6.1 — audit action constants for the factory_runs lifecycle.
//
// `audit_log.action` is a free-text column (api/db/schema.ts). Following the
// spec 115 `auditActions.ts` pattern: centralise the literals so renames stay
// grep-able and dashboards keying on these strings can't drift from the
// writers.
//
// Actor convention:
//   - `factory.run.reserved` / `.completed` / `.failed` / `.cancelled` carry
//     the user that triggered the run (or initiated the cancel) as actor.
//   - `factory.run.swept` carries the system user (spec 119 seed migration
//     `2_seed_system_user`) — the sweeper is a server-side cron, not a user
//     action.

export const FACTORY_RUN_RESERVED = "factory.run.reserved" as const;
export const FACTORY_RUN_COMPLETED = "factory.run.completed" as const;
export const FACTORY_RUN_FAILED = "factory.run.failed" as const;
export const FACTORY_RUN_CANCELLED = "factory.run.cancelled" as const;
export const FACTORY_RUN_SWEPT = "factory.run.swept" as const;
// Spec 198 FR-005/FR-014 — run-grant + countersign lifecycle. Grant
// refusals are governance evidence (goal-shift / revocation propagation,
// ASI01 m4/m7), recorded with the refusal reason in metadata.
export const FACTORY_RUN_GRANT_ISSUED = "factory.run.grant_issued" as const;
export const FACTORY_RUN_GRANT_REFUSED = "factory.run.grant_refused" as const;
export const FACTORY_RUN_COUNTERSIGNED = "factory.run.countersigned" as const;

export type FactoryRunAuditAction =
  | typeof FACTORY_RUN_RESERVED
  | typeof FACTORY_RUN_COMPLETED
  | typeof FACTORY_RUN_FAILED
  | typeof FACTORY_RUN_CANCELLED
  | typeof FACTORY_RUN_SWEPT
  | typeof FACTORY_RUN_GRANT_ISSUED
  | typeof FACTORY_RUN_GRANT_REFUSED
  | typeof FACTORY_RUN_COUNTERSIGNED;
