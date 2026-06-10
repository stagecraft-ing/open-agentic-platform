// Spec 112 §5.3 — scaffold-output invariant: the per-request copy hands
// gitInitAndPush a VCS-free tree. Regression for the 2026-06-09 production
// create failure: template-encore's dual generator runs `git init` per
// variant, and the embedded commit-less repos made `git add -A` fail with
// "'internal/' does not have a commit checked out".
//
// Pure-filesystem test (profile "dual" applies no extras, so the function
// never shells npm/tsx) — runs under bare vitest.

import { describe, expect, test } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync, existsSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { scaffoldFromPrebuilt } from "./perRequestScaffold";

function makeWorkspaceWithDualPrebuilt(): string {
  const ws = mkdtempSync(join(tmpdir(), "scaffold-vcs-test-"));
  const prebuilt = join(ws, "_prebuilt-dual");

  // Mirror what setup-dual-app.ts emits: two variant trees, each with an
  // embedded `git init` (.git dir, no commits), plus a root-level .git
  // (the legacy single-profile shape) and a node_modules to confirm the
  // existing exclusion still holds.
  for (const variant of ["public", "internal"]) {
    const root = join(prebuilt, variant);
    mkdirSync(join(root, ".git", "refs"), { recursive: true });
    writeFileSync(join(root, ".git", "HEAD"), "ref: refs/heads/main\n");
    mkdirSync(join(root, "apps", "api"), { recursive: true });
    writeFileSync(join(root, "apps", "api", "main.ts"), "export {};\n");
    writeFileSync(join(root, "package.json"), `{"name":"${variant}"}\n`);
    // .gitignore (a FILE starting with ".git") must survive the filter.
    writeFileSync(join(root, ".gitignore"), "node_modules\n");
  }
  mkdirSync(join(prebuilt, ".git"), { recursive: true });
  writeFileSync(join(prebuilt, ".git", "HEAD"), "ref: refs/heads/main\n");
  mkdirSync(join(prebuilt, "node_modules", "left-pad"), { recursive: true });
  writeFileSync(join(prebuilt, "node_modules", "left-pad", "index.js"), "");
  writeFileSync(join(prebuilt, "README.md"), "# dual\n");
  return ws;
}

describe("scaffoldFromPrebuilt — VCS-free output (spec 112 §5.3)", () => {
  test("strips .git at any depth, keeps real files and .gitignore", async () => {
    const ws = makeWorkspaceWithDualPrebuilt();
    const dest = join(ws, "out", "june-test");
    try {
      const result = await scaffoldFromPrebuilt({
        workspaceDir: ws,
        profile: "dual",
        selectedModules: [],
        destDir: dest,
        pipelineStateSeed: { level: "L0" },
      });
      expect(result.destDir).toBe(dest);

      // The poison: embedded per-variant repos and the root carryover.
      expect(existsSync(join(dest, ".git"))).toBe(false);
      expect(existsSync(join(dest, "public", ".git"))).toBe(false);
      expect(existsSync(join(dest, "internal", ".git"))).toBe(false);

      // The payload survives.
      expect(existsSync(join(dest, "README.md"))).toBe(true);
      expect(existsSync(join(dest, "public", "apps", "api", "main.ts"))).toBe(true);
      expect(existsSync(join(dest, "internal", "package.json"))).toBe(true);
      expect(existsSync(join(dest, "public", ".gitignore"))).toBe(true);

      // Existing exclusion unchanged.
      expect(existsSync(join(dest, "node_modules", "left-pad", "index.js"))).toBe(
        false,
      );

      // The L0 seed still lands (step 4).
      expect(existsSync(join(dest, ".factory", "pipeline-state.json"))).toBe(true);
    } finally {
      rmSync(ws, { recursive: true, force: true });
    }
  });
});
