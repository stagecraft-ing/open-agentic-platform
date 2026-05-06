// Spec 141 §2.2 — migration 37 idempotence + effect.
//
// Asserts:
//   1. After migration 36 inserts a synthetic substrate row carrying
//      `aim-vue-node-template`, migration 37 rewrites it to
//      `aim-vue-node` in both `upstream_body` and `frontmatter`.
//   2. Migration 37 inserts a sibling `factory_upstreams` row keyed
//      `(org_id, 'aim-vue-node')` from the existing
//      `legacy-template-mixed` row, with role='scaffold' and matching
//      repo_url/ref.
//   3. Re-running migration 37 is a no-op (idempotence) — the UPDATE
//      filter and the INSERT's ON CONFLICT clause both hold.
//
// Like migration 36's test, this re-executes the .up.sql file directly
// (stripping the file's own BEGIN/COMMIT) so the assertion proves
// "the SQL itself is safe to re-run", not "the runner's once-per-DB
// gate prevents a second run".

import { describe, expect, beforeAll, afterAll, it } from "vitest";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { sql } from "drizzle-orm";
import { db } from "../drizzle";

const ORG_ID = "70141141-0000-0000-0000-000000000001";
const SEED_ARTIFACT_ID = "70141141-0000-0000-0000-000000000010";

const HERE = dirname(fileURLToPath(import.meta.url));
const MIGRATION_36 = join(HERE, "36_aim_vue_node_manifest_cutover.up.sql");
const MIGRATION_37 = join(HERE, "37_aim_vue_node_canonical_source_id.up.sql");

async function runMigration(absPath: string): Promise<void> {
  const body = await readFile(absPath, "utf8");
  const stripped = body
    .replace(/^\s*BEGIN\s*;\s*$/gim, "")
    .replace(/^\s*COMMIT\s*;\s*$/gim, "");
  await db.execute(sql.raw(stripped));
}

async function readManifest(): Promise<{
  upstream_body: string;
  scaffold_source_id: string;
} | null> {
  const result = await db.execute<{
    upstream_body: string;
    scaffold_source_id: string;
  }>(sql`
    SELECT upstream_body,
           frontmatter->>'scaffold_source_id' AS scaffold_source_id
      FROM factory_artifact_substrate
     WHERE org_id = ${ORG_ID}
       AND origin = 'oap-self'
       AND path = 'adapters/aim-vue-node/manifest.yaml'
  `);
  return result.rows[0] ?? null;
}

async function readCanonicalUpstream(): Promise<{
  source_id: string;
  role: string;
  repo_url: string;
  ref: string;
} | null> {
  const result = await db.execute<{
    source_id: string;
    role: string;
    repo_url: string;
    ref: string;
  }>(sql`
    SELECT source_id, role, repo_url, ref
      FROM factory_upstreams
     WHERE org_id = ${ORG_ID}
       AND source_id = 'aim-vue-node'
  `);
  return result.rows[0] ?? null;
}

async function countSpec141Audits(): Promise<number> {
  const result = await db.execute<{ count: string }>(sql`
    SELECT COUNT(*)::text AS count
      FROM factory_artifact_substrate_audit
     WHERE org_id = ${ORG_ID}
       AND action = 'artifact.synced'
       AND after->>'reason' = 'spec-141-migration-37'
  `);
  return Number(result.rows[0]?.count ?? "0");
}

