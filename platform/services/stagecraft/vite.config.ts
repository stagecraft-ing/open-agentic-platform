import { defineConfig } from "vitest/config";
import path from "path";

// When running under `encore test`, the Encore CLI provides the native
// runtime.  When running bare `vitest` (npm test / CI), we swap in
// lightweight mocks so unit tests can execute without the runtime binary.
const hasEncoreRuntime = !!process.env.ENCORE_RUNTIME_LIB;

export default defineConfig({
  resolve: {
    alias: {
      "~encore": path.resolve(__dirname, "./encore.gen"),
    },
  },
  test: {
    ...(!hasEncoreRuntime && {
      alias: {
        "encore.dev/api": path.resolve(
          __dirname,
          "./test/__mocks__/encore-api.ts",
        ),
        "encore.dev/log": path.resolve(
          __dirname,
          "./test/__mocks__/encore-log.ts",
        ),
        "encore.dev/pubsub": path.resolve(
          __dirname,
          "./test/__mocks__/encore-pubsub.ts",
        ),
      },
      // Integration tests that require Encore service infrastructure
      // (databases, service-to-service calls) must run via `encore test`.
      exclude: ["**/node_modules/**", "**/dist/**", "**/check.test.ts"],
    }),
  },
});
