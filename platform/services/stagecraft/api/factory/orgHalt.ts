// Spec 208 FR-001/FR-002/FR-004: org-wide agent kill-switch verb.
//
// The switch is a thin org-scoped verb over primitives that already ship: it
// writes a quarantine record (org_halts) that the existing fail-closed check
// sites (grant issuance/renewal in grantDuplexHandlers.ts, new-session
// registration in api/sync/duplex.ts, serve/bind in admission.ts) consult via
// `isHaltedInScope`. No parallel control plane.
//
// Two-tier shape (the revocations.ts liftRevocationCore precedent): each verb
// is a `*Core(auth, req)` function plus a thin `api(...)` endpoint wrapper that
// resolves `auth` through `requireFactoryConfigure`. AC-2/AC-4 tests drive the
// core functions directly with an explicit `{orgId, userId}` and never need an
// HTTP/auth context. The human-actor gate (AC-4: a service identity is
// rejected) is structural: the only route in is the `auth: true` endpoint
// through `requireFactoryConfigure`, and a service identity has no
// org-membership row to satisfy it.
//
// Phase 1 enforces `org` and `project` scopes. `agent-profile` is rejected at
// the verb (see pullHaltCore): no Phase-1 seam carries the agent profile in
// scope (the duplex handshake is per-OPC-connection, the grant is run/project
// bound), so accepting an agent-profile halt would audit it "active" while
// enforcing nothing, which is exactly the false-containment a kill switch must
// not ship. The `isHaltedInScope` agent-profile branch stays ready for the
// phase that adds a profile-carrying seam.

import { api, APIError } from "encore.dev/api";
import { and, eq, ne, or } from "drizzle-orm";
import { db } from "../db/drizzle";
import { auditLog, orgHalts } from "../db/schema";
import { requireFactoryConfigure } from "./revocations";
import {
  FACTORY_ORG_HALT_ACTIVATED,
  FACTORY_ORG_HALT_LIFTED,
} from "./auditActions";

export type HaltScope = "org" | "project" | "agent-profile";
export type HaltState = "halted" | "reintegrating" | "lifted";

/** Discriminators a check site can supply to scope the halt consult. */
export interface HaltScopeQuery {
  /** The project the action belongs to (known at the grant seam). */
  projectId?: string | null;
  /** The agent profile the action belongs to (known at session seams). */
  agentProfile?: string | null;
}

/**
 * Returns the id of an active (non-'lifted') halt covering this scope, or
 * null. Scope union (spec 208 §Scope lattice): an `org` halt subsumes
 * everything; a `project` halt matches when its scope_key === projectId; an
 * `agent-profile` halt matches when its scope_key === agentProfile. A
 * 'reintegrating' row still counts as active (FR-004); only 'lifted' clears
 * the scope, which is exactly the partial liveness index predicate.
 *
 * The scope predicate is pushed into SQL with `.limit(1)` (the
 * `hasActiveRevocation` precedent) so this fail-closed hot-path consult is an
 * indexed equality lookup, not an in-app scan: `org_id` + `state != 'lifted'`
 * is the partial index, and the scope-union arms key off `scope`/`scope_key`.
 */
export async function isHaltedInScope(
  orgId: string,
  { projectId, agentProfile }: HaltScopeQuery = {},
): Promise<string | null> {
  const scopeArms = [eq(orgHalts.scope, "org")];
  if (projectId) {
    scopeArms.push(
      and(eq(orgHalts.scope, "project"), eq(orgHalts.scopeKey, projectId))!,
    );
  }
  if (agentProfile) {
    scopeArms.push(
      and(
        eq(orgHalts.scope, "agent-profile"),
        eq(orgHalts.scopeKey, agentProfile),
      )!,
    );
  }

  const [row] = await db
    .select({ id: orgHalts.id })
    .from(orgHalts)
    .where(
      and(
        eq(orgHalts.orgId, orgId),
        ne(orgHalts.state, "lifted"),
        or(...scopeArms),
      ),
    )
    .limit(1);

  return row?.id ?? null;
}

export interface PullHaltRequest {
  scope: HaltScope;
  /**
   * The scope target: a project id for `project`. Ignored for `org` (the org
   * id is used by convention).
   */
  scopeKey?: string;
  reason: string;
}

export interface PullHaltResponse {
  haltId: string;
  scope: HaltScope;
  scopeKey: string;
  /** The active state: 'halted' for a fresh pull, or the existing state if an
   * active halt already covered this scope (the pull is idempotent). */
  state: "halted" | "reintegrating";
}

/**
 * FR-001: set a halt for one scope. Requires `factory:configure` (resolved by
 * the endpoint wrapper) and a non-empty reason (the reason is audit evidence,
 * not a toggle). The row write and its activation audit are one transaction
 * (the reason-as-evidence guarantee must be atomic). Idempotent: if an active
 * (non-'lifted') halt already covers the exact scope, it is returned rather
 * than duplicated, and the partial UNIQUE index is the DB backstop against a
 * racing second insert.
 */
