// Spec 137 Phase 4↔5 integration / migration 41 — deploy-descriptor
// secrets on the gate row.
//
// Encore's migration runner has applied migration 41 by the time tests
// run. This test verifies:
//
//   1. The new `enabled_requires_secrets` CHECK fires when an enabled
//      gate is missing either secret column.
//   2. An enabled gate carrying both secrets is accepted.
//   3. The `tls_secret_name` column defaults to 'tenants-wildcard-tls'
//      so existing call sites that don't set it explicitly stay
//      backward-compatible.
//
// Gated to `encore test` via vite.config.ts's exclude list — mutates
// real `environments` rows.

import { describe, expect, beforeAll, afterAll, it } from "vitest";
import { sql } from "drizzle-orm";
import { db } from "../drizzle";

const ORG_ID = "70137041-0000-0000-0000-000000000001";
const PROJECT_ID = "70137041-0000-0000-0000-000000000002";
const ENV_ID = "70137041-0000-0000-0000-000000000010";

describe("spec 137 — migration 41 (gate deploy-descriptor secrets)", () => {
  beforeAll(async () => {
    await db.execute(sql`
      INSERT INTO organizations (id, name, slug)
        VALUES (${ORG_ID}, 'spec137-mig41-org', 'spec137-mig41-org')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO projects (id, org_id, name, slug)
        VALUES (${PROJECT_ID}, ${ORG_ID}, 'spec137-mig41-project', 'spec137-mig41-project')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO environments (id, project_id, name, kind)
        VALUES (${ENV_ID}, ${PROJECT_ID}, ${ENV_ID}, 'development')
        ON CONFLICT (id) DO NOTHING
    `);
  });

  afterAll(async () => {
    await db.execute(sql`
      DELETE FROM environment_access_gates WHERE environment_id = ${ENV_ID}
    `);
    await db.execute(sql`DELETE FROM environments WHERE id = ${ENV_ID}`);
    await db.execute(sql`DELETE FROM projects WHERE id = ${PROJECT_ID}`);
    await db.execute(sql`DELETE FROM organizations WHERE id = ${ORG_ID}`);
  });

  it("rejects enabled=true with NULL cookie_secret (enabled_requires_secrets)", async () => {
    await expect(
      db.execute(sql`
        INSERT INTO environment_access_gates (
          environment_id, enabled, rauthy_client_ref,
          rauthy_client_secret, cookie_secret
        )
        VALUES (${ENV_ID}, true, 'rauthy-client-x', 'rauthy-secret-x', NULL)
      `),
    ).rejects.toThrow(/enabled_requires_secrets/i);
  });

  it("rejects enabled=true with NULL rauthy_client_secret (enabled_requires_secrets)", async () => {
    await expect(
      db.execute(sql`
        INSERT INTO environment_access_gates (
          environment_id, enabled, rauthy_client_ref,
          rauthy_client_secret, cookie_secret
        )
        VALUES (${ENV_ID}, true, 'rauthy-client-x', NULL, 'cookie-secret-x')
      `),
    ).rejects.toThrow(/enabled_requires_secrets/i);
  });

  it("accepts enabled=true with both secrets populated", async () => {
    await db.execute(sql`
      INSERT INTO environment_access_gates (
        environment_id, enabled, rauthy_client_ref,
        rauthy_client_secret, cookie_secret
      )
      VALUES (
        ${ENV_ID}, true, 'rauthy-client-x',
        'rauthy-secret-x', 'cookie-secret-x'
      )
    `);
    const row = await db.execute<{
      tls_secret_name: string;
      rauthy_client_secret: string;
      cookie_secret: string;
    }>(sql`
      SELECT tls_secret_name, rauthy_client_secret, cookie_secret
        FROM environment_access_gates
        WHERE environment_id = ${ENV_ID}
    `);
    expect(row.rows[0]?.tls_secret_name).toBe("tenants-wildcard-tls");
    expect(row.rows[0]?.rauthy_client_secret).toBe("rauthy-secret-x");
    expect(row.rows[0]?.cookie_secret).toBe("cookie-secret-x");
    await db.execute(sql`
      DELETE FROM environment_access_gates WHERE environment_id = ${ENV_ID}
    `);
  });

  it("accepts enabled=false with NULL secrets and applies tls_secret_name default", async () => {
    await db.execute(sql`
      INSERT INTO environment_access_gates (environment_id, enabled)
        VALUES (${ENV_ID}, false)
    `);
    const row = await db.execute<{ tls_secret_name: string }>(sql`
      SELECT tls_secret_name FROM environment_access_gates
        WHERE environment_id = ${ENV_ID}
    `);
    expect(row.rows[0]?.tls_secret_name).toBe("tenants-wildcard-tls");
    await db.execute(sql`
      DELETE FROM environment_access_gates WHERE environment_id = ${ENV_ID}
    `);
  });

  it("honours an explicit tls_secret_name override", async () => {
    await db.execute(sql`
      INSERT INTO environment_access_gates (environment_id, enabled, tls_secret_name)
        VALUES (${ENV_ID}, false, 'custom-org-wildcard-tls')
    `);
    const row = await db.execute<{ tls_secret_name: string }>(sql`
      SELECT tls_secret_name FROM environment_access_gates
        WHERE environment_id = ${ENV_ID}
    `);
    expect(row.rows[0]?.tls_secret_name).toBe("custom-org-wildcard-tls");
    await db.execute(sql`
      DELETE FROM environment_access_gates WHERE environment_id = ${ENV_ID}
    `);
  });
});
