// Spec 115 FR-017 — workspace policy slice that gates extractor invocations.
//
// Phase 0 ships the type, the deterministic-only fallback, and a stub
// `resolveExtractionPolicy` that always returns the fallback. Phase 2
// (task T040 + T041) replaces the stub with a real loader that reads
// `build/policy/workspaces/{workspaceId}.json` (compiled by the
// policy-kernel) with a 30s in-memory cache.
//
// The fallback is what brand-new workspaces and bundle-pending workspaces
// receive (spec §4 edge case "No policy bundle resolved for the
// workspace"). Vision off, audio off, $0 ceilings — agent extractors
// fail closed under it.

// ---------------------------------------------------------------------------
// Type
// ---------------------------------------------------------------------------

export type ExtractionPolicy = {
  visionAllowed: boolean;
  audioAllowed: boolean;
  /**
   * Pinned model id for agent extractors. When unset, the agent extractor
   * falls back to its own default. Pinning makes `promptFingerprint`
   * stable across workspaces.
   */
  modelPin?: string;
  /** Per-call USD ceiling (FR-018). Pre-flight estimate must fit under this. */
  costCeilingUsdPerCall: number;
  /** Per-day USD ceiling (FR-019). Day-aggregate must stay under this. */
  costCeilingUsdPerDay: number;
  /**
   * Provenance — used in audit metadata so reviewers can identify when the
   * deterministic-only fallback was applied versus a real compiled bundle.
   */
  source: "compiled_bundle" | "default_fallback";
};

// ---------------------------------------------------------------------------
// Default fallback (deterministic-only)
// ---------------------------------------------------------------------------

export const DEFAULT_DETERMINISTIC_ONLY_POLICY: ExtractionPolicy = {
  visionAllowed: false,
  audioAllowed: false,
  costCeilingUsdPerCall: 0,
  costCeilingUsdPerDay: 0,
  source: "default_fallback",
};

// ---------------------------------------------------------------------------
// Resolver (Phase 0 stub)
// ---------------------------------------------------------------------------

/**
 * Returns the policy slice for a workspace. Phase 0 always returns the
 * deterministic-only fallback regardless of workspaceId. Phase 2 replaces
 * this with the real bundle loader.
 *
 * Async signature is intentional even at the stub stage so callers don't
 * have to migrate when the real loader (which reads from disk + caches)
 * lands.
 */
export async function resolveExtractionPolicy(
  _workspaceId: string,
): Promise<ExtractionPolicy> {
  return DEFAULT_DETERMINISTIC_ONLY_POLICY;
}
