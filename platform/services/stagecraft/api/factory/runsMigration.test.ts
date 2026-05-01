// Spec 124 Phase 1 — migration 31 (`factory_runs`) integration test.
//
// Verifies the migration body, FK semantics, idempotency unique key, and the
// status CHECK constraint against a real Postgres. Designed to run under
// `encore test` (which provisions a per-test database with all migrations
// applied) — the assertions match what `make ci` will run.
//
// What's covered:
//   * project deletion CASCADEs to factory_runs (T016)
//   * adapter deletion is rejected by the FK (NO ACTION default)
//   * (org_id, client_run_id) is unique → reservation idempotency primitive
//   * the status CHECK rejects values outside the closed set
//
// What's NOT covered here:
//   * The ordering-guard pre-flight (T013) is exercised manually during
//     local `make dev-platform` migration runs and on the spec 124 phase-1
//     verification log; running it under `encore test` is moot because
//     Encore always applies migrations in order.

import { describe, expect, it, beforeAll, afterAll } from "vitest";
import { sql } from "drizzle-orm";
import { db } from "../db/drizzle";
import { factoryRuns } from "../db/schema";

const ORG_ID = "11111111-0000-0000-0000-0000000000a1";
const USER_ID = "11111111-0000-0000-0000-0000000000a2";
const PROJECT_ID = "11111111-0000-0000-0000-0000000000a3";
const ADAPTER_ID = "11111111-0000-0000-0000-0000000000a4";
const PROCESS_ID = "11111111-0000-0000-0000-0000000000a5";

