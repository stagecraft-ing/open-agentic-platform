import { defineConfig } from "vitest/config";
import path from "path";

// When running under `encore test`, the Encore CLI provides the native
// runtime.  When running bare `vitest` (npm test / CI), we swap in
// lightweight mocks so unit tests can execute without the runtime binary.
const hasEncoreRuntime = !!process.env.ENCORE_RUNTIME_LIB;

// Array form preserves entry order: vite matches the first `find` that
// applies, so more-specific prefixes (e.g. `~encore/auth`) MUST precede
// broader ones (`~encore`).
const bareVitestAliases = [
  {
    find: "encore.dev/api",
    replacement: path.resolve(__dirname, "./test/__mocks__/encore-api.ts"),
  },
  {
    find: "encore.dev/log",
    replacement: path.resolve(__dirname, "./test/__mocks__/encore-log.ts"),
  },
  {
    find: "encore.dev/pubsub",
    replacement: path.resolve(__dirname, "./test/__mocks__/encore-pubsub.ts"),
  },
  // `~encore/*` normally resolves into `./encore.gen/*`, which is generated
  // by the Encore CLI and git-ignored. CI runs `npm test` without that
  // directory, so stub the auth barrel before the broader `~encore` prefix.
  {
    find: "~encore/auth",
    replacement: path.resolve(__dirname, "./test/__mocks__/encore-auth.ts"),
  },
];

const encoreRootAlias = {
  find: "~encore",
  replacement: path.resolve(__dirname, "./encore.gen"),
};

export default defineConfig({
  resolve: {
    alias: hasEncoreRuntime
      ? [encoreRootAlias]
      : [...bareVitestAliases, encoreRootAlias],
  },
  test: {
    // Integration tests that require Encore service infrastructure
    // (databases, service-to-service calls) must run via `encore test`.
    exclude: [
      "**/node_modules/**",
      "**/dist/**",
      "**/check.test.ts",
      // Spec 124 — factory_runs migration assertions hit the live db
      // client and exercise FK/CHECK semantics that require Postgres.
      "**/runsMigration.test.ts",
      // Spec 124 — /api/factory/runs reservation/list/detail integration
      // tests touch agent_catalog + project_agent_bindings + factory_*
      // tables; they run under `encore test`.
      "**/factory/runs.test.ts",
      // Spec 124 — duplex handler integration tests mutate `factory_runs`
      // and `audit_log` rows; gated to `encore test` for the live DB.
      "**/factory/runDuplexHandlers.test.ts",
      // Spec 124 — runs staleness sweeper tests mutate `factory_runs`
      // and emit audit rows; same DB-bound posture as the others.
      "**/factory/runsScheduler.test.ts",
      // Spec 139 — conflict + artifacts API integration tests touch the
      // live `factory_artifacts*` tables and run only under `encore test`.
      "**/factory/conflicts.test.ts",
      "**/factory/artifacts.test.ts",
      // Spec 139 Phase 2 — agent_catalog → substrate migration dry-run +
      // dispatch / createOapNative E2E tests; live DB.
      "**/factory/agentCatalogMigration.dryrun.test.ts",
      "**/agents/dispatch.test.ts",
      "**/projects/scaffold/createOapNative.test.ts",
      // Spec 139 Phase 4b — bindings.ts substrate-direct integration
      // tests (bind / repin / unbind / retired-upstream).
      "**/agents/bindings.integration.test.ts",
      // Spec 140 Phase 1 — migration 36 idempotence test mutates
      // `factory_artifact_substrate*` tables; runs under `encore test`.
      "**/db/migrations/36_aim_vue_node_manifest_cutover.test.ts",
      // Spec 141 — migration 37 idempotence + effect test mutates the
      // same `factory_artifact_substrate*` + `factory_upstreams` tables.
      "**/db/migrations/37_aim_vue_node_canonical_source_id.test.ts",
      // Spec 140 Phase 2 — scaffold scheduler resolver test queries the
      // live `factory_upstreams` table.
      "**/projects/scaffold/scheduler.test.ts",
    ],
  },
});
