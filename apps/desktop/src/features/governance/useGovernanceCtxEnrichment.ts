import type { GovernanceState } from './useGovernanceStatus';
import { useGitCtxResourceEnrichment } from '../git/useGitCtxEnrichment';

/**
 * Git context enrichment for the governance overview panel.
 * Registry + featuregraph remain authoritative; this is additive git context.
 *
 * @deprecated T006 gitctx MCP bridge (Phase 6): gitctx-mcp binary removed.
 * This hook will return empty results once the binary is absent.
 * Future: route through axiomregent github.* tools.
 */
export function useGovernanceCtxEnrichment(repoRoot: string, state: GovernanceState) {
  const shouldFetch =
    !!repoRoot.trim() && (state.status === 'success' || state.status === 'degraded');
  return useGitCtxResourceEnrichment(repoRoot, shouldFetch);
}
