/**
 * Spec 201 FR-001 — server-side `ApprovalSummary` assembly from recorded
 * facts. DB half: fetches admission state + substrate rows and feeds the
 * pure assembly in `approvalSummary-pure.ts`.
 *
 * Read-only by contract (FR-003): no row writes, no model calls, no I/O
 * other than DB reads; the only clock-dependent value is `assembledAt`,
 * which is outside the hashed field set.
 */

import { api, APIError } from "encore.dev/api";
import { getAuthData } from "~encore/auth";
import { and, eq, inArray, isNotNull } from "drizzle-orm";
import { db } from "../db/drizzle";
import { factoryArtifactSubstrate } from "../db/schema";
import { isFactoryAdmitted, loadLatestAdmission } from "./admission";
import {
  assembleApprovalSummaryFromFacts,
  OVERRIDE_VERIFICATION_PREDICATE,
  type ApprovalSummary,
  type ApprovalSummaryResult,
} from "./approvalSummary-pure";

export {
  OVERRIDE_VERIFICATION_PREDICATE,
  computeApprovalSummaryHash,
  type ApprovalSummary,
  type ApprovalSummaryResult,
  type ApprovalProvenanceLink,
  type ApprovalConsumedOverride,
} from "./approvalSummary-pure";

// ---------------------------------------------------------------------------
// Per-artifact read endpoint — what the override-verify surface renders
// (spec 201 FR-002; phase 2). GET, read-only: satisfies FR-003 preview
// purity by construction.
// ---------------------------------------------------------------------------

/** Flat (Encore requires a named interface, not a union, at the wire
 * boundary): `applicable: false` → spec 111 user-authored trust class, no
 * envelope governs the row (FR-004 scoping amendment), other fields
 * absent. Otherwise exactly one of `summary` (ok) or `reason` (refused)
 * is present. */
export interface ArtifactApprovalSummaryResponse {
  applicable: boolean;
  ok?: boolean;
  reason?: string;
  summary?: ApprovalSummary;
}

export async function getArtifactApprovalSummaryCore(
  auth: { orgId: string; userID: string },
  req: { id: string },
): Promise<ArtifactApprovalSummaryResponse> {
  // Inline org-scoped lookup (importing artifacts.ts here would cycle —
  // verifyOverrideCore imports this module).
  const rows = await db
    .select({
      origin: factoryArtifactSubstrate.origin,
    })
    .from(factoryArtifactSubstrate)
    .where(
      and(
        eq(factoryArtifactSubstrate.orgId, auth.orgId),
        eq(factoryArtifactSubstrate.id, req.id),
      ),
    )
    .limit(1);
  if (!rows[0]) {
    throw APIError.notFound(`artifact ${req.id} not found`);
  }
  if (rows[0].origin === "user-authored") {
    return { applicable: false };
  }
  const result = await assembleApprovalSummary({
    orgId: auth.orgId,
    origin: rows[0].origin,
    gatePredicate: OVERRIDE_VERIFICATION_PREDICATE,
    actorId: auth.userID,
  });
  return result.ok
    ? { applicable: true, ok: true, summary: result.summary }
    : { applicable: true, ok: false, reason: result.reason };
}

export const getArtifactApprovalSummary = api(
  {
    expose: true,
    auth: true,
    method: "GET",
    path: "/api/factory/artifacts/:id/approval-summary",
  },
  async (req: { id: string }): Promise<ArtifactApprovalSummaryResponse> => {
    const auth = getAuthData()!;
    return getArtifactApprovalSummaryCore(
      { orgId: auth.orgId, userID: auth.userID },
      req,
    );
  },
);

export type AssembleApprovalSummaryInput = {
  orgId: string;
  /** Admitted-factory origin (the substrate `origin` discriminator). */
  origin: string;
  /** Envelope-declared gate predicate, or the reserved
   * `overrides.require_verified` for the override-verify surface. */
  gatePredicate: string;
  /** Rauthy subject who will perform the approval (TOCTOU-bound). */
  actorId: string;
};

export async function assembleApprovalSummary(
  input: AssembleApprovalSummaryInput,
): Promise<ApprovalSummaryResult> {
  const verdict = await isFactoryAdmitted(input.orgId, input.origin);
  const state = await loadLatestAdmission(input.orgId, input.origin);

  const provenanceRows = await db
    .select({
      id: factoryArtifactSubstrate.id,
      path: factoryArtifactSubstrate.path,
      kind: factoryArtifactSubstrate.kind,
      contentHash: factoryArtifactSubstrate.contentHash,
    })
    .from(factoryArtifactSubstrate)
    .where(
      and(
        eq(factoryArtifactSubstrate.orgId, input.orgId),
        eq(factoryArtifactSubstrate.origin, input.origin),
        eq(factoryArtifactSubstrate.status, "active"),
        inArray(factoryArtifactSubstrate.kind, [
          "governance-envelope",
          "adapter-manifest",
        ]),
      ),
    )
    .orderBy(factoryArtifactSubstrate.path);

  // FR-001 rule 2 — spec 198 FR-013 c parity (`collectConsumedOverrides`):
  // org + origin, active, non-null user_body, path-ordered.
  const overrideRows = await db
    .select({
      id: factoryArtifactSubstrate.id,
      path: factoryArtifactSubstrate.path,
      contentHash: factoryArtifactSubstrate.contentHash,
      userBodyVerified: factoryArtifactSubstrate.userBodyVerified,
      verifiedBy: factoryArtifactSubstrate.verifiedBy,
      verifiedAt: factoryArtifactSubstrate.verifiedAt,
    })
    .from(factoryArtifactSubstrate)
    .where(
      and(
        eq(factoryArtifactSubstrate.orgId, input.orgId),
        eq(factoryArtifactSubstrate.origin, input.origin),
        eq(factoryArtifactSubstrate.status, "active"),
        isNotNull(factoryArtifactSubstrate.userBody),
      ),
    )
    .orderBy(factoryArtifactSubstrate.path);

  return assembleApprovalSummaryFromFacts({
    gatePredicate: input.gatePredicate,
    actorId: input.actorId,
    assembledAt: new Date().toISOString(),
    admission: {
      admitted: verdict.admitted,
      reason: verdict.reason,
      envelopeHash: state.envelopeHash,
      composed: state.composed,
    },
    provenanceRows,
    overrideRows,
  });
}
