// Spec 139 Phase 2 — pure-functional tests for the OAP-native adapter
// ingest sanitisers (T054 + T055 / D-4 fixes).
//
// Locks the four mechanical fixes from Phase 0 D-4:
//   1. `stack.runtime` bumped to `node-24` for `next-prisma` and
//      `encore-react`; `rust-axum` keeps `native`.
//   2. Manifest gets `orchestration_source_id` / `scaffold_source_id` /
//      `scaffold_runtime` injected.
//   3. Manifest's `validation:` block dropped (canonical lives at
//      `validation/invariants.yaml`).
//   4. Pattern files without frontmatter get auto-generated
//      `id` / `adapter` / `category` keys.
//
// Pure-functional — no DB, runs under `npm test`.

import { describe, expect, test } from "vitest";
import { parse as parseYaml } from "yaml";
import {
  OAP_NATIVE_ADAPTERS,
  sanitiseForIngest,
} from "./oapNativeSanitise";

const NEXT_PRISMA = OAP_NATIVE_ADAPTERS["next-prisma"];
const RUST_AXUM = OAP_NATIVE_ADAPTERS["rust-axum"];
const ENCORE_REACT = OAP_NATIVE_ADAPTERS["encore-react"];

describe("spec 139 Phase 2 — OAP-native ingest sanitisers (T054 + T055)", () => {
  test("D-4 fix #1: bumps runtime to node-24 for next-prisma", async () => {
    const input = `
schema_version: "1.0.0"
adapter:
  name: next-prisma
stack:
  language: typescript
  runtime: node-22
validation:
  invariants: [legacy]
`;
    const out = await sanitiseForIngest({
      rel: "manifest.yaml",
      body: input,
      adapterName: "next-prisma",
      config: NEXT_PRISMA,
    });
    const parsed = parseYaml(out.body) as Record<string, unknown>;
    expect((parsed.stack as Record<string, unknown>).runtime).toBe("node-24");
  });

  test("D-4 fix #1: bumps runtime to node-24 for encore-react", async () => {
    const input = `
adapter:
  name: encore-react
stack:
  runtime: node-20
`;
    const out = await sanitiseForIngest({
      rel: "manifest.yaml",
      body: input,
      adapterName: "encore-react",
      config: ENCORE_REACT,
    });
    const parsed = parseYaml(out.body) as Record<string, unknown>;
    expect((parsed.stack as Record<string, unknown>).runtime).toBe("node-24");
  });

  test("D-4 fix #1: rust-axum keeps `native` runtime untouched", async () => {
    const input = `
adapter:
  name: rust-axum
stack:
  runtime: native
`;
    const out = await sanitiseForIngest({
      rel: "manifest.yaml",
      body: input,
      adapterName: "rust-axum",
      config: RUST_AXUM,
    });
    const parsed = parseYaml(out.body) as Record<string, unknown>;
    expect((parsed.stack as Record<string, unknown>).runtime).toBe("native");
  });

  test("T055: injects orchestration_source_id / scaffold_source_id / scaffold_runtime", async () => {
    const input = `
adapter:
  name: next-prisma
stack:
  runtime: node-22
`;
    const out = await sanitiseForIngest({
      rel: "manifest.yaml",
      body: input,
      adapterName: "next-prisma",
      config: NEXT_PRISMA,
    });
    const parsed = parseYaml(out.body) as Record<string, unknown>;
    expect(parsed.orchestration_source_id).toBe("oap-next-prisma");
    expect(parsed.scaffold_source_id).toBe("oap-next-prisma-scaffold");
    expect(parsed.scaffold_runtime).toBe("node-24");
  });

  test("D-4 fix #3: drops manifest validation block", async () => {
    const input = `
adapter:
  name: next-prisma
stack:
  runtime: node-22
validation:
  invariants:
    - legacy-rule-1
    - legacy-rule-2
`;
    const out = await sanitiseForIngest({
      rel: "manifest.yaml",
      body: input,
      adapterName: "next-prisma",
      config: NEXT_PRISMA,
    });
    const parsed = parseYaml(out.body) as Record<string, unknown>;
    expect(parsed.validation).toBeUndefined();
  });

  test("D-4 fix #4: pattern without frontmatter gets auto id/adapter/category", async () => {
    const input = "# Pattern: route handler\n\nThis is the pattern body.";
    const out = await sanitiseForIngest({
      rel: "patterns/api/route-handler.md",
      body: input,
      adapterName: "next-prisma",
      config: NEXT_PRISMA,
    });
    expect(out.frontmatter).toEqual({
      id: "next-prisma-pattern-api-route-handler",
      adapter: "next-prisma",
      category: "api",
    });
    expect(out.body.startsWith("---")).toBe(true);
    expect(out.body).toContain("This is the pattern body");
  });

  test("D-4 fix #4: pattern WITH frontmatter is preserved verbatim", async () => {
    const input = `---
id: explicit-id
custom: keep-me
---

# Pattern body`;
    const out = await sanitiseForIngest({
      rel: "patterns/api/custom.md",
      body: input,
      adapterName: "next-prisma",
      config: NEXT_PRISMA,
    });
    expect(out.frontmatter).toEqual({ id: "explicit-id", custom: "keep-me" });
    expect(out.body).toBe(input);
  });

  test("non-pattern non-manifest files pass through verbatim", async () => {
    const input = "scaffold readme content";
    const out = await sanitiseForIngest({
      rel: "scaffold/README.md",
      body: input,
      adapterName: "next-prisma",
      config: NEXT_PRISMA,
    });
    expect(out.body).toBe(input);
    expect(out.frontmatter).toBeNull();
  });

  test("category derives from the directory under patterns/", async () => {
    const cases: Array<[string, string]> = [
      ["patterns/api/foo.md", "api"],
      ["patterns/data/foo.md", "data"],
      ["patterns/ui/foo.md", "ui"],
      ["patterns/page-types/foo.md", "page-types"],
    ];
    for (const [rel, expectedCategory] of cases) {
      const out = await sanitiseForIngest({
        rel,
        body: "# body",
        adapterName: "next-prisma",
        config: NEXT_PRISMA,
      });
      expect(out.frontmatter?.category).toBe(expectedCategory);
    }
  });
});
