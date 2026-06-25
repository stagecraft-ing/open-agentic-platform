// Spec 208 FR-001/FR-002/FR-004 (AC-2): org-wide kill-switch enforcement.
//
// Covers the shared fail-closed consult (isHaltedInScope) across the full
// scope lattice (org / project / agent-profile, plus the lifted and
// reintegrating states), the verb lifecycle (pullHaltCore / liftHaltCore +
// their audit emission and the human-actor / validation gates), and the
// end-to-end AC-2 refusals: with a halt active, grant issuance and grant
// renewal are refused with reason "halted" and a detail that names the
// quarantine record. Project-scope precision is proven both directions: a
// sibling-project halt leaves the grant working, and a same-project halt
// refuses it.
//
// Excluded from `npm test` and run only under `encore test` (the live DB is
// required), same posture as grantDuplexHandlers.test.ts. Signing uses a
// throwaway Ed25519 keypair injected via the env fallback (CONST-002: no
// committed key material) so grantPreflight reaches the halt consult instead
// of short-circuiting on signing-unconfigured.

import { describe, expect, it, beforeAll, afterAll, afterEach } from "vitest";
import { generateKeyPairSync } from "node:crypto";
import { eq } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  auditLog,
  factoryAdmissions,
  factoryRunGrants,
  factoryRuns,
  factoryUpstreams,
  orgHalts,
  organizations,
  projects,
  users,
  type OrgHaltInsert,
} from "../db/schema";
import { handleGrantRenew, handleGrantRequest } from "./grantDuplexHandlers";
import { isHaltedInScope, liftHaltCore, pullHaltCore } from "./orgHalt";
import {
  FACTORY_ORG_HALT_ACTIVATED,
  FACTORY_ORG_HALT_LIFTED,
  FACTORY_RUN_GRANT_REFUSED,
} from "./auditActions";
import type {
  ClientFactoryRunGrantRenew,
  ClientFactoryRunGrantRequest,
} from "../sync/types";

const ORG_ID = "20808080-0000-0000-0000-0000000000a1";
const USER_ID = "20808080-0000-0000-0000-0000000000a3";
const PROJECT_ID = "20808080-0000-0000-0000-0000000000a4";
const SIBLING_PROJECT_ID = "20808080-0000-0000-0000-0000000000a5";
const AGENT_PROFILE = "api-scaffolder";

const RUN_ISSUE = "20808080-0000-0000-0000-0000000000c1";
const RUN_RENEW = "20808080-0000-0000-0000-0000000000c2";
const RUN_SIBLING = "20808080-0000-0000-0000-0000000000c3";
const RUN_PROJ = "20808080-0000-0000-0000-0000000000c4";

const ORIGIN = "factory-halt-test";
const ENVELOPE_HASH = "envhash-halt-test-aaa";
const CTX = { orgId: ORG_ID, userId: USER_ID };
const AUTH = { orgId: ORG_ID, userId: USER_ID };

const KID = "fk-halt-test";
const PEM = generateKeyPairSync("ed25519")
  .privateKey.export({ type: "pkcs8", format: "pem" })
  .toString();

const META = (eventId: string) => ({
  v: 2 as const,
  eventId,
  sentAt: "2026-06-24T12:00:00Z",
});

function grantRequest(
  runId: string,
  overrides: Partial<ClientFactoryRunGrantRequest> = {},
): ClientFactoryRunGrantRequest {
  return {
    kind: "factory.run.grant_request",
    meta: META(`evt-req-${runId}`),
    runId,
    goalId: "goal-1",
    goal: "scaffold the portal",
    constraints: ["no-deploy-without-gate"],
    capsuleHash: "capsule-aaa",
    envelopeHash: ENVELOPE_HASH,
    ...overrides,
  };
}

function grantRenew(
  runId: string,
  seq: number,
  overrides: Partial<ClientFactoryRunGrantRenew> = {},
): ClientFactoryRunGrantRenew {
  return {
    kind: "factory.run.grant_renew",
    meta: META(`evt-renew-${runId}-${seq}`),
    runId,
    goalId: "goal-1",
    capsuleHash: "capsule-aaa",
    seq,
    ...overrides,
  };
}

