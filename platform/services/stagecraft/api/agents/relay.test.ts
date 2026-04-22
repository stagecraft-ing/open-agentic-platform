/**
 * Spec 111 Phase 3 — agent catalog relay tests.
 *
 * Covers the two outbound paths:
 *   - publishAgentCatalogUpdated: builds an agent.catalog.updated envelope
 *     from a row and dispatches via the broadcast path.
 *   - sendAgentCatalogSnapshot: builds the directory (hashes only, no bodies)
 *     from currently-published rows and sends it targeted to one client.
 *
 * The inbound `agent.catalog.fetch_request` handler lives in sync/service.ts
 * (closer to handleInbound, mirroring the audit-candidate pattern) and is
 * exercised through the handleInbound flow rather than unit-tested here.
 */
import { beforeEach, describe, expect, test, vi } from "vitest";

// Stub db modules before importing the module under test — identical pattern
// to relay.test.ts in api/sync. Real DB access requires the Encore runtime.
// vi.mock factories are hoisted; use vi.hoisted for shared state that the
// factory closes over.
const fixture = vi.hoisted(() => ({ selectRows: [] as unknown[] }));

vi.mock("../db/drizzle", () => ({
  db: {
    select(_shape?: unknown) {
      const chain: {
        from: () => typeof chain;
        where: () => typeof chain;
        limit: () => Promise<unknown[]>;
        then: (resolve: (rows: unknown[]) => void) => void;
      } = {
        from() {
          return chain;
        },
        where() {
          return chain;
        },
        limit() {
          return Promise.resolve(fixture.selectRows);
        },
        then(resolve) {
          resolve(fixture.selectRows);
        },
      };
      return chain;
    },
  },
}));

vi.mock("../db/schema", () => ({
  agentCatalog: { id: "id", workspaceId: "ws", status: "status" },
}));

vi.mock("../sync/service", () => ({
  dispatchServerEvent: vi.fn(async () => ({
    eventId: "evt-stub",
    cursor: "00001",
    delivered: 2,
  })),
  sendTargetedServerEvent: vi.fn(async () => true),
}));

import {
  publishAgentCatalogUpdated,
  sendAgentCatalogSnapshot,
} from "./relay";
import {
  dispatchServerEvent,
  sendTargetedServerEvent,
} from "../sync/service";

const dispatchMock = dispatchServerEvent as unknown as ReturnType<typeof vi.fn>;
const targetedMock = sendTargetedServerEvent as unknown as ReturnType<
  typeof vi.fn
>;

function makeRow(overrides: Record<string, unknown> = {}): {
  id: string;
  workspaceId: string;
  name: string;
  version: number;
  status: string;
  frontmatter: Record<string, unknown>;
  bodyMarkdown: string;
  contentHash: string;
  createdBy: string;
  createdAt: Date;
  updatedAt: Date;
} {
  return {
    id: "a-1",
    workspaceId: "ws-1",
    name: "triage",
    version: 2,
    status: "published",
    frontmatter: { name: "triage", model: "opus" },
    bodyMarkdown: "# triage body",
    contentHash: "h".repeat(64),
    createdBy: "u-1",
    createdAt: new Date("2026-04-22T00:00:00Z"),
    updatedAt: new Date("2026-04-22T00:05:00Z"),
    ...overrides,
  };
}

