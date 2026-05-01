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
    ],
  },
});
