// Spec 163 FR-002 / spec 103 — registry-consumer subprocess wrapper.
//
// The reader is the only path through which stagecraft consumes the
// registry. These tests exercise it against the OAP repo's own
// .derived/spec-registry/registry.json — the most readily available
// real fixture. The path is resolved relative to this file so the
// test does not depend on cwd.

import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "vitest";
import {
  getSpecDetail,
  getSpecRelationships,
  listSpecs,
  parseRelationshipsText,
} from "./registryReader";

const __dirname = dirname(fileURLToPath(import.meta.url));
// platform/services/stagecraft/api/specRegistry/ → repo root is 5 levels up.
const REPO_ROOT = resolve(__dirname, "..", "..", "..", "..", "..");
const REGISTRY_PATH = resolve(
  REPO_ROOT,
  ".derived/spec-registry/registry.json"
);
const BINARY_PATH = resolve(
  REPO_ROOT,
  "tools/spec-spine/registry-consumer/target/release/registry-consumer"
);

const haveFixture =
  existsSync(REGISTRY_PATH) && existsSync(BINARY_PATH);

describe("parseRelationshipsText (pure parser)", () => {
  test("parses outgoing + incoming spec-bearing edges", () => {
    const text = [
      "Relationships for: 163-stagecraft-requirements-view",
      "",
      "Outgoing (2):",
      "  extends → 087-unified-workspace-architecture",
      "  refines → 130-spec-coupling-primary-owner",
      "",
      "Incoming (1):",
      "  amends ← 200-some-amender",
    ].join("\n");
    const r = parseRelationshipsText("163-stagecraft-requirements-view", text);
    expect(r.outgoing).toEqual([
      { kind: "extends", otherSpec: "087-unified-workspace-architecture" },
      { kind: "refines", otherSpec: "130-spec-coupling-primary-owner" },
    ]);
    expect(r.incoming).toEqual([
      { kind: "amends", otherSpec: "200-some-amender" },
    ]);
  });

  test("handles empty (none) blocks", () => {
    const text = [
      "Relationships for: x",
      "",
      "Outgoing (0):",
      "  (none)",
      "",
      "Incoming (0):",
      "  (none)",
    ].join("\n");
    const r = parseRelationshipsText("x", text);
    expect(r.outgoing).toEqual([]);
    expect(r.incoming).toEqual([]);
  });

  test("skips path-only outgoing edges (e.g. constrains with no other spec)", () => {
    // Mirrors the real format emitted by `print_relationships_human`
    // for a constrains edge that lists paths but no spec on the far end.
    const text = [
      "Outgoing (2):",
      "  extends → 001-spec-compiler-mvp",
      "  constrains",
    ].join("\n");
    const r = parseRelationshipsText("y", text);
    expect(r.outgoing).toEqual([
      { kind: "extends", otherSpec: "001-spec-compiler-mvp" },
    ]);
  });

  test("strips trailing [paths] attribute on outgoing edges", () => {
    const text = [
      "Outgoing (1):",
      "  extends → 087-unified-workspace-architecture [platform/services/stagecraft/web/app/routes]",
    ].join("\n");
    const r = parseRelationshipsText("z", text);
    expect(r.outgoing).toEqual([
      { kind: "extends", otherSpec: "087-unified-workspace-architecture" },
    ]);
  });
});

describe.skipIf(!haveFixture)("registryReader against OAP's own registry", () => {
  test("listSpecs returns sorted typed rows (FR-001, FR-002)", async () => {
    const specs = await listSpecs(REGISTRY_PATH, { binaryPath: BINARY_PATH });
    expect(specs.length).toBeGreaterThan(150);
    // Sorted by id.
    for (let i = 1; i < specs.length; i++) {
      expect(specs[i - 1].id.localeCompare(specs[i].id)).toBeLessThanOrEqual(0);
    }
    const bootstrap = specs.find((s) => s.id === "000-bootstrap-spec-system");
    expect(bootstrap).toBeDefined();
    expect(bootstrap!.status).toBe("approved");
    expect(bootstrap!.title).toBeTruthy();
  });

  test("getSpecDetail attaches the markdown body (FR-006)", async () => {
    const detail = await getSpecDetail(
      "163-stagecraft-requirements-view",
      REGISTRY_PATH,
      REPO_ROOT,
      { binaryPath: BINARY_PATH }
    );
    expect(detail.id).toBe("163-stagecraft-requirements-view");
    expect(detail.body).toContain("Spec-spine Requirements view");
    // Frontmatter must be stripped — the body must not start with `---`.
    expect(detail.body.startsWith("---")).toBe(false);
    // Spec 163's references include a pair-spec edge.
    expect(detail.references.some((r) => r.role === "pair-spec")).toBe(true);
  });

  test("getSpecRelationships parses the human surface (FR-006)", async () => {
    const r = await getSpecRelationships(
      "163-stagecraft-requirements-view",
      REGISTRY_PATH,
      { binaryPath: BINARY_PATH }
    );
    expect(r.id).toBe("163-stagecraft-requirements-view");
    // Spec 163 extends 087.
    expect(
      r.outgoing.some(
        (e) =>
          e.kind === "extends" &&
          e.otherSpec === "087-unified-workspace-architecture"
      )
    ).toBe(true);
  });
});