async function seedRun(runId: string): Promise<void> {
  await db.insert(factoryRuns).values({
    id: runId,
    orgId: ORG_ID,
    projectId: PROJECT_ID,
    triggeredBy: USER_ID,
    adapterId: "adapter-x",
    processId: "process-x",
    clientRunId: `client-${runId}`,
    status: "running",
    sourceShas: { adapter: "a", process: "p", contracts: {}, agents: [] },
  });
}

/** Insert a halt row directly with sensible defaults (state control for the
 * lattice tests; the verb suite uses pullHaltCore instead). */
async function insertHalt(
  overrides: Partial<OrgHaltInsert> & Pick<OrgHaltInsert, "scope" | "scopeKey">,
): Promise<string> {
  const [row] = await db
    .insert(orgHalts)
    .values({
      orgId: ORG_ID,
      state: "halted",
      reason: "test halt",
      pulledBy: USER_ID,
      ...overrides,
    })
    .returning({ id: orgHalts.id });
  return row.id;
}

async function clearHalts(): Promise<void> {
  await db.delete(orgHalts).where(eq(orgHalts.orgId, ORG_ID));
}

async function cleanup(): Promise<void> {
  await clearHalts();
  await db.delete(factoryRunGrants).where(eq(factoryRunGrants.orgId, ORG_ID));
  await db.delete(factoryRuns).where(eq(factoryRuns.orgId, ORG_ID));
  await db.delete(factoryAdmissions).where(eq(factoryAdmissions.orgId, ORG_ID));
  await db.delete(factoryUpstreams).where(eq(factoryUpstreams.orgId, ORG_ID));
  await db.delete(auditLog).where(eq(auditLog.actorUserId, USER_ID));
  await db.delete(projects).where(eq(projects.id, SIBLING_PROJECT_ID));
  await db.delete(projects).where(eq(projects.id, PROJECT_ID));
  await db.delete(organizations).where(eq(organizations.id, ORG_ID));
  await db.delete(users).where(eq(users.id, USER_ID));
}

beforeAll(async () => {
  process.env.FACTORY_SIGNING_PRIVATE_KEY = PEM;
  process.env.FACTORY_SIGNING_KID = KID;
  await cleanup();
  await db.insert(users).values({
    id: USER_ID,
    email: "halt-test-208@example.test",
    name: "Halt Test User",
  });
  await db.insert(organizations).values({
    id: ORG_ID,
    name: "Halt Test Org",
    slug: "halt-test-org-208",
  });
  await db.insert(projects).values([
    {
      id: PROJECT_ID,
      orgId: ORG_ID,
      name: "Halt Test Project",
      slug: "halt-test-project-208",
      objectStoreBucket: "halt-test-bucket-208",
      createdBy: USER_ID,
    },
    {
      id: SIBLING_PROJECT_ID,
      orgId: ORG_ID,
      name: "Halt Sibling Project",
      slug: "halt-sibling-project-208",
      objectStoreBucket: "halt-sibling-bucket-208",
      createdBy: USER_ID,
    },
  ]);
  await db.insert(factoryUpstreams).values({
    orgId: ORG_ID,
    sourceId: ORIGIN,
    role: "orchestration",
    repoUrl: "https://github.com/example/factory-halt-test.git",
    ref: "main",
  });
  await db.insert(factoryAdmissions).values({
    orgId: ORG_ID,
    origin: ORIGIN,
    status: "admitted",
    envelopeHash: ENVELOPE_HASH,
    composed: {
      process: null,
      adapters: {
        "acme-vue-encore": { governance: {}, manifestHash: "manifesthash-halt" },
      },
      agentDigests: { "adapters/a/agents/x.md": "agenthash-halt" },
      agentIds: { "adapters/a/agents/x.md": AGENT_PROFILE },
    },
    violations: [],
    scaffoldResolutions: {},
    factorySha: "f-sha",
  });
  await Promise.all([
    seedRun(RUN_ISSUE),
    seedRun(RUN_RENEW),
    seedRun(RUN_SIBLING),
    seedRun(RUN_PROJ),
  ]);
});

afterAll(async () => {
  delete process.env.FACTORY_SIGNING_PRIVATE_KEY;
  delete process.env.FACTORY_SIGNING_KID;
  await cleanup();
});

