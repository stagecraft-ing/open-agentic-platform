// Spec 137 Phase 1 / T014 — migration 40 idempotence + CHECK behaviour.
//
// Encore's migration runner has already applied migration 40 by the time
// tests run, so the tables exist. This test:
//
//   1. Verifies the four CHECK constraints fire as designed.
//   2. Verifies the unique allowlist index enforces case-insensitive
//      uniqueness without `citext`.
//   3. Verifies `ON DELETE CASCADE` on `environments(id)` cascades through
//      both gate tables.
//
// **Gated to `encore test`** via vite.config.ts's exclude list — this
// test mutates real `environments` rows.

import { describe, expect, beforeAll, afterAll, it } from "vitest";
import { sql } from "drizzle-orm";
import { db } from "../drizzle";

const ORG_ID = "70137040-0000-0000-0000-000000000001";
const PROJECT_ID = "70137040-0000-0000-0000-000000000002";
const ENV_ID_GATE = "70137040-0000-0000-0000-000000000010";
const ENV_ID_ALLOW = "70137040-0000-0000-0000-000000000011";
const ENV_ID_CASCADE = "70137040-0000-0000-0000-000000000012";

describe("spec 137 Phase 1 — migration 40 (environment access gates)", () => {
  beforeAll(async () => {
    // Provision org → project → environments fixture
    await db.execute(sql`
      INSERT INTO organizations (id, name, slug)
        VALUES (${ORG_ID}, 'spec137-mig40-org', 'spec137-mig40-org')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO projects (id, org_id, name, slug)
        VALUES (${PROJECT_ID}, ${ORG_ID}, 'spec137-mig40-project', 'spec137-mig40-project')
        ON CONFLICT (id) DO NOTHING
    `);
    for (const envId of [ENV_ID_GATE, ENV_ID_ALLOW, ENV_ID_CASCADE]) {
      await db.execute(sql`
        INSERT INTO environments (id, project_id, name, kind)
          VALUES (${envId}, ${PROJECT_ID}, ${envId}, 'development')
          ON CONFLICT (id) DO NOTHING
      `);
    }
  });

  afterAll(async () => {
    // The CASCADE tests delete envs by themselves; clean up anything left
    await db.execute(sql`
      DELETE FROM environment_access_gate_allowlist_emails
        WHERE environment_id IN (${ENV_ID_GATE}, ${ENV_ID_ALLOW}, ${ENV_ID_CASCADE})
    `);
    await db.execute(sql`
      DELETE FROM environment_access_gates
        WHERE environment_id IN (${ENV_ID_GATE}, ${ENV_ID_ALLOW}, ${ENV_ID_CASCADE})
    `);
    await db.execute(sql`
      DELETE FROM environments
        WHERE id IN (${ENV_ID_GATE}, ${ENV_ID_ALLOW}, ${ENV_ID_CASCADE})
    `);
    await db.execute(sql`DELETE FROM projects WHERE id = ${PROJECT_ID}`);
    await db.execute(sql`DELETE FROM organizations WHERE id = ${ORG_ID}`);
  });

  // -------------------------------------------------------------------------
  // CHECK enabled_requires_ref
  // -------------------------------------------------------------------------

  it("rejects enabled=true with NULL rauthy_client_ref (enabled_requires_ref)", async () => {
    await expect(
      db.execute(sql`
        INSERT INTO environment_access_gates (environment_id, enabled, rauthy_client_ref)
          VALUES (${ENV_ID_GATE}, true, NULL)
      `),
    ).rejects.toThrow(/enabled_requires_ref/i);
  });

  it("accepts enabled=true with a rauthy_client_ref present", async () => {
    await db.execute(sql`
      INSERT INTO environment_access_gates (environment_id, enabled, rauthy_client_ref)
        VALUES (${ENV_ID_GATE}, true, 'rauthy-client-smoke-1')
    `);
    const row = await db.execute<{ count: string }>(sql`
      SELECT COUNT(*)::text AS count FROM environment_access_gates
        WHERE environment_id = ${ENV_ID_GATE}
    `);
    expect(Number(row.rows[0]?.count ?? "0")).toBe(1);
    // Clean up so the next test starts fresh
    await db.execute(sql`
      DELETE FROM environment_access_gates WHERE environment_id = ${ENV_ID_GATE}
    `);
  });

  it("accepts enabled=false with NULL rauthy_client_ref", async () => {
    await db.execute(sql`
      INSERT INTO environment_access_gates (environment_id, enabled, rauthy_client_ref)
        VALUES (${ENV_ID_GATE}, false, NULL)
    `);
    await db.execute(sql`
      DELETE FROM environment_access_gates WHERE environment_id = ${ENV_ID_GATE}
    `);
  });

  // -------------------------------------------------------------------------
  // CHECK federated_provider_values
  // -------------------------------------------------------------------------

  it("rejects unknown federated provider value (federated_provider_values)", async () => {
    await expect(
      db.execute(sql`
        INSERT INTO environment_access_gates (
          environment_id, enabled, rauthy_client_ref,
          login_method_federated_provider, login_method_federated_provider_client_ref
        )
        VALUES (${ENV_ID_GATE}, false, NULL, 'okta', 'okta-client-1')
      `),
    ).rejects.toThrow(/federated_provider_values/i);
  });

  it("accepts each known federated provider value", async () => {
    for (const provider of ["google", "microsoft", "github", "generic_oidc"]) {
      await db.execute(sql`
        INSERT INTO environment_access_gates (
          environment_id, enabled, rauthy_client_ref,
          login_method_federated_provider, login_method_federated_provider_client_ref
        )
        VALUES (${ENV_ID_GATE}, false, NULL, ${provider}, ${`${provider}-client-1`})
      `);
      await db.execute(sql`
        DELETE FROM environment_access_gates WHERE environment_id = ${ENV_ID_GATE}
      `);
    }
  });

  // -------------------------------------------------------------------------
  // CHECK federated_pair_consistent
  // -------------------------------------------------------------------------

  it("rejects federated_provider without client_ref (federated_pair_consistent)", async () => {
    await expect(
      db.execute(sql`
        INSERT INTO environment_access_gates (
          environment_id, enabled, rauthy_client_ref,
          login_method_federated_provider, login_method_federated_provider_client_ref
        )
        VALUES (${ENV_ID_GATE}, false, NULL, 'google', NULL)
      `),
    ).rejects.toThrow(/federated_pair_consistent/i);
  });

  it("rejects federated client_ref without provider (federated_pair_consistent)", async () => {
    await expect(
      db.execute(sql`
        INSERT INTO environment_access_gates (
          environment_id, enabled, rauthy_client_ref,
          login_method_federated_provider, login_method_federated_provider_client_ref
        )
        VALUES (${ENV_ID_GATE}, false, NULL, NULL, 'orphan-client')
      `),
    ).rejects.toThrow(/federated_pair_consistent/i);
  });

  // -------------------------------------------------------------------------
  // Allowlist CHECK + uniqueness
  // -------------------------------------------------------------------------

  it("rejects allowlist row with unknown kind (kind_values)", async () => {
    await expect(
      db.execute(sql`
        INSERT INTO environment_access_gate_allowlist_emails (environment_id, kind, value)
          VALUES (${ENV_ID_ALLOW}, 'phone', '555-1234')
      `),
    ).rejects.toThrow(/kind_values/i);
  });

  it("enforces case-insensitive uniqueness via lower(value) index", async () => {
    await db.execute(sql`
      INSERT INTO environment_access_gate_allowlist_emails (environment_id, kind, value)
        VALUES (${ENV_ID_ALLOW}, 'email', 'Alice@Example.COM')
    `);
    await expect(
      db.execute(sql`
        INSERT INTO environment_access_gate_allowlist_emails (environment_id, kind, value)
          VALUES (${ENV_ID_ALLOW}, 'email', 'alice@example.com')
      `),
    ).rejects.toThrow(/environment_access_gate_allowlist_emails_unique/i);
    // Different kind on the same value: allowed (kind is part of the unique index)
    await db.execute(sql`
      INSERT INTO environment_access_gate_allowlist_emails (environment_id, kind, value)
        VALUES (${ENV_ID_ALLOW}, 'domain', 'alice@example.com')
    `);
    // Cleanup
    await db.execute(sql`
      DELETE FROM environment_access_gate_allowlist_emails WHERE environment_id = ${ENV_ID_ALLOW}
    `);
  });

  // -------------------------------------------------------------------------
  // ON DELETE CASCADE
  // -------------------------------------------------------------------------

  it("cascades environment delete through both gate tables", async () => {
    // Plant a gate + two allowlist entries on the cascade env
    await db.execute(sql`
      INSERT INTO environment_access_gates (environment_id, enabled, rauthy_client_ref)
        VALUES (${ENV_ID_CASCADE}, true, 'rauthy-cascade-client')
    `);
    await db.execute(sql`
      INSERT INTO environment_access_gate_allowlist_emails (environment_id, kind, value)
        VALUES (${ENV_ID_CASCADE}, 'email', 'cascade@example.com')
    `);
    await db.execute(sql`
      INSERT INTO environment_access_gate_allowlist_emails (environment_id, kind, value)
        VALUES (${ENV_ID_CASCADE}, 'domain', 'example.com')
    `);

    // Delete the environment row — CASCADE should remove the gate + 2 allowlist entries
    await db.execute(sql`DELETE FROM environments WHERE id = ${ENV_ID_CASCADE}`);

    const gateCount = await db.execute<{ count: string }>(sql`
      SELECT COUNT(*)::text AS count FROM environment_access_gates
        WHERE environment_id = ${ENV_ID_CASCADE}
    `);
    const allowCount = await db.execute<{ count: string }>(sql`
      SELECT COUNT(*)::text AS count FROM environment_access_gate_allowlist_emails
        WHERE environment_id = ${ENV_ID_CASCADE}
    `);
    expect(Number(gateCount.rows[0]?.count ?? "0")).toBe(0);
    expect(Number(allowCount.rows[0]?.count ?? "0")).toBe(0);
  });
});
