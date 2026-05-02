// Spec 124 §4 / T021 + T023 — `/api/factory/runs` integration tests.
//
// Covers the four canonical reservation cases plus list/detail behaviour:
//   * happy-path reservation populates a row with `source_shas.agents[]`
//   * idempotent replay with the same `clientRunId` returns the existing
//     row (and a hot loop produces exactly one row)
//   * foreign-org GETs are 404
//   * a project binding pinned to a retired catalog row rejects the
//     reservation (412 / failedPrecondition)
//   * list pagination + filters return the expected slice
//
// These tests touch the live database; they are EXCLUDED from `npm test`
// (vite.config.ts) and run only under `encore test`. See the
// `runsMigration.test.ts` neighbour for the same exclusion mechanic.

import { describe, expect, it, beforeAll, afterAll } from "vitest";
import { sql } from "drizzle-orm";
import { db } from "../db/drizzle";
import { reserveRunCore, listRunsCore, getRunCore } from "./runs";
import { APIError } from "encore.dev/api";

// Per-test fixture ids. Distinct from the runsMigration.test.ts fixtures
// to avoid cross-test seeding collisions when both run under one invocation.
const ORG_ID = "22222222-0000-0000-0000-0000000000a1";
const USER_ID = "22222222-0000-0000-0000-0000000000a2";
const PROJECT_ID = "22222222-0000-0000-0000-0000000000a3";
const ADAPTER_ID = "22222222-0000-0000-0000-0000000000a4";
const PROCESS_ID = "22222222-0000-0000-0000-0000000000a5";
const AGENT_ID = "22222222-0000-0000-0000-0000000000a6";
const RETIRED_AGENT_ID = "22222222-0000-0000-0000-0000000000a7";
const FOREIGN_ORG = "33333333-0000-0000-0000-0000000000a1";

// The reservation/list/detail Core functions take auth explicitly so the
// integration test does not need to wire the `~encore/auth` runtime.
// The thin api() wrappers in `runs.ts` are exercised end-to-end by the
// desktop integration test in spec 124 Phase 5.
const ORG_CTX = { orgId: ORG_ID, userID: USER_ID };
const FOREIGN_CTX = { orgId: FOREIGN_ORG, userID: USER_ID };