// No halt may leak across tests: a stray org halt would make the renewal
// test's clean issuance fail closed.
afterEach(async () => {
  await clearHalts();
});

describe("isHaltedInScope (scope lattice, FR-001/FR-004)", () => {
  it("an org halt subsumes every scope query", async () => {
    const id = await insertHalt({ scope: "org", scopeKey: ORG_ID });
    expect(await isHaltedInScope(ORG_ID)).toBe(id);
    expect(await isHaltedInScope(ORG_ID, { projectId: PROJECT_ID })).toBe(id);
    expect(await isHaltedInScope(ORG_ID, { agentProfile: AGENT_PROFILE })).toBe(
      id,
    );
  });

  it("a project halt matches its project, not a sibling", async () => {
    const id = await insertHalt({ scope: "project", scopeKey: PROJECT_ID });
    expect(await isHaltedInScope(ORG_ID, { projectId: PROJECT_ID })).toBe(id);
    expect(
      await isHaltedInScope(ORG_ID, { projectId: SIBLING_PROJECT_ID }),
    ).toBeNull();
    // No projectId in scope (e.g. the org-wide duplex seam) does not match a
    // project-scoped halt.
    expect(await isHaltedInScope(ORG_ID)).toBeNull();
  });

  it("an agent-profile halt matches its profile, not another", async () => {
    const id = await insertHalt({
      scope: "agent-profile",
      scopeKey: AGENT_PROFILE,
    });
    expect(await isHaltedInScope(ORG_ID, { agentProfile: AGENT_PROFILE })).toBe(
      id,
    );
    expect(
      await isHaltedInScope(ORG_ID, { agentProfile: "other-profile" }),
    ).toBeNull();
  });

  it("a lifted halt no longer matches", async () => {
    await insertHalt({ scope: "org", scopeKey: ORG_ID, state: "lifted" });
    expect(await isHaltedInScope(ORG_ID)).toBeNull();
  });

  it("a reintegrating halt still matches (FR-004 stays enforced)", async () => {
    const id = await insertHalt({
      scope: "org",
      scopeKey: ORG_ID,
      state: "reintegrating",
    });
    expect(await isHaltedInScope(ORG_ID)).toBe(id);
  });
});

describe("pullHaltCore / liftHaltCore (FR-001/FR-004 verb + audit)", () => {
  it("pullHaltCore writes a halted row and audits activation", async () => {
    const res = await pullHaltCore(AUTH, {
      scope: "org",
      reason: "supply-chain incident",
    });
    expect(res.state).toBe("halted");
    expect(res.scopeKey).toBe(ORG_ID);

    const [row] = await db
      .select()
      .from(orgHalts)
      .where(eq(orgHalts.id, res.haltId));
    expect(row.state).toBe("halted");
    expect(row.reason).toBe("supply-chain incident");
    expect(row.pulledBy).toBe(USER_ID);

    const audits = await db
      .select()
      .from(auditLog)
      .where(eq(auditLog.action, FACTORY_ORG_HALT_ACTIVATED));
    expect(audits.some((a) => a.targetId === res.haltId)).toBe(true);
  });

  it("pullHaltCore rejects an empty reason", async () => {
    await expect(
      pullHaltCore(AUTH, { scope: "org", reason: "   " }),
    ).rejects.toThrow(/non-empty reason/);
  });

  it("pullHaltCore rejects a project scope with no scopeKey", async () => {
    await expect(
      pullHaltCore(AUTH, { scope: "project", reason: "bad" }),
    ).rejects.toThrow(/scopeKey is required/);
  });

  it("pullHaltCore rejects an agent-profile scope in Phase 1 (not enforceable)", async () => {
    await expect(
      pullHaltCore(AUTH, {
        scope: "agent-profile",
        scopeKey: AGENT_PROFILE,
        reason: "surgical",
      }),
    ).rejects.toThrow(/agent-profile.*not yet enforceable/);
  });

  it("pullHaltCore is idempotent: a repeated pull returns the same record", async () => {
    const first = await pullHaltCore(AUTH, { scope: "org", reason: "incident" });
    const second = await pullHaltCore(AUTH, {
      scope: "org",
      reason: "incident again",
    });
    expect(second.haltId).toBe(first.haltId);
    const rows = await db
      .select()
      .from(orgHalts)
      .where(eq(orgHalts.orgId, ORG_ID));
    expect(rows).toHaveLength(1);
  });

  it("liftHaltCore transitions halted to reintegrating and audits the lift", async () => {
    const pulled = await pullHaltCore(AUTH, {
      scope: "project",
      scopeKey: PROJECT_ID,
      reason: "drill",
    });
    const lifted = await liftHaltCore(AUTH, { id: pulled.haltId });
    expect(lifted.state).toBe("reintegrating");

    const [row] = await db
      .select()
      .from(orgHalts)
      .where(eq(orgHalts.id, pulled.haltId));
    expect(row.state).toBe("reintegrating");
    expect(row.liftedBy).toBe(USER_ID);
    expect(row.liftedAt).not.toBeNull();

    const audits = await db
      .select()
      .from(auditLog)
      .where(eq(auditLog.action, FACTORY_ORG_HALT_LIFTED));
    expect(audits.some((a) => a.targetId === pulled.haltId)).toBe(true);
  });

  it("liftHaltCore refuses a non-halted record", async () => {
    const pulled = await pullHaltCore(AUTH, {
      scope: "org",
      reason: "twice",
    });
    await liftHaltCore(AUTH, { id: pulled.haltId }); // -> reintegrating
    await expect(liftHaltCore(AUTH, { id: pulled.haltId })).rejects.toThrow(
      /not 'halted'/,
    );
  });

  it("liftHaltCore refuses a record from another org", async () => {
    const pulled = await pullHaltCore(AUTH, { scope: "org", reason: "x" });
    await expect(
      liftHaltCore(
        { orgId: "20808080-0000-0000-0000-0000000000ff", userId: USER_ID },
        { id: pulled.haltId },
      ),
    ).rejects.toThrow(/not found/);
  });
});