describe("spec 141 — migration 37 (T-mig37)", () => {
  beforeAll(async () => {
    await db.execute(sql`
      INSERT INTO organizations (id, name, slug)
        VALUES (${ORG_ID}, 'spec141-mig-org', 'spec141-mig-org')
        ON CONFLICT (id) DO NOTHING
    `);
    // Seed a substrate row so migration 36's
    // `SELECT DISTINCT s.org_id` finds this org.
    await db.execute(sql`
      INSERT INTO factory_artifact_substrate (
        id, org_id, origin, path, kind, version, status,
        upstream_sha, upstream_body, content_hash, conflict_state
      )
      VALUES (
        ${SEED_ARTIFACT_ID}, ${ORG_ID}, 'goa-software-factory',
        'Factory Agent/spec141-seed.md', 'skill', 1, 'active',
        'spec141-seed-sha', 'seed body', 'spec141-seed-hash', 'ok'
      )
      ON CONFLICT (id) DO NOTHING
    `);
    // Seed the legacy template upstream row so step 2 of migration 37
    // has something to copy.
    await db.execute(sql`
      INSERT INTO factory_upstreams (
        org_id, source_id, role, repo_url, ref, subpath
      ) VALUES (
        ${ORG_ID}, 'legacy-template-mixed', 'scaffold',
        'GovAlta-Pronghorn/template', 'main', NULL
      )
      ON CONFLICT (org_id, source_id) DO NOTHING
    `);
    // Defensive cleanup — wipe any prior run's artifacts.
    await db.execute(sql`
      DELETE FROM factory_artifact_substrate_audit
       WHERE org_id = ${ORG_ID}
         AND action = 'artifact.synced'
         AND (after->>'reason' = 'spec-140-migration-36'
              OR after->>'reason' = 'spec-141-migration-37')
    `);
    await db.execute(sql`
      DELETE FROM factory_artifact_substrate
       WHERE org_id = ${ORG_ID}
         AND origin = 'oap-self'
         AND path = 'adapters/aim-vue-node/manifest.yaml'
    `);
    await db.execute(sql`
      DELETE FROM factory_upstreams
       WHERE org_id = ${ORG_ID}
         AND source_id = 'aim-vue-node'
    `);

    // Run migration 36 (immutable — inserts the row with the
    // pre-amendment literal `aim-vue-node-template`).
    await runMigration(MIGRATION_36);
  });

  afterAll(async () => {
    await db.execute(sql`
      DELETE FROM factory_artifact_substrate_audit
       WHERE org_id = ${ORG_ID}
    `);
    await db.execute(sql`
      DELETE FROM factory_artifact_substrate
       WHERE org_id = ${ORG_ID}
    `);
    await db.execute(sql`
      DELETE FROM factory_upstreams WHERE org_id = ${ORG_ID}
    `);
    await db.execute(sql`DELETE FROM organizations WHERE id = ${ORG_ID}`);
  });

  it("migration 36 leaves the row at the pre-amendment literal", async () => {
    const m = await readManifest();
    expect(m).not.toBeNull();
    expect(m!.scaffold_source_id).toBe("aim-vue-node-template");
    expect(m!.upstream_body).toContain(
      "scaffold_source_id: aim-vue-node-template",
    );
  });

  it("migration 37 rewrites the substrate row to the canonical literal", async () => {
    await runMigration(MIGRATION_37);

    const m = await readManifest();
    expect(m).not.toBeNull();
    expect(m!.scaffold_source_id).toBe("aim-vue-node");
    // Body carries the new literal AND no longer carries the old one
    // (replace, not append).
    expect(m!.upstream_body).toContain("scaffold_source_id: aim-vue-node\n");
    expect(m!.upstream_body).not.toContain(
      "scaffold_source_id: aim-vue-node-template",
    );

    expect(await countSpec141Audits()).toBe(1);
  });

  it("migration 37 promotes legacy-template-mixed to a sibling source-id-keyed row", async () => {
    const sibling = await readCanonicalUpstream();
    expect(sibling).not.toBeNull();
    expect(sibling!.source_id).toBe("aim-vue-node");
    expect(sibling!.role).toBe("scaffold");
    expect(sibling!.repo_url).toBe("GovAlta-Pronghorn/template");
    expect(sibling!.ref).toBe("main");
  });

  it("re-running migration 37 is a no-op", async () => {
    const beforeManifest = await readManifest();
    const beforeAudits = await countSpec141Audits();
    const beforeSibling = await readCanonicalUpstream();

    await runMigration(MIGRATION_37);

    const afterManifest = await readManifest();
    const afterAudits = await countSpec141Audits();
    const afterSibling = await readCanonicalUpstream();

    expect(afterManifest).toEqual(beforeManifest);
    expect(afterAudits).toBe(beforeAudits);
    expect(afterSibling).toEqual(beforeSibling);
  });
});
