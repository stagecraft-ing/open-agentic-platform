// Spec 112 §6.3 — unit tests for the pure bundle assembly helpers.
//
// DB and Encore-runtime path covered by integration testing; this file
// pins the shape-mapping invariants so OPC and the web UI can rely on a
// stable contract.

import { describe, expect, test } from "vitest";
import { buildOpcBundle, cloneUrlFor } from "./opcBundleHelpers";

const SYNCED = new Date("2026-04-22T10:00:00.000Z");

const baseProject = {
  id: "11111111-1111-1111-1111-111111111111",
  name: "Family Violence Portal",
  slug: "fv-portal",
  workspaceId: "22222222-2222-2222-2222-222222222222",
  orgId: "33333333-3333-3333-3333-333333333333",
  factoryAdapterId: "44444444-4444-4444-4444-444444444444",
};

const baseRepo = {
  githubOrg: "GovAlta-Pronghorn",
  repoName: "cfs-emergency-family-violence-services-funding-request-portal",
  defaultBranch: "main",
};

const baseAdapter = {
  id: "44444444-4444-4444-4444-444444444444",
  name: "aim-vue-node",
  version: "3.0.0",
  sourceSha: "abc123",
  syncedAt: SYNCED,
  manifest: { kind: "adapter", capabilities: { dual_stack: true } },
};

describe("buildOpcBundle", () => {
  test("composes a fully-populated bundle for an imported factory project", () => {
    const bundle = buildOpcBundle({
      project: baseProject,
      repo: baseRepo,
      adapter: baseAdapter,
      contracts: [
        {
          name: "build-spec",
          version: "1.0.0",
          sourceSha: "c1",
          syncedAt: SYNCED,
          schema: { $id: "build-spec" },
        },
      ],
      processes: [
        {
          name: "7-stage-build",
          version: "1.0.0",
          sourceSha: "p1",
          syncedAt: SYNCED,
          definition: { stages: 7 },
        },
      ],
      agents: [
        {
          id: "a1",
          name: "explorer",
          version: 2,
          contentHash: "h1",
          frontmatter: { role: "explorer" },
          bodyMarkdown: "# explorer",
        },
      ],
      cloneToken: null,
    });

    expect(bundle.project).toEqual({
      id: baseProject.id,
      name: baseProject.name,
      slug: baseProject.slug,
      workspaceId: baseProject.workspaceId,
      orgId: baseProject.orgId,
    });
    expect(bundle.repo).toEqual({
      cloneUrl:
        "https://github.com/GovAlta-Pronghorn/cfs-emergency-family-violence-services-funding-request-portal.git",
      githubOrg: baseRepo.githubOrg,
      repoName: baseRepo.repoName,
      defaultBranch: "main",
    });
    expect(bundle.deepLink).toBe(
      `opc://project/open?project_id=${baseProject.id}&url=${encodeURIComponent(
        bundle.repo!.cloneUrl
      )}`
    );
    expect(bundle.adapter).toMatchObject({
      id: baseAdapter.id,
      name: "aim-vue-node",
      syncedAt: SYNCED.toISOString(),
    });
    expect(bundle.contracts).toHaveLength(1);
    expect(bundle.contracts[0]).toMatchObject({
      name: "build-spec",
      syncedAt: SYNCED.toISOString(),
    });
    expect(bundle.processes).toHaveLength(1);
    expect(bundle.agents).toHaveLength(1);
    expect(bundle.agents[0].status).toBe("published");
  });

  test("nulls deep link and repo when no primary repo is bound", () => {
    const bundle = buildOpcBundle({
      project: baseProject,
      repo: null,
      adapter: baseAdapter,
      contracts: [],
      processes: [],
      agents: [],
      cloneToken: null,
    });

    expect(bundle.repo).toBeNull();
    expect(bundle.deepLink).toBeNull();
    expect(bundle.adapter).not.toBeNull();
    expect(bundle.cloneToken).toBeNull();
  });

  test("nulls adapter for non-factory projects, still returns catalog rows", () => {
    const bundle = buildOpcBundle({
      project: { ...baseProject, factoryAdapterId: null },
      repo: baseRepo,
      adapter: null,
      contracts: [
        {
          name: "build-spec",
          version: "1.0.0",
          sourceSha: "c1",
          syncedAt: SYNCED,
          schema: {},
        },
      ],
      processes: [],
      agents: [],
      cloneToken: null,
    });

    expect(bundle.adapter).toBeNull();
    expect(bundle.deepLink).not.toBeNull();
    expect(bundle.contracts).toHaveLength(1);
  });

  test("ISO-8601 stringification is consistent across resource families", () => {
    const bundle = buildOpcBundle({
      project: baseProject,
      repo: baseRepo,
      adapter: baseAdapter,
      contracts: [
        {
          name: "build-spec",
          version: "1.0.0",
          sourceSha: "c1",
          syncedAt: SYNCED,
          schema: {},
        },
      ],
      processes: [
        {
          name: "7-stage-build",
          version: "1.0.0",
          sourceSha: "p1",
          syncedAt: SYNCED,
          definition: {},
        },
      ],
      agents: [],
      cloneToken: null,
    });

    expect(bundle.adapter!.syncedAt).toBe(SYNCED.toISOString());
    expect(bundle.contracts[0].syncedAt).toBe(SYNCED.toISOString());
    expect(bundle.processes[0].syncedAt).toBe(SYNCED.toISOString());
  });

  test("propagates a github_installation clone token through the bundle", () => {
    const bundle = buildOpcBundle({
      project: baseProject,
      repo: baseRepo,
      adapter: baseAdapter,
      contracts: [],
      processes: [],
      agents: [],
      cloneToken: {
        value: "ghs_FAKE_INSTALL_TOKEN",
        source: "github_installation",
        expiresAt: "2026-04-22T11:00:00.000Z",
      },
    });

    expect(bundle.cloneToken).toEqual({
      value: "ghs_FAKE_INSTALL_TOKEN",
      source: "github_installation",
      expiresAt: "2026-04-22T11:00:00.000Z",
    });
  });

  test("propagates a project_github_pat clone token with null expiry", () => {
    const bundle = buildOpcBundle({
      project: baseProject,
      repo: baseRepo,
      adapter: baseAdapter,
      contracts: [],
      processes: [],
      agents: [],
      cloneToken: {
        value: "ghp_FAKE_PAT",
        source: "project_github_pat",
        expiresAt: null,
      },
    });

    expect(bundle.cloneToken?.source).toBe("project_github_pat");
    expect(bundle.cloneToken?.expiresAt).toBeNull();
  });
});

describe("cloneUrlFor", () => {
  test("returns canonical https github clone URL with .git suffix", () => {
    expect(cloneUrlFor("acme", "foo")).toBe("https://github.com/acme/foo.git");
  });
});