describe("grant refusal under halt (AC-2, FR-001/FR-002)", () => {
  it("refuses grant issuance during an org halt, naming the record", async () => {
    const halt = await pullHaltCore(AUTH, {
      scope: "org",
      reason: "rogue divergence",
    });
    const { reply } = await handleGrantRequest(grantRequest(RUN_ISSUE), CTX);
    expect(reply?.granted).toBe(false);
    expect(reply?.refusedReason).toBe("halted");
    expect(reply?.detail).toContain(halt.haltId);

    const audits = await db
      .select()
      .from(auditLog)
      .where(eq(auditLog.action, FACTORY_RUN_GRANT_REFUSED));
    expect(audits.length).toBeGreaterThan(0);
  });

  it("refuses grant renewal during an org halt (in-flight run pauses)", async () => {
    // Clean issuance first (no halt active), then halt, then renew.
    const issued = await handleGrantRequest(grantRequest(RUN_RENEW), CTX);
    expect(issued.reply?.granted).toBe(true);

    await pullHaltCore(AUTH, { scope: "org", reason: "halt mid-run" });
    const { reply } = await handleGrantRenew(grantRenew(RUN_RENEW, 1), CTX);
    expect(reply?.granted).toBe(false);
    expect(reply?.refusedReason).toBe("halted");
  });

  it("a sibling-project halt leaves the grant working; a same-project halt refuses it", async () => {
    // A halt on the sibling project must not refuse our project's grant (AC-3
    // scope precision); the run is bound to PROJECT_ID.
    await pullHaltCore(AUTH, {
      scope: "project",
      scopeKey: SIBLING_PROJECT_ID,
      reason: "sibling only",
    });
    const ok = await handleGrantRequest(grantRequest(RUN_SIBLING), CTX);
    expect(ok.reply?.granted).toBe(true);
    await clearHalts();

    // A halt on our own project refuses (project-scoped precision the other
    // direction).
    const halt = await pullHaltCore(AUTH, {
      scope: "project",
      scopeKey: PROJECT_ID,
      reason: "our project",
    });
    const refused = await handleGrantRequest(grantRequest(RUN_PROJ), CTX);
    expect(refused.reply?.granted).toBe(false);
    expect(refused.reply?.refusedReason).toBe("halted");
    expect(refused.reply?.detail).toContain(halt.haltId);
  });
});
