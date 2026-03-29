import { useEffect, useMemo, useState } from 'react';
import { createMcpClient } from '@opc/mcp-client';
import type { GitContextViewState, GitCtxEnrichment } from './types';

export type GitCtxEnrichmentState =
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'success'; data: GitCtxEnrichment }
  | { status: 'degraded'; message: string };

const RESOURCE_URI = 'gitctx://context/current';

function parseResource(result: unknown): GitCtxEnrichment {
  const root = typeof result === 'object' && result !== null ? (result as Record<string, unknown>) : {};
  const contents = Array.isArray(root.contents) ? root.contents : [];
  const first = (contents[0] ?? {}) as Record<string, unknown>;
  const text = typeof first.text === 'string' ? first.text : null;
  if (!text) {
    throw new Error('gitctx resource response missing text content');
  }
  const parsed = JSON.parse(text) as Partial<GitCtxEnrichment>;
  return {
    authenticated: Boolean(parsed.authenticated),
    status: typeof parsed.status === 'string' ? parsed.status : 'unknown',
    repository: parsed.repository ?? null,
    current_branch: typeof parsed.current_branch === 'string' ? parsed.current_branch : null,
    current_path: typeof parsed.current_path === 'string' ? parsed.current_path : null,
  };
}

/**
 * Fetch `gitctx://context/current` when `shouldFetch` is true (Rust-owned MCP bridge / T006).
 * Shared by the git context panel and governance panel — additive only; callers own base state.
 */
export function useGitCtxResourceEnrichment(repoPath: string, shouldFetch: boolean): GitCtxEnrichmentState {
  const client = useMemo(() => createMcpClient('gitctx'), []);
  const [state, setState] = useState<GitCtxEnrichmentState>({ status: 'idle' });

  useEffect(() => {
    let active = true;

    async function load() {
      if (!repoPath.trim() || !shouldFetch) {
        setState({ status: 'idle' });
        return;
      }

      setState({ status: 'loading' });
      try {
        const resource = await client.readResource(RESOURCE_URI);
        if (!active) return;
        const data = parseResource(resource);
        setState({ status: 'success', data });
      } catch (e) {
        if (!active) return;
        const message = e instanceof Error ? e.message : String(e);
        setState({ status: 'degraded', message: `gitctx enrichment unavailable: ${message}` });
      }
    }

    void load();
    return () => {
      active = false;
    };
  }, [client, repoPath, shouldFetch]);

  return state;
}

/**
 * Additive gitctx enrichment layered on native git context (PR-4 / T006).
 */
export function useGitCtxEnrichment(repoPath: string, baseState: GitContextViewState): GitCtxEnrichmentState {
  const shouldFetch =
    !!repoPath.trim() && (baseState.status === 'success' || baseState.status === 'degraded');
  return useGitCtxResourceEnrichment(repoPath, shouldFetch);
}
