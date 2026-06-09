/**
 * Spec 198 FR-010 — revocation operations on the admission graph.
 *
 * One mechanism, four keys (factory / adapter / agent / content-hash), two
 * modes (`revoked` / `quarantined`). Rows here are consulted fail-closed by
 * the serve path (`browse.ts` via `admission.isFactoryAdmitted`), the bind
 * path (`projects/create.ts`), and — when the run-grant service lands
 * (FR-005/FR-014 phase 4) — grant issuance/renewal.
 *
 * Lifting a quarantine is NOT an un-revoke: reintegration requires fresh
 * two-sided validation (a re-sync re-evaluates admission) plus this
 * explicit human approval (ASI10 m7). Content-hash revocations are never
 * lifted — fixed upstream bytes carry a new hash and re-enter via normal
 * admission.
 */

import { api, APIError } from "encore.dev/api";
import log from "encore.dev/log";
import { getAuthData } from "~encore/auth";
import { and, desc, eq, isNull, or, sql } from "drizzle-orm";
import { db } from "../db/drizzle";
import { auditLog, factoryRevocations } from "../db/schema";
import { getUserOrgRole, hasOrgPermission } from "../auth/membership";

type ScopeKind = "factory" | "adapter" | "agent" | "content-hash";
type Mode = "revoked" | "quarantined";

async function requireFactoryConfigure(): Promise<{
  orgId: string;
  userId: string;
}> {
  const auth = getAuthData()!;
  const role = await getUserOrgRole(auth.userID, auth.orgId);
  if (!role || !hasOrgPermission(role, "factory:configure")) {
    throw APIError.permissionDenied(
      "factory:configure permission required for revocation operations",
    );
  }
  return { orgId: auth.orgId, userId: auth.userID };
}

export type RevocationRow = {
  id: string;
  scopeKind: ScopeKind;
  key: string;
  mode: Mode;
  reason: string;
  createdAt: string;
  liftedAt: string | null;
};

export const listRevocations = api(
  { expose: true, auth: true, method: "GET", path: "/api/factory/revocations" },
  async (): Promise<{ revocations: RevocationRow[] }> => {
    const { orgId } = await requireFactoryConfigure();
    const rows = await db
      .select()
      .from(factoryRevocations)
      .where(
        or(
          eq(factoryRevocations.orgId, sql`${orgId}::uuid`),
          isNull(factoryRevocations.orgId),
        ),
      )
      .orderBy(desc(factoryRevocations.createdAt))
      .limit(200);
    return {
      revocations: rows.map((r) => ({
        id: r.id,
        scopeKind: r.scopeKind,
        key: r.key,
        mode: r.mode,
        reason: r.reason,
        createdAt: r.createdAt.toISOString(),
        liftedAt: r.liftedAt ? r.liftedAt.toISOString() : null,
      })),
    };
  },
);

export const createRevocation = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/factory/revocations",
  },
  async (req: {
    scope_kind: ScopeKind;
    key: string;
    mode: Mode;
    reason: string;
  }): Promise<{ id: string }> => {
    const { orgId, userId } = await requireFactoryConfigure();
    if (!req.key.trim()) {
      throw APIError.invalidArgument("revocation key is empty");
    }
    if (!req.reason.trim()) {
      throw APIError.invalidArgument(
        "a revocation requires a reason (it is audit evidence, not a toggle)",
      );
    }
    const [row] = await db
      .insert(factoryRevocations)
      .values({
        orgId,
        scopeKind: req.scope_kind,
        key: req.key,
        mode: req.mode,
        reason: req.reason,
        actor: userId,
      })
      .returning({ id: factoryRevocations.id });
    await db.insert(auditLog).values({
      actorUserId: userId,
      action: "factory.revocation.created",
      targetType: "factory_revocations",
      targetId: row.id,
      metadata: {
        orgId,
        scopeKind: req.scope_kind,
        key: req.key,
        mode: req.mode,
        reason: req.reason,
      },
    });
    log.warn("factory revocation created — takes effect at serve/bind/grant", {
      orgId,
      scopeKind: req.scope_kind,
      key: req.key,
      mode: req.mode,
    });
    return { id: row.id };
  },
);

export const liftRevocation = api(
  {
    expose: true,
    auth: true,
    method: "POST",
    path: "/api/factory/revocations/:id/lift",
  },
  async (req: { id: string }): Promise<{ lifted: boolean }> => {
    const { orgId, userId } = await requireFactoryConfigure();
    const [row] = await db
      .select()
      .from(factoryRevocations)
      .where(eq(factoryRevocations.id, req.id))
      .limit(1);
    if (!row || (row.orgId !== null && row.orgId !== orgId)) {
      throw APIError.notFound("revocation not found");
    }
    if (row.orgId === null) {
      throw APIError.permissionDenied(
        "global advisories cannot be lifted org-side",
      );
    }
    if (row.scopeKind === "content-hash") {
      throw APIError.failedPrecondition(
        "content-hash revocations are never lifted — fixed upstream bytes carry a new hash and re-enter via normal admission (spec 198 FR-010)",
      );
    }
    if (row.liftedAt) return { lifted: true };
    await db
      .update(factoryRevocations)
      .set({ liftedAt: new Date(), liftedBy: userId })
      .where(eq(factoryRevocations.id, req.id));
    await db.insert(auditLog).values({
      actorUserId: userId,
      action: "factory.revocation.lifted",
      targetType: "factory_revocations",
      targetId: req.id,
      metadata: {
        orgId,
        scopeKind: row.scopeKind,
        key: row.key,
        note: "reintegration requires a fresh sync (re-evaluated admission) — lifting alone does not re-admit",
      },
    });
    return { lifted: true };
  },
);
