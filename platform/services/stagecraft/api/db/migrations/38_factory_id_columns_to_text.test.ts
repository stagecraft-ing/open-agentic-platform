// Spec 142 §2.4 — migration 38 effect.
//
// Asserts the post-migration state:
//   1. The four affected columns
//      (`projects.factory_adapter_id`, `scaffold_jobs.factory_adapter_id`,
//      `factory_runs.adapter_id`, `factory_runs.process_id`) report
//      `text` from information_schema.
//   2. NOT NULL on the three NOT NULL columns is preserved.
//   3. A synthetic-string adapter id (the spec 139 runtime format) round-
//      trips through INSERT/SELECT on `scaffold_jobs.factory_adapter_id`.
//      Pre-migration this would fail with `22P02 invalid input syntax
//      for type uuid` — the live-cluster surface that motivated this
//      spec.
//
// The standard migrate.mjs runner has already applied migration 38 by
// the time this test executes, so the assertions verify the resulting
// schema rather than re-executing the SQL. ALTER COLUMN TYPE is not
// safely repeatable in test mid-flight (it acquires AccessExclusive
// and rewrites indexes) and the runner's once-per-DB gate means a
// re-execution here would not change anything anyway.

import { describe, expect, beforeAll, afterAll, it } from "vitest";
import { sql } from "drizzle-orm";
import { db } from "../drizzle";

const ORG_ID = "70142142-0000-0000-0000-000000000001";
const USER_ID = "70142142-0000-0000-0000-000000000002";
const SYNTHETIC_ADAPTER_ID = "synthetic-adapter-70142142-aim-vue-node";

type ColumnInfo = {
  data_type: string;
  is_nullable: "YES" | "NO";
} & Record<string, unknown>;

async function readColumn(
  table: string,
  column: string,
): Promise<ColumnInfo | null> {
  const result = await db.execute<ColumnInfo>(sql`
    SELECT data_type, is_nullable
      FROM information_schema.columns
     WHERE table_name = ${table}
       AND column_name = ${column}
  `);
  return result.rows[0] ?? null;
}

describe("spec 142 — migration 38 (factory id columns → text)", () => {
  beforeAll(async () => {
    await db.execute(sql`
      INSERT INTO organizations (id, name, slug)
        VALUES (${ORG_ID}, 'spec142-mig-org', 'spec142-mig-org')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO users (id, email, name, role)
        VALUES (${USER_ID}, 'spec142@test.local', 'spec142-test-user', 'user')
        ON CONFLICT (id) DO NOTHING
    `);
  });

  afterAll(async () => {
    await db.execute(sql`
      DELETE FROM scaffold_jobs WHERE org_id = ${ORG_ID}
    `);
    await db.execute(sql`DELETE FROM users WHERE id = ${USER_ID}`);
    await db.execute(sql`DELETE FROM organizations WHERE id = ${ORG_ID}`);
  });

  it("projects.factory_adapter_id is text NULL", async () => {
    const col = await readColumn("projects", "factory_adapter_id");
    expect(col?.data_type).toBe("text");
    expect(col?.is_nullable).toBe("YES");
  });

  it("scaffold_jobs.factory_adapter_id is text NOT NULL", async () => {
    const col = await readColumn("scaffold_jobs", "factory_adapter_id");
    expect(col?.data_type).toBe("text");
    expect(col?.is_nullable).toBe("NO");
  });

  it("factory_runs.adapter_id is text NOT NULL", async () => {
    const col = await readColumn("factory_runs", "adapter_id");
    expect(col?.data_type).toBe("text");
    expect(col?.is_nullable).toBe("NO");
  });

  it("factory_runs.process_id is text NOT NULL", async () => {
    const col = await readColumn("factory_runs", "process_id");
    expect(col?.data_type).toBe("text");
    expect(col?.is_nullable).toBe("NO");
  });

  it("scaffold_jobs accepts a spec-139 synthetic adapter id and round-trips it", async () => {
    const inserted = await db.execute<{ id: string; factory_adapter_id: string }>(sql`
      INSERT INTO scaffold_jobs (
        org_id, factory_adapter_id, requested_by, variant, status, step,
        github_org, repo_name
      ) VALUES (
        ${ORG_ID}, ${SYNTHETIC_ADAPTER_ID}, ${USER_ID}, 'dual',
        'running', 'repo-create', 'spec142-org', 'spec142-test-repo'
      )
      RETURNING id, factory_adapter_id
    `);
    const row = inserted.rows[0];
    expect(row?.factory_adapter_id).toBe(SYNTHETIC_ADAPTER_ID);

    const fetched = await db.execute<{ factory_adapter_id: string }>(sql`
      SELECT factory_adapter_id
        FROM scaffold_jobs
       WHERE id = ${row.id}
    `);
    expect(fetched.rows[0]?.factory_adapter_id).toBe(SYNTHETIC_ADAPTER_ID);
  });
});
