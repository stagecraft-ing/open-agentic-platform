/**
 * Spec 123 §5.2 / T035 — bindings.ts pure-helper coverage.
 *
 * The endpoint handlers themselves call into Drizzle which is stubbed at
 * the module boundary like the rest of the stagecraft vitest suite. This
 * file keeps the test surface focused on the integrity probe and the
 * wire-shape projection, both of which exercise the spec invariants
 * directly without needing live DB transactions.
 */
import { describe, expect, test, vi } from "vitest";

const fixture = vi.hoisted(() => ({
  integrityRows: [] as unknown[],
}));

vi.mock("../db/drizzle", () => ({
  db: {
    select(_shape?: unknown) {
      const chain: {
        from: () => typeof chain;
        innerJoin: () => typeof chain;
        where: () => typeof chain;
        limit: () => Promise<unknown[]>;
        then: (resolve: (rows: unknown[]) => void) => void;
      } = {
        from() {
          return chain;
        },
        innerJoin() {
          return chain;
        },
        where() {
          return chain;
        },
        limit() {
          return Promise.resolve(fixture.integrityRows);
        },
        then(resolve) {
          resolve(fixture.integrityRows);
        },
      };
      return chain;
    },
  },
}));

vi.mock("../db/schema", () => ({
  agentCatalog: {
    id: "id",
    orgId: "org_id",
    name: "name",
    version: "version",
    status: "status",
    contentHash: "content_hash",
  },
  auditLog: {},
  projectAgentBindings: {
    id: "id",
    projectId: "project_id",
    orgAgentId: "org_agent_id",
    pinnedVersion: "pinned_version",
    pinnedContentHash: "pinned_content_hash",
  },
  projects: { id: "id", orgId: "org_id" },
}));

import { verifyBindingIntegrity } from "./bindings";

describe("verifyBindingIntegrity", () => {
  test("returns an empty list when every binding's pinned hash matches the catalog row", async () => {
    fixture.integrityRows = [
      {
        bindingId: "b1",
        projectId: "p1",
        orgAgentId: "a1",
        pinnedVersion: 2,
        recordedContentHash: "h".repeat(64),
        currentContentHash: "h".repeat(64),
        currentVersion: 2,
      },
      {
        bindingId: "b2",
        projectId: "p2",
        orgAgentId: "a2",
        pinnedVersion: 5,
        recordedContentHash: "k".repeat(64),
        currentContentHash: "k".repeat(64),
        currentVersion: 5,
      },
    ];
    const violations = await verifyBindingIntegrity();
    expect(violations).toEqual([]);
  });

  test("flags hash drift — pinned content_hash no longer matches the catalog row", async () => {
    fixture.integrityRows = [
      {
        bindingId: "b1",
        projectId: "p1",
        orgAgentId: "a1",
        pinnedVersion: 2,
        recordedContentHash: "old_hash",
        currentContentHash: "new_hash_after_undeclared_mutation",
        currentVersion: 2,
      },
    ];
    const violations = await verifyBindingIntegrity();
    expect(violations).toHaveLength(1);
    expect(violations[0]).toMatchObject({
      binding_id: "b1",
      reason: "hash_drift",
      recorded_content_hash: "old_hash",
      current_content_hash: "new_hash_after_undeclared_mutation",
    });
  });

  test("flags row_missing when the binding's pinned_version no longer matches the catalog row's version", async () => {
    fixture.integrityRows = [
      {
        bindingId: "b1",
        projectId: "p1",
        orgAgentId: "a1",
        pinnedVersion: 2,
        recordedContentHash: "h".repeat(64),
        currentContentHash: "h".repeat(64),
        currentVersion: 3, // catalog has moved past v2 somehow
      },
    ];
    const violations = await verifyBindingIntegrity();
    expect(violations).toHaveLength(1);
    expect(violations[0].reason).toBe("row_missing");
  });
});
