import { useEffect, useMemo, useState } from 'react';
import { createMcpClient } from '@opc/mcp-client';
import type { GitContextViewState, GitCtxEnrichment } from './types';

type EnrichmentState =
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
 * Additive gitctx enrichment via Rust-owned MCP bridge (per-request stdio to gitctx-mcp).
 * Readiness is the outcome of `readResource`, not sidecar port discovery.
 */
export function useGitCtxEnrichment(repoPath: string, baseState: GitContextViewState): EnrichmentState {
  const client = useMemo(() => createMcpClient('gitctx'), []);
  const [state, setState] = useState<EnrichmentState>({ status: 'idle' });

  useEffect(() => {
    let active = true;

    async function load() {
      if (!repoPath.trim()) {
        setState({ status: 'idle' });
        return;
      }
      if (baseState.status !== 'success' && baseState.status !== 'degraded') {
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
  }, [baseState.status, client, repoPath]);

  return state;
}