describe("spec 124 — factory_runs migration", () => {
  beforeAll(async () => {
    // Idempotent seed — if the test database has been re-used, ON CONFLICT
    // makes the seed a no-op rather than failing the suite.
    await db.execute(sql`
      INSERT INTO organizations (id, name, slug)
        VALUES (${ORG_ID}, 'spec124-test-org', 'spec124-test-org')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO users (id, email, password_hash, name, role)
        VALUES (${USER_ID}, 'spec124@test', 'x', 'Spec124 Tester', 'user')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO projects (id, org_id, name, slug, description, object_store_bucket, created_by)
        VALUES (${PROJECT_ID}, ${ORG_ID}, 'spec124-p', 'spec124-p', '', 'b-spec124', ${USER_ID})
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO factory_adapters (id, org_id, name, version, manifest, source_sha)
        VALUES (${ADAPTER_ID}, ${ORG_ID}, 'spec124-rest', 'v1', '{}'::jsonb, 'ada-sha-spec124')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO factory_processes (id, org_id, name, version, definition, source_sha)
        VALUES (${PROCESS_ID}, ${ORG_ID}, 'spec124-rp', 'v1', '{}'::jsonb, 'proc-sha-spec124')
        ON CONFLICT (id) DO NOTHING
    `);
  });

  afterAll(async () => {
    // Best-effort cleanup. CASCADE on project_id will reach factory_runs
    // even if a test left rows behind.
    await db.execute(sql`DELETE FROM factory_runs WHERE org_id = ${ORG_ID}`);
    await db.execute(sql`DELETE FROM projects WHERE id = ${PROJECT_ID}`);
    await db.execute(sql`DELETE FROM factory_processes WHERE id = ${PROCESS_ID}`);
    await db.execute(sql`DELETE FROM factory_adapters WHERE id = ${ADAPTER_ID}`);
    await db.execute(sql`DELETE FROM users WHERE id = ${USER_ID}`);
    await db.execute(sql`DELETE FROM organizations WHERE id = ${ORG_ID}`);
  });

  it("inserts a row and projects sourceShas / stageProgress as JSON", async () => {
    const runId = "11111111-0000-0000-0000-0000000000b1";
    await db.execute(sql`
      INSERT INTO factory_runs (id, org_id, project_id, triggered_by, adapter_id, process_id, client_run_id, status, source_shas)
        VALUES (${runId}, ${ORG_ID}, ${PROJECT_ID}, ${USER_ID}, ${ADAPTER_ID}, ${PROCESS_ID}, 'cli-run-insert', 'queued',
                '{"adapter":"ada-sha-spec124","process":"proc-sha-spec124","contracts":{},"agents":[]}'::jsonb)
    `);
    const rows = await db.select().from(factoryRuns).where(sql`${factoryRuns.id} = ${runId}`);
    expect(rows).toHaveLength(1);
    expect(rows[0].status).toBe("queued");
    expect(rows[0].clientRunId).toBe("cli-run-insert");
    expect(rows[0].sourceShas).toEqual({
      adapter: "ada-sha-spec124",
      process: "proc-sha-spec124",
      contracts: {},
      agents: [],
    });
    expect(rows[0].stageProgress).toEqual([]);
  });

  it("CASCADE-deletes the run when its project is deleted", async () => {
    const runId = "11111111-0000-0000-0000-0000000000b2";
    await db.execute(sql`
      INSERT INTO factory_runs (id, org_id, project_id, triggered_by, adapter_id, process_id, client_run_id, status, source_shas)
        VALUES (${runId}, ${ORG_ID}, ${PROJECT_ID}, ${USER_ID}, ${ADAPTER_ID}, ${PROCESS_ID}, 'cli-run-cascade', 'running', '{}'::jsonb)
    `);
    // Drop the project; FK CASCADE on project_id should remove the run.
    await db.execute(sql`DELETE FROM projects WHERE id = ${PROJECT_ID}`);
    const rows = await db.select().from(factoryRuns).where(sql`${factoryRuns.id} = ${runId}`);
    expect(rows).toHaveLength(0);

    // Reseed for subsequent tests.
    await db.execute(sql`
      INSERT INTO projects (id, org_id, name, slug, description, object_store_bucket, created_by)
        VALUES (${PROJECT_ID}, ${ORG_ID}, 'spec124-p', 'spec124-p', '', 'b-spec124', ${USER_ID})
    `);
  });

  it("rejects deletion of an adapter still referenced by a run (no cascade)", async () => {
    const runId = "11111111-0000-0000-0000-0000000000b3";
    await db.execute(sql`
      INSERT INTO factory_runs (id, org_id, project_id, triggered_by, adapter_id, process_id, client_run_id, status, source_shas)
        VALUES (${runId}, ${ORG_ID}, ${PROJECT_ID}, ${USER_ID}, ${ADAPTER_ID}, ${PROCESS_ID}, 'cli-run-no-cascade', 'queued', '{}'::jsonb)
    `);
    await expect(
      db.execute(sql`DELETE FROM factory_adapters WHERE id = ${ADAPTER_ID}`),
    ).rejects.toThrow(/foreign key constraint/i);
    await db.execute(sql`DELETE FROM factory_runs WHERE id = ${runId}`);
  });

  it("treats (org_id, client_run_id) as unique — second insert with same key is rejected", async () => {
    const a = "11111111-0000-0000-0000-0000000000b4";
    const b = "11111111-0000-0000-0000-0000000000b5";
    await db.execute(sql`
      INSERT INTO factory_runs (id, org_id, project_id, triggered_by, adapter_id, process_id, client_run_id, status, source_shas)
        VALUES (${a}, ${ORG_ID}, ${PROJECT_ID}, ${USER_ID}, ${ADAPTER_ID}, ${PROCESS_ID}, 'cli-run-dup', 'queued', '{}'::jsonb)
    `);
    await expect(
      db.execute(sql`
        INSERT INTO factory_runs (id, org_id, project_id, triggered_by, adapter_id, process_id, client_run_id, status, source_shas)
          VALUES (${b}, ${ORG_ID}, ${PROJECT_ID}, ${USER_ID}, ${ADAPTER_ID}, ${PROCESS_ID}, 'cli-run-dup', 'queued', '{}'::jsonb)
      `),
    ).rejects.toThrow(/factory_runs_org_client_run_id_uniq|unique constraint/i);
    await db.execute(sql`DELETE FROM factory_runs WHERE id = ${a}`);
  });

  it("rejects status values outside the closed set", async () => {
    const runId = "11111111-0000-0000-0000-0000000000b6";
    await expect(
      db.execute(sql`
        INSERT INTO factory_runs (id, org_id, project_id, triggered_by, adapter_id, process_id, client_run_id, status, source_shas)
          VALUES (${runId}, ${ORG_ID}, ${PROJECT_ID}, ${USER_ID}, ${ADAPTER_ID}, ${PROCESS_ID}, 'cli-run-bad-status', 'in_progress', '{}'::jsonb)
      `),
    ).rejects.toThrow(/factory_runs_status_check|check constraint/i);
  });
});