describe("spec 124 — /api/factory/runs", () => {
  beforeAll(async () => {
    await db.execute(sql`
      INSERT INTO organizations (id, name, slug)
        VALUES (${ORG_ID}, 'spec124-runs-org', 'spec124-runs-org')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO organizations (id, name, slug)
        VALUES (${FOREIGN_ORG}, 'spec124-foreign-org', 'spec124-foreign-org')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO users (id, email, password_hash, name, role)
        VALUES (${USER_ID}, 'spec124-runs@test', 'x', 'Runs Tester', 'user')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO projects (id, org_id, name, slug, description, object_store_bucket, created_by)
        VALUES (${PROJECT_ID}, ${ORG_ID}, 'spec124-rp', 'spec124-rp', '', 'bucket-spec124-runs', ${USER_ID})
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO factory_adapters (id, org_id, name, version, manifest, source_sha)
        VALUES (${ADAPTER_ID}, ${ORG_ID}, 'spec124-rest', 'v1', '{}'::jsonb, 'ada-sha-runs')
        ON CONFLICT (id) DO NOTHING
    `);
    // Process body references one agent via by_name_latest. The agent is
    // resolved at reservation time.
    await db.execute(sql`
      INSERT INTO factory_processes (id, org_id, name, version, definition, source_sha)
        VALUES (${PROCESS_ID}, ${ORG_ID}, 'spec124-rp-process', 'v1',
                '{"stages":[{"id":"s0","agent_ref":{"by_name_latest":{"name":"extract"}}}]}'::jsonb,
                'proc-sha-runs')
        ON CONFLICT (id) DO NOTHING
    `);
    // Published agent for happy-path resolution.
    await db.execute(sql`
      INSERT INTO agent_catalog (id, org_id, name, version, status, frontmatter, body_markdown, content_hash, created_by)
        VALUES (${AGENT_ID}, ${ORG_ID}, 'extract', 1, 'published', '{}'::jsonb, '# extract', 'agent-hash-1', ${USER_ID})
        ON CONFLICT (id) DO NOTHING
    `);
    // Retired agent for the rejection test (kept under a different name).
    await db.execute(sql`
      INSERT INTO agent_catalog (id, org_id, name, version, status, frontmatter, body_markdown, content_hash, created_by)
        VALUES (${RETIRED_AGENT_ID}, ${ORG_ID}, 'retired-trigger', 1, 'retired', '{}'::jsonb, '# retired', 'agent-hash-retired', ${USER_ID})
        ON CONFLICT (id) DO NOTHING
    `);
    // Bind the published agent to the project. Phase resolves
    // `by_name_latest` against this binding, not against the catalog directly.
    await db.execute(sql`
      INSERT INTO project_agent_bindings (project_id, org_agent_id, pinned_version, pinned_content_hash, bound_by)
        VALUES (${PROJECT_ID}, ${AGENT_ID}, 1, 'agent-hash-1', ${USER_ID})
        ON CONFLICT (project_id, org_agent_id) DO NOTHING
    `);
  });

  afterAll(async () => {
    await db.execute(sql`DELETE FROM factory_runs WHERE org_id = ${ORG_ID} OR org_id = ${FOREIGN_ORG}`);
    await db.execute(sql`DELETE FROM project_agent_bindings WHERE project_id = ${PROJECT_ID}`);
    await db.execute(sql`DELETE FROM agent_catalog WHERE id IN (${AGENT_ID}, ${RETIRED_AGENT_ID})`);
    await db.execute(sql`DELETE FROM projects WHERE id = ${PROJECT_ID}`);
    await db.execute(sql`DELETE FROM factory_processes WHERE id = ${PROCESS_ID}`);
    await db.execute(sql`DELETE FROM factory_adapters WHERE id = ${ADAPTER_ID}`);
    await db.execute(sql`DELETE FROM users WHERE id = ${USER_ID}`);
    await db.execute(sql`DELETE FROM organizations WHERE id IN (${ORG_ID}, ${FOREIGN_ORG})`);
  });

  it("reserves a new run, populates source_shas.agents, returns reserved=true", async () => {
    const result = await reserveRunCore(
      {
        adapterName: "spec124-rest",
        processName: "spec124-rp-process",
        projectId: PROJECT_ID,
        clientRunId: "happy-path-1",
      },
      ORG_CTX,
    );
    expect(result.reserved).toBe(true);
    expect(result.runId).toBeDefined();
    expect(result.sourceShas.adapter).toBe("ada-sha-runs");
    expect(result.sourceShas.process).toBe("proc-sha-runs");
    expect(result.sourceShas.agents).toEqual([
      { orgAgentId: AGENT_ID, version: 1, contentHash: "agent-hash-1" },
    ]);
  });

  it("idempotent replay returns the same runId without creating a second row", async () => {
    const first = await reserveRunCore(
      {
        adapterName: "spec124-rest",
        processName: "spec124-rp-process",
        projectId: PROJECT_ID,
        clientRunId: "idempotent-1",
      },
      ORG_CTX,
    );
    const second = await reserveRunCore(
      {
        adapterName: "spec124-rest",
        processName: "spec124-rp-process",
        projectId: PROJECT_ID,
        clientRunId: "idempotent-1",
      },
      ORG_CTX,
    );
    expect(first.reserved).toBe(true);
    expect(second.reserved).toBe(false);
    expect(second.runId).toBe(first.runId);
    // Confirm the DB has exactly one row.
    const rows = await db.execute(sql`
      SELECT count(*) AS c FROM factory_runs WHERE org_id = ${ORG_ID} AND client_run_id = 'idempotent-1'
    `);
    expect(Number((rows.rows[0] as { c: string | number }).c)).toBe(1);
  });

  it("hot-loop concurrent reservations produce exactly one row (T023)", async () => {
    // Fire the same (orgId, clientRunId) ten times in parallel; the
    // (org_id, client_run_id) unique index plus ON CONFLICT DO NOTHING
    // ensures only one row is created.
    const results = await Promise.all(
      Array.from({ length: 10 }, () =>
        reserveRunCore(
          {
            adapterName: "spec124-rest",
            processName: "spec124-rp-process",
            projectId: PROJECT_ID,
            clientRunId: "hot-loop-1",
          },
          ORG_CTX,
        ),
      ),
    );
    const ids = new Set(results.map((r) => r.runId));
    expect(ids.size).toBe(1);
    const rows = await db.execute(sql`
      SELECT count(*) AS c FROM factory_runs WHERE org_id = ${ORG_ID} AND client_run_id = 'hot-loop-1'
    `);
    expect(Number((rows.rows[0] as { c: string | number }).c)).toBe(1);
  });

  it("rejects reservation when the project's binding points at a retired catalog row", async () => {
    // Re-bind the project to the retired agent under the binding name
    // the process expects. We swap bindings under a unique-fixture
    // project so the happy-path tests above don't see this state.
    const altProject = "22222222-0000-0000-0000-0000000000b9";
    await db.execute(sql`
      INSERT INTO projects (id, org_id, name, slug, description, object_store_bucket, created_by)
        VALUES (${altProject}, ${ORG_ID}, 'spec124-retired-p', 'spec124-retired-p', '', 'bucket-retired', ${USER_ID})
        ON CONFLICT (id) DO NOTHING
    `);
    // Bind the project to the *retired* row under the name the process
    // looks up (`extract`) by reusing the same agent name with status=retired.
    const retiredNamedAgent = "22222222-0000-0000-0000-0000000000ba";
    await db.execute(sql`
      INSERT INTO agent_catalog (id, org_id, name, version, status, frontmatter, body_markdown, content_hash, created_by)
        VALUES (${retiredNamedAgent}, ${ORG_ID}, 'extract', 9, 'retired', '{}'::jsonb, '# retired-extract', 'agent-hash-retired-named', ${USER_ID})
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO project_agent_bindings (project_id, org_agent_id, pinned_version, pinned_content_hash, bound_by)
        VALUES (${altProject}, ${retiredNamedAgent}, 9, 'agent-hash-retired-named', ${USER_ID})
        ON CONFLICT (project_id, org_agent_id) DO NOTHING
    `);

    await expect(
      reserveRunCore(
        {
          adapterName: "spec124-rest",
          processName: "spec124-rp-process",
          projectId: altProject,
          clientRunId: "retired-1",
        },
        ORG_CTX,
      ),
    ).rejects.toThrow(/retired/i);

    // Cleanup the alt fixture.
    await db.execute(sql`DELETE FROM project_agent_bindings WHERE project_id = ${altProject}`);
    await db.execute(sql`DELETE FROM agent_catalog WHERE id = ${retiredNamedAgent}`);
    await db.execute(sql`DELETE FROM projects WHERE id = ${altProject}`);
  });

  it("two projects bound to the same agent record identical source_shas.agents (A-8 cross-project comparator)", async () => {
    // A-8 / spec §10: a Stage CD comparator (spec 122) needs two runs of
    // the same (adapter, process) against two different projects to record
    // identical `agents[].content_hash` arrays when both projects bind the
    // same catalog row. This test pins the comparator contract at the
    // reservation surface — anything later (resolver drift, binding
    // mutation) is owned by the consumer specs.
    const altProjectId = "22222222-0000-0000-0000-0000000000c1";
    await db.execute(sql`
      INSERT INTO projects (id, org_id, name, slug, description, object_store_bucket, created_by)
        VALUES (${altProjectId}, ${ORG_ID}, 'spec124-rp-alt', 'spec124-rp-alt', '', 'bucket-spec124-runs-alt', ${USER_ID})
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO project_agent_bindings (project_id, org_agent_id, pinned_version, pinned_content_hash, bound_by)
        VALUES (${altProjectId}, ${AGENT_ID}, 1, 'agent-hash-1', ${USER_ID})
        ON CONFLICT (project_id, org_agent_id) DO NOTHING
    `);

    const runA = await reserveRunCore(
      {
        adapterName: "spec124-rest",
        processName: "spec124-rp-process",
        projectId: PROJECT_ID,
        clientRunId: "comparator-a",
      },
      ORG_CTX,
    );
    const runB = await reserveRunCore(
      {
        adapterName: "spec124-rest",
        processName: "spec124-rp-process",
        projectId: altProjectId,
        clientRunId: "comparator-b",
      },
      ORG_CTX,
    );

    expect(runA.runId).not.toBe(runB.runId);
    const hashesA = runA.sourceShas.agents.map((a) => a.contentHash);
    const hashesB = runB.sourceShas.agents.map((a) => a.contentHash);
    expect(hashesA).toEqual(hashesB);
    expect(runA.sourceShas.agents).toEqual(runB.sourceShas.agents);

    await db.execute(sql`DELETE FROM project_agent_bindings WHERE project_id = ${altProjectId}`);
    await db.execute(sql`DELETE FROM projects WHERE id = ${altProjectId}`);
  });

  it("returns 404 when fetching a run from a foreign org", async () => {
    const created = await reserveRunCore(
      {
        adapterName: "spec124-rest",
        processName: "spec124-rp-process",
        projectId: PROJECT_ID,
        clientRunId: "foreign-org-1",
      },
      ORG_CTX,
    );
    await expect(
      getRunCore({ id: created.runId }, FOREIGN_CTX),
    ).rejects.toThrow(/run not found/i);
  });

  it("listRunsCore is org-scoped, filters by status and adapter, and paginates", async () => {
    // Create two runs in different statuses.
    const a = await reserveRunCore(
      {
        adapterName: "spec124-rest",
        processName: "spec124-rp-process",
        projectId: PROJECT_ID,
        clientRunId: "list-a",
      },
      ORG_CTX,
    );
    const b = await reserveRunCore(
      {
        adapterName: "spec124-rest",
        processName: "spec124-rp-process",
        projectId: PROJECT_ID,
        clientRunId: "list-b",
      },
      ORG_CTX,
    );
    // Flip one to running so the status filter has something to match.
    await db.execute(sql`UPDATE factory_runs SET status = 'running' WHERE id = ${a.runId}`);

    const queued = await listRunsCore(
      { status: "queued", adapter: "spec124-rest" },
      ORG_CTX,
    );
    const queuedIds = queued.runs.map((r) => r.id);
    expect(queuedIds).toContain(b.runId);
    expect(queuedIds).not.toContain(a.runId);

    const running = await listRunsCore({ status: "running" }, ORG_CTX);
    const runningIds = running.runs.map((r) => r.id);
    expect(runningIds).toContain(a.runId);
    expect(runningIds).not.toContain(b.runId);

    // Foreign org sees nothing.
    const foreign = await listRunsCore({}, FOREIGN_CTX);
    const foreignIds = new Set(foreign.runs.map((r) => r.id));
    expect(foreignIds.has(a.runId)).toBe(false);
    expect(foreignIds.has(b.runId)).toBe(false);

    // Pagination: limit=1 must produce a nextCursor.
    const page1 = await listRunsCore({ limit: 1 }, ORG_CTX);
    expect(page1.runs).toHaveLength(1);
    expect(page1.nextCursor).toBeDefined();
  });

  it("invalid `before` cursor surfaces as invalidArgument", async () => {
    await expect(
      listRunsCore({ before: "not-a-date" }, ORG_CTX),
    ).rejects.toMatchObject({ code: APIError.invalidArgument("x").code });
  });
});