describe("publishAgentCatalogUpdated", () => {
  beforeEach(() => {
    dispatchMock.mockClear();
    dispatchMock.mockResolvedValue({
      eventId: "evt-1",
      cursor: "00042",
      delivered: 3,
    });
  });

  test("dispatches an agent.catalog.updated envelope with the row payload", async () => {
    const row = makeRow();
    const result = await publishAgentCatalogUpdated(row as never);

    expect(result).toEqual({
      eventId: "evt-1",
      cursor: "00042",
      delivered: 3,
    });
    expect(dispatchMock).toHaveBeenCalledTimes(1);
    const [workspaceId, envelope] = dispatchMock.mock.calls[0];
    expect(workspaceId).toBe("ws-1");
    expect(envelope.kind).toBe("agent.catalog.updated");
    expect(envelope.agentId).toBe("a-1");
    expect(envelope.name).toBe("triage");
    expect(envelope.version).toBe(2);
    expect(envelope.status).toBe("published");
    expect(envelope.contentHash).toBe("h".repeat(64));
    expect(envelope.frontmatter).toEqual({ name: "triage", model: "opus" });
    expect(envelope.bodyMarkdown).toBe("# triage body");
    expect(envelope.updatedAt).toBe("2026-04-22T00:05:00.000Z");
  });

  test("dispatches retired rows too — absence-means-removed semantics still need the terminal event", async () => {
    const row = makeRow({ status: "retired" });
    await publishAgentCatalogUpdated(row as never);
    const [, envelope] = dispatchMock.mock.calls[0];
    expect(envelope.status).toBe("retired");
  });

  test("refuses to relay a draft — drafts never travel the wire", async () => {
    const row = makeRow({ status: "draft" });
    await expect(
      publishAgentCatalogUpdated(row as never),
    ).rejects.toThrow(/draft/);
    expect(dispatchMock).not.toHaveBeenCalled();
  });
});

describe("sendAgentCatalogSnapshot", () => {
  beforeEach(() => {
    targetedMock.mockClear();
    targetedMock.mockResolvedValue(true);
    fixture.selectRows = [];
  });

  test("builds a directory-only snapshot (no bodies) and targets the requesting client", async () => {
    fixture.selectRows = [
      {
        id: "a-1",
        name: "triage",
        version: 2,
        status: "published",
        contentHash: "a".repeat(64),
        updatedAt: new Date("2026-04-22T00:05:00Z"),
      },
      {
        id: "a-2",
        name: "review",
        version: 1,
        status: "published",
        contentHash: "b".repeat(64),
        updatedAt: new Date("2026-04-22T00:06:00Z"),
      },
    ];

    const sent = await sendAgentCatalogSnapshot("ws-1", "client-x");
    expect(sent).toBe(true);

    expect(targetedMock).toHaveBeenCalledTimes(1);
    const [workspaceId, clientId, envelope] = targetedMock.mock.calls[0];
    expect(workspaceId).toBe("ws-1");
    expect(clientId).toBe("client-x");
    expect(envelope.kind).toBe("agent.catalog.snapshot");
    expect(envelope.entries).toHaveLength(2);

    // Critical invariant (spec 111 §2.3): snapshot entries must NOT include
    // frontmatter or bodyMarkdown — the desktop pulls bodies via
    // agent.catalog.fetch_request on cache miss.
    for (const entry of envelope.entries) {
      expect(entry).not.toHaveProperty("frontmatter");
      expect(entry).not.toHaveProperty("bodyMarkdown");
      expect(entry.status).toBe("published");
      expect(entry.contentHash).toMatch(/^[a-f0-9]{64}$/);
    }

    expect(envelope.entries[0]).toEqual({
      agentId: "a-1",
      name: "triage",
      version: 2,
      status: "published",
      contentHash: "a".repeat(64),
      updatedAt: "2026-04-22T00:05:00.000Z",
    });
    expect(typeof envelope.generatedAt).toBe("string");
  });

  test("sends an empty snapshot when the workspace has no published agents", async () => {
    fixture.selectRows = [];
    const sent = await sendAgentCatalogSnapshot("ws-empty", "client-y");
    expect(sent).toBe(true);
    const [, , envelope] = targetedMock.mock.calls[0];
    expect(envelope.entries).toEqual([]);
  });

  test("returns false when the targeted client is not connected", async () => {
    fixture.selectRows = [];
    targetedMock.mockResolvedValueOnce(false);
    const sent = await sendAgentCatalogSnapshot("ws-1", "gone");
    expect(sent).toBe(false);
  });
});