export async function pullHaltCore(
  auth: { orgId: string; userId: string },
  req: PullHaltRequest,
): Promise<PullHaltResponse> {
  if (req.scope === "agent-profile") {
    // See the file header: no Phase-1 seam carries the agent profile, so an
    // agent-profile halt would enforce nothing. Reject rather than ship a
    // halt an operator mistakes for containment.
    throw APIError.unimplemented(
      "agent-profile-scoped halts are not yet enforceable (no Phase-1 seam " +
        "carries the agent profile); use 'org' or 'project' scope",
    );
  }
  const reason = req.reason?.trim();
  if (!reason) {
    throw APIError.invalidArgument(
      "a non-empty reason is required to pull a halt",
    );
  }
  // For an `org` scope the scope_key is the org id by convention; a project
  // scope carries an explicit target.
  const scopeKey =
    req.scope === "org" ? auth.orgId : (req.scopeKey ?? "").trim();
  if (req.scope !== "org" && !scopeKey) {
    throw APIError.invalidArgument(
      `scopeKey is required for a ${req.scope}-scoped halt`,
    );
  }

  return db.transaction(async (tx) => {
    const [existing] = await tx
      .select({ id: orgHalts.id, state: orgHalts.state })
      .from(orgHalts)
      .where(
        and(
          eq(orgHalts.orgId, auth.orgId),
          eq(orgHalts.scope, req.scope),
          eq(orgHalts.scopeKey, scopeKey),
          ne(orgHalts.state, "lifted"),
        ),
      )
      .limit(1);
    if (existing) {
      return {
        haltId: existing.id,
        scope: req.scope,
        scopeKey,
        state: existing.state as "halted" | "reintegrating",
      };
    }

    const [row] = await tx
      .insert(orgHalts)
      .values({
        orgId: auth.orgId,
        scope: req.scope,
        scopeKey,
        state: "halted",
        reason,
        pulledBy: auth.userId,
      })
      .returning({ id: orgHalts.id });

    await tx.insert(auditLog).values({
      actorUserId: auth.userId,
      action: FACTORY_ORG_HALT_ACTIVATED,
      targetType: "org_halts",
      targetId: row.id,
      metadata: { orgId: auth.orgId, scope: req.scope, scopeKey, reason },
    });

    return { haltId: row.id, scope: req.scope, scopeKey, state: "halted" };
  });
}

export const pullHalt = api(
  { method: "POST", path: "/api/factory/org-halts", expose: true, auth: true },
  async (req: PullHaltRequest): Promise<PullHaltResponse> => {
    const auth = await requireFactoryConfigure();
    return pullHaltCore(auth, req);
  },
);

export interface LiftHaltRequest {
  id: string;
}

export interface LiftHaltResponse {
  haltId: string;
  state: "reintegrating";
}

/**
 * FR-004 (Phase 1 leg): begin lifting a halt. Requires a human actor id
 * (the endpoint wrapper's `factory:configure` gate; a service identity is
 * rejected, AC-4) and transitions 'halted' -> 'reintegrating'. Lifting alone
 * does not re-admit: a 'reintegrating' scope still refuses new sessions and
 * grants until staged per-scope re-admission flips it to 'lifted', which
 * lands in Phase 3 (the revocations.ts "lifting alone does not re-admit"
 * precedent).
 *
 * The transition is a compare-and-swap inside one transaction: the UPDATE
 * carries `AND state = 'halted'` and the affected-row count is the gate, so
 * two concurrent lifts cannot both transition + double-audit (a TOCTOU the
 * read-then-write shape would otherwise allow). The state mutation and its
 * audit are atomic.
 */
export async function liftHaltCore(
  auth: { orgId: string; userId: string },
  req: LiftHaltRequest,
): Promise<LiftHaltResponse> {
  return db.transaction(async (tx) => {
    const [row] = await tx
      .select({
        orgId: orgHalts.orgId,
        state: orgHalts.state,
        scope: orgHalts.scope,
        scopeKey: orgHalts.scopeKey,
      })
      .from(orgHalts)
      .where(eq(orgHalts.id, req.id));

    if (!row || row.orgId !== auth.orgId) {
      throw APIError.notFound("halt record not found");
    }
    if (row.state !== "halted") {
      throw APIError.failedPrecondition(
        `halt ${req.id} is '${row.state}', not 'halted'`,
      );
    }

    // Compare-and-swap: only the writer that observes 'halted' transitions it.
    const updated = await tx
      .update(orgHalts)
      .set({
        state: "reintegrating",
        liftedBy: auth.userId,
        liftedAt: new Date(),
      })
      .where(and(eq(orgHalts.id, req.id), eq(orgHalts.state, "halted")))
      .returning({ id: orgHalts.id });
    if (updated.length === 0) {
      // Lost the race: another lift transitioned it between the read and here.
      throw APIError.failedPrecondition(
        `halt ${req.id} is no longer 'halted'`,
      );
    }

    await tx.insert(auditLog).values({
      actorUserId: auth.userId,
      action: FACTORY_ORG_HALT_LIFTED,
      targetType: "org_halts",
      targetId: req.id,
      metadata: { orgId: auth.orgId, scope: row.scope, scopeKey: row.scopeKey },
    });

    return { haltId: req.id, state: "reintegrating" };
  });
}

export const liftHalt = api(
  {
    method: "POST",
    path: "/api/factory/org-halts/:id/lift",
    expose: true,
    auth: true,
  },
  async ({ id }: { id: string }): Promise<LiftHaltResponse> => {
    const auth = await requireFactoryConfigure();
    return liftHaltCore(auth, { id });
  },
);
