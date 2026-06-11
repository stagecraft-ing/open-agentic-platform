// Spec 201 phase 2 — DB-bound tests for the approval-summary read
// endpoint (FR-001 assembly against live substrate) and AC-3 GET purity
// (reads leave the database unchanged).
//
// DB-bound; gated to `encore test` via vite.config.ts exclude list.

import { describe, expect, it, beforeAll } from "vitest";
import { sql } from "drizzle-orm";
import { db } from "../db/drizzle";
import {
  getArtifactApprovalSummaryCore,
} from "./approvalSummary";
import {
  getArtifactByIdCore,
  listArtifactsCore,
} from "./artifacts";

const ORG_ID = "88888888-0000-0000-0000-000000000001";
const USER_ID = "88888888-0000-0000-0000-000000000002";
const ORIGIN = "factory-encore";
const ENVELOPE_HASH = "3".padStart(64, "0");

let overriddenId: string;
let userAuthoredId: string;

const AUTH = { orgId: ORG_ID, userID: USER_ID };

describe("spec 201 — /api/factory/artifacts/:id/approval-summary (encore test)", () => {
  beforeAll(async () => {
    await db.execute(sql`
      INSERT INTO organizations (id, name, slug)
        VALUES (${ORG_ID}, 'spec201-as-org', 'spec201-as-org')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      INSERT INTO users (id, email, password_hash, name, role)
        VALUES (${USER_ID}, 'spec201-as@test', 'x', 'AS Tester', 'user')
        ON CONFLICT (id) DO NOTHING
    `);
    await db.execute(sql`
      DELETE FROM factory_artifact_substrate WHERE org_id = ${ORG_ID}::uuid
    `);
    await db.execute(sql`
      DELETE FROM factory_admissions WHERE org_id = ${ORG_ID}::uuid
    `);

    const overridden = await db.execute(sql`
      INSERT INTO factory_artifact_substrate
        (org_id, origin, path, kind, version, status, upstream_sha, upstream_body, user_body, user_modified_by, content_hash, conflict_state)
      VALUES
        (${ORG_ID}::uuid, ${ORIGIN},
         'process/agents/architect.md', 'agent', 1, 'active',
         ${"a".repeat(40)}, 'upstream agent body', 'overridden agent body',
         ${USER_ID}::uuid, ${"4".padStart(64, "0")}, 'ok')
      RETURNING id
    `);
    overriddenId = (overridden.rows[0] as { id: string }).id;

    const userAuthored = await db.execute(sql`
      INSERT INTO factory_artifact_substrate
        (org_id, origin, path, kind, version, status, user_body, user_modified_by, content_hash, conflict_state)
      VALUES
        (${ORG_ID}::uuid, 'user-authored',
         'agents/custom.md', 'agent', 1, 'active', 'custom agent body',
         ${USER_ID}::uuid, ${"5".padStart(64, "0")}, 'ok')
      RETURNING id
    `);
    userAuthoredId = (userAuthored.rows[0] as { id: string }).id;

    await db.execute(sql`
      INSERT INTO factory_artifact_substrate
        (org_id, origin, path, kind, version, status, upstream_sha, upstream_body, content_hash, conflict_state)
      VALUES
        (${ORG_ID}::uuid, ${ORIGIN},
         'process/governance-envelope.yaml', 'governance-envelope', 1, 'active',
         ${"b".repeat(40)}, 'schema_version: "1.0.0"', ${ENVELOPE_HASH}, 'ok')
    `);
    const composed = JSON.stringify({
      process: {
        schema_version: "1.0.0",
        process: {
          id: "factory-encore-process",
          objective_class: "scaffold",
          goal_identifier_scheme: "uuid",
        },
        ceilings: { max_tier: "tier2", max_mutation: "scoped-write" },
        gates: [{ predicate: "approval-before-build-spec-freeze" }],
        emits: [{ kind: "build-spec" }],
        constituents: { agents: "process/agents/*.md" },
        overrides: { require_verified: true },
      },
      adapters: {},
      agentDigests: {},
    });
    await db.execute(sql`
      INSERT INTO factory_admissions
        (org_id, origin, status, envelope_hash, composed, violations, scaffold_resolutions)
      VALUES
        (${ORG_ID}::uuid, ${ORIGIN}, 'admitted', ${ENVELOPE_HASH},
         ${composed}::jsonb, '[]'::jsonb, '{}'::jsonb)
    `);
  });

  it("assembles the basis for an admitted-origin override (FR-001)", async () => {
    const res = await getArtifactApprovalSummaryCore(AUTH, {
      id: overriddenId,
    });
    expect(res.applicable).toBe(true);
    expect(res.ok).toBe(true);
    const s = res.summary;
    if (!s) throw new Error("expected ok summary");
    expect(s.gatePredicate).toBe("overrides.require_verified");
    expect(s.provenanceLinks).toHaveLength(1);
    expect(s.provenanceLinks[0].kind).toBe("governance-envelope");
    expect(s.provenanceLinks[0].contentHash).toBe(ENVELOPE_HASH);
    expect(s.consumedOverrides).toHaveLength(1);
    expect(s.consumedOverrides[0].path).toBe("process/agents/architect.md");
    expect(s.consumedOverrides[0].requireVerifiedSatisfied).toBe(false);
    expect(s.summaryHash).toMatch(/^[0-9a-f]{64}$/);
    expect(s.actorId).toBe(USER_ID);
  });

  it("returns the user-authored trust class without a summary (FR-004 scope)", async () => {
    const res = await getArtifactApprovalSummaryCore(AUTH, {
      id: userAuthoredId,
    });
    expect(res).toEqual({ applicable: false });
  });

  it("AC-3 — the read path writes no rows", async () => {
    // Org-scoped counts: other DB-bound suites run concurrently in the
    // same database, so global counts race; this org is exclusively ours.
    const counts = async () => {
      const [a] = (
        await db.execute(sql`
          SELECT (SELECT count(*) FROM factory_artifact_substrate_audit
                    WHERE org_id = ${ORG_ID}::uuid)::int AS substrate_audit,
                 (SELECT count(*) FROM audit_log
                    WHERE actor_user_id = ${USER_ID})::int AS audit_log,
                 (SELECT count(*) FROM factory_runs
                    WHERE org_id = ${ORG_ID}::uuid)::int AS factory_runs,
                 (SELECT count(*) FROM factory_artifact_substrate
                    WHERE org_id = ${ORG_ID}::uuid)::int AS substrate
        `)
      ).rows as Array<Record<string, number>>;
      return a;
    };
    const before = await counts();
    await listArtifactsCore(AUTH, {});
    await getArtifactByIdCore(AUTH, { id: overriddenId });
    await getArtifactApprovalSummaryCore(AUTH, { id: overriddenId });
    const after = await counts();
    expect(after).toEqual(before);
  });
});
