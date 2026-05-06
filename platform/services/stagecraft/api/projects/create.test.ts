// Spec 140 §2.2 / T032 — pin the failedPrecondition message wording.
//
// `create.ts`'s scaffold-source-id gate produces user-visible
// `APIError.failedPrecondition` strings that the readiness banners and
// error-toaster surface verbatim. T032 asserts the strings reference
// the new `scaffold_source_id` field and not the legacy
// `template_remote` field.
//
// A behavioural test would require seeding an org + an adapter
// manifest + a missing factory_upstreams row, which crosses into
// Encore-runtime + DB territory. The string-pinning shape below is
// proportional: it locks the message at refactor time and surfaces
// regressions without requiring an integration harness.

import { describe, expect, test } from "vitest";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const CREATE_TS = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "create.ts"),
  "utf8",
);

describe("spec 140 §2.2 — create.ts precondition messages (T032)", () => {
  test("the manifest-missing-field message names scaffold_source_id (not template_remote)", () => {
    expect(CREATE_TS).toContain(
      "has no scaffold_source_id in its manifest",
    );
    expect(CREATE_TS).not.toMatch(
      /has no template_remote in its manifest/,
    );
  });

  test("the upstream-not-resolved message names scaffold_source_id and points at /app/factory/upstreams", () => {
    expect(CREATE_TS).toContain(
      "no factory_upstreams row matches",
    );
    expect(CREATE_TS).toContain("/app/factory/upstreams");
  });

  test("create.ts no longer reads manifest.template_remote in production code", () => {
    // Comments are allowed (this is a pin against runtime reads); strip
    // single- and multi-line comments before checking.
    const code = CREATE_TS
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .replace(/(^|\s)\/\/[^\n]*/g, "");
    expect(code).not.toMatch(/manifest\.template_remote/);
    expect(code).not.toMatch(/manifest\.template_default_branch/);
  });
});
