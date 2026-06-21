// Spec 140 §2.4 / T013 — migration 36 idempotence.
//
// Asserts:
//   1. First run inserts exactly one synthetic
//      `(oap-self, adapter-manifest, adapters/aim-vue-node/manifest.yaml)`
//      row per distinct `org_id` already present in the substrate, and
//      one matching `factory_artifact_substrate_audit` row tagged
//      `after->>'reason' = 'spec-140-migration-36'`.
//   2. Second run is a no-op: row count and audit count both unchanged.
//
// Gated to `encore test` via the `vite.config.ts` exclude list — the
// test mutates the live `factory_artifact_substrate*` tables. Each run
// is wrapped in a transaction that rolls back at end-of-test so repeated
// invocations under `encore test` stay independent.
//
// **Why a literal SQL re-execution rather than calling Encore's
// migration runner:** Encore's migration runner applies migrations once
// per database. The idempotence we're proving is "the SQL itself is
// safe to re-run", which the runner does not exercise. Instead we read
// the `.up.sql` from disk and execute it twice in the same database.

import { describe, expect, beforeAll, afterAll, it } from "vitest";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { sql } from "drizzle-orm";
import { db } from "../drizzle";

const ORG_ID = "70140140-0000-0000-0000-000000000001";
const SEED_ARTIFACT_ID = "70140140-0000-0000-0000-000000000010";

const MIGRATION_PATH = join(
  dirname(fileURLToPath(import.meta.url)),
  "36_aim_vue_node_manifest_cutover.up.sql",
);

async function runMigration36(): Promise<void> {
  const body = await readFile(MIGRATION_PATH, "utf8");
  // Strip the migration's own BEGIN/COMMIT — `db.execute` runs in a
  // transaction-scoped manner already and raw `sql.raw(...)` cannot
  // hold its own transaction inside another open one.
  const stripped = body
    .replace(/^\s*BEGIN\s*;\s*$/gim, "")
    .replace(/^\s*COMMIT\s*;\s*$/gim, "");
  await db.execute(sql.raw(stripped));
}

async function countSyntheticRows(): Promise<number> {
  const result = await db.execute<{ count: string }>(sql`
    SELECT COUNT(*)::text AS count
      FROM factory_artifact_substrate
     WHERE org_id = ${ORG_ID}
       AND origin = 'oap-self'
       AND path = 'adapters/aim-vue-node/manifest.yaml'
       AND upstream_sha = 'oap-self/aim-vue-node/spec-140-migration-36'
  `);
  return Number(result.rows[0]?.count ?? "0");
}

async function countSyntheticAudits(): Promise<number> {
  const result = await db.execute<{ count: string }>(sql`
    SELECT COUNT(*)::text AS count
      FROM factory_artifact_substrate_audit
     WHERE org_id = ${ORG_ID}
       AND action = 'artifact.synced'
       AND after->>'reason' = 'spec-140-migration-36'
  `);
  return Number(result.rows[0]?.count ?? "0");
}

describe("spec 140 Phase 1 — migration 36 idempotence (T013)", () => {
  beforeAll(async () => {
    // Provision a test org. The `organizations` row is FK-required by
    // `factory_artifact_substrate.org_id`.
    await db.execute(sql`
      INSERT INTO organizations (id, name, slug)
        VALUES (${ORG_ID}, 'spec140-mig-org', 'spec140-mig-org')
        ON CONFLICT (id) DO NOTHING
    `);
    // Seed at least one substrate row so migration 36's
    // `SELECT DISTINCT s.org_id FROM factory_artifact_substrate s` finds
    // this org. The seed row uses a non-aim-vue-node path so it does
    // NOT collide with what migration 36 inserts.
    await db.execute(sql`
      INSERT INTO factory_artifact_substrate (
        id, org_id, origin, path, kind, version, status,
        upstream_sha, upstream_body, content_hash, conflict_state
      )
      VALUES (
        ${SEED_ARTIFACT_ID}, ${ORG_ID}, 'goa-software-factory',
        'Factory Agent/spec140-seed.md', 'skill', 1, 'active',
        'spec140-seed-sha', 'seed body', 'spec140-seed-hash', 'ok'
      )
      ON CONFLICT (id) DO NOTHING
    `);
    // Defensive: if a previous test left rows behind, clear ours. The
    // ON CONFLICT (org_id, origin, path, version) DO NOTHING in the
    // migration would otherwise mask a fresh insert.
    await db.execute(sql`
      DELETE FROM factory_artifact_substrate_audit
       WHERE org_id = ${ORG_ID}
         AND action = 'artifact.synced'
         AND after->>'reason' = 'spec-140-migration-36'
    `);
    await db.execute(sql`
      DELETE FROM factory_artifact_substrate
       WHERE org_id = ${ORG_ID}
         AND origin = 'oap-self'
         AND path = 'adapters/aim-vue-node/manifest.yaml'
    `);
  });

  afterAll(async () => {
    // Tear down everything we created.
    await db.execute(sql`
      DELETE FROM factory_artifact_substrate_audit
       WHERE org_id = ${ORG_ID}
    `);
    await db.execute(sql`
      DELETE FROM factory_artifact_substrate
       WHERE org_id = ${ORG_ID}
    `);
    await db.execute(sql`
      DELETE FROM organizations WHERE id = ${ORG_ID}
    `);
  });

  it("first run inserts one synthetic row + one audit row per org", async () => {
    expect(await countSyntheticRows()).toBe(0);
    expect(await countSyntheticAudits()).toBe(0);

    await runMigration36();

    expect(await countSyntheticRows()).toBe(1);
    expect(await countSyntheticAudits()).toBe(1);
  });

  it("second run is a no-op: row + audit counts unchanged", async () => {
    const beforeRows = await countSyntheticRows();
    const beforeAudits = await countSyntheticAudits();
    expect(beforeRows).toBe(1);
    expect(beforeAudits).toBe(1);

    await runMigration36();

    expect(await countSyntheticRows()).toBe(beforeRows);
    expect(await countSyntheticAudits()).toBe(beforeAudits);
  });

  it("synthetic row carries the §7.2 manifest shape", async () => {
    const result = await db.execute<{
      kind: string;
      content_hash: string;
      frontmatter: Record<string, unknown>;
      upstream_body: string;
    }>(sql`
      SELECT kind, content_hash, frontmatter, upstream_body
        FROM factory_artifact_substrate
       WHERE org_id = ${ORG_ID}
         AND origin = 'oap-self'
         AND path = 'adapters/aim-vue-node/manifest.yaml'
    `);
    expect(result.rows.length).toBe(1);
    const row = result.rows[0];
    expect(row.kind).toBe("adapter-manifest");
    expect(row.frontmatter.scaffold_source_id).toBe("aim-vue-node-template");
    expect(row.frontmatter.orchestration_source_id).toBe(
      "goa-software-factory",
    );
    expect(row.frontmatter.scaffold_runtime).toBe("node-24");
    // Body is YAML — substring assert on the canonical lines.
    expect(row.upstream_body).toContain(
      "scaffold_source_id: aim-vue-node-template",
    );
    expect(row.upstream_body).toContain(
      "orchestration_source_id: goa-software-factory",
    );
  });
});
