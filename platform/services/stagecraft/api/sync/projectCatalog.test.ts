// Spec 112 §7 — unit tests for the project.catalog.upsert envelope builder.

import { describe, expect, test } from "vitest";
import { buildProjectCatalogUpsert } from "./projectCatalog";

const baseMeta = {
  eventId: "00000000-0000-0000-0000-000000000001",
  correlationId: "corr-1",
  workspaceCursor: "42",
  workspaceId: "ws-1",
  issuedAt: "2026-04-23T00:00:00.000Z",
};

describe("buildProjectCatalogUpsert", () => {
  test("emits a complete upsert for a factory-adapter-bound project with a repo", () => {
    const env = buildProjectCatalogUpsert({
      project: {
        id: "proj-1",
        workspaceId: "ws-1",
        name: "My Portal",
        slug: "my-portal",
        description: "desc",
        factoryAdapterId: "adap-1",
        detectionLevel: "scaffold_only",
        updatedAt: new Date("2026-04-22T12:00:00.000Z"),
      },
      repo: { githubOrg: "acme", repoName: "my-portal", defaultBranch: "main" },
      meta: baseMeta,
    });
    expect(env.kind).toBe("project.catalog.upsert");
    expect(env.projectId).toBe("proj-1");
    expect(env.workspaceId).toBe("ws-1");
    expect(env.factoryAdapterId).toBe("adap-1");
    expect(env.detectionLevel).toBe("scaffold_only");
    expect(env.tombstone).toBe(false);
    expect(env.repo).toEqual({
      githubOrg: "acme",
      repoName: "my-portal",
      defaultBranch: "main",
      cloneUrl: "https://github.com/acme/my-portal.git",
      htmlUrl: "https://github.com/acme/my-portal",
    });
    expect(env.oapDeepLink).toContain("oap://project/open");
    expect(env.oapDeepLink).toContain("project_id=proj-1");
    expect(env.oapDeepLink).toContain("level=scaffold_only");
    expect(env.updatedAt).toBe("2026-04-22T12:00:00.000Z");
  });

  test("propagates tombstone on deletion and omits detection level when unset", () => {
    const env = buildProjectCatalogUpsert({
      project: {
        id: "proj-2",
        workspaceId: "ws-1",
        name: "Closed",
        slug: "closed",
        description: "",
        factoryAdapterId: null,
        detectionLevel: null,
        updatedAt: "2026-04-21T00:00:00.000Z",
      },
      repo: null,
      meta: baseMeta,
      tombstone: true,
    });
    expect(env.tombstone).toBe(true);
    expect(env.repo).toBeNull();
    expect(env.detectionLevel).toBeNull();
    // Deep link should not carry level=not_factory or level=null.
    expect(env.oapDeepLink).not.toMatch(/level=/);
  });

  test("string updatedAt is passed through unchanged", () => {
    const env = buildProjectCatalogUpsert({
      project: {
        id: "proj-3",
        workspaceId: "ws-1",
        name: "p",
        slug: "p",
        description: "",
        factoryAdapterId: null,
        detectionLevel: "acp_produced",
        updatedAt: "2026-04-23T10:00:00Z",
      },
      repo: { githubOrg: "acme", repoName: "p", defaultBranch: "main" },
      meta: baseMeta,
    });
    expect(env.updatedAt).toBe("2026-04-23T10:00:00Z");
  });
});
