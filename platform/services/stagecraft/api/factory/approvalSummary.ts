/**
 * Spec 201 FR-001 — server-side `ApprovalSummary` assembly from recorded
 * facts. DB half: fetches admission state + substrate rows and feeds the
 * pure assembly in `approvalSummary-pure.ts`.
 *
 * Read-only by contract (FR-003): no row writes, no model calls, no I/O
 * other than DB reads; the only clock-dependent value is `assembledAt`,
 * which is outside the hashed field set.
 */

import { and, eq, inArray, isNotNull } from "drizzle-orm";
import { db } from "../db/drizzle";
import { factoryArtifactSubstrate } from "../db/schema";
import { isFactoryAdmitted, loadLatestAdmission } from "./admission";
import {
  assembleApprovalSummaryFromFacts,
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
