import type { GovernanceState } from './useGovernanceStatus';
import { useGitCtxResourceEnrichment } from '../git/useGitCtxEnrichment';

/**
 * Same T006 gitctx MCP bridge as the git panel, layered on governance overview once loaded.
 * Registry + featuregraph remain authoritative; this is additive GitHub/gitctx context.
 */
export function useGovernanceCtxEnrichment(repoRoot: string, state: GovernanceState) {
  const shouldFetch =
    !!repoRoot.trim() && (state.status === 'success' || state.status === 'degraded');
  return useGitCtxResourceEnrichment(repoRoot, shouldFetch);
}
