/**
 * Spec 201 FR-002 — fail-closed approve-control decision logic for the
 * override-verify surface, as a pure function so the contract is unit
 * tested without DOM infrastructure (same posture as
 * `factory-run-helpers.ts`). The JSX in `app.factory.artifacts.tsx` stays
 * a thin renderer over the returned state.
 */

import type {
  ApprovalSummaryWire,
  ArtifactApprovalSummaryResponse,
} from "./factory-api.server";

export type ApprovalControlState =
  /** No override, or the current revision is already verified — no
   * verify control, no basis panel. */
  | { kind: "hidden" }
  /** Spec 111 user-authored trust class — verify keeps its legacy
   * presentation; no envelope-grounded basis exists (FR-004 scoping). */
  | { kind: "legacy-verify" }
  /** Basis unavailable — render the attributable reason and NO verify
   * control (FR-002 fail-closed). */
  | { kind: "blocked"; reason: string }
  /** Basis assembled — render the fact sections, then the verify control
   * in a distinct subtree (FR-005). `unverifiedPaths` lists overrides in
   * scope that an envelope predicate leaves unsatisfied — informative on
   * this surface (verify IS the resolution path), blocking on approve
   * surfaces (phase 3). */
  | {
      kind: "verify-with-basis";
      summary: ApprovalSummaryWire;
      unverifiedPaths: string[];
    };

export function approvalControlState(
  detail: { userBody: string | null; overrideVerified: boolean | null },
  approval: ArtifactApprovalSummaryResponse | null,
): ApprovalControlState {
  if (detail.userBody === null || detail.overrideVerified === true) {
    return { kind: "hidden" };
  }
  if (approval === null) {
    // The loader fetches the basis whenever the verify control would
    // render; a missing response is a failed fetch — fail closed.
    return {
      kind: "blocked",
      reason:
        "approval basis could not be loaded — the verify control is " +
        "withheld until the summary endpoint responds (spec 201 FR-002)",
    };
  }
  if (!approval.applicable) {
    return { kind: "legacy-verify" };
  }
  if (approval.ok !== true || !approval.summary) {
    return {
      kind: "blocked",
      reason:
        approval.reason ??
        "approval basis refused without a stated reason (spec 201 FR-002)",
    };
  }
  return {
    kind: "verify-with-basis",
    summary: approval.summary,
    unverifiedPaths: approval.summary.consumedOverrides
      .filter((o) => !o.requireVerifiedSatisfied)
      .map((o) => o.path),
  };
}
