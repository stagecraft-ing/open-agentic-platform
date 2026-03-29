import React, { useState } from 'react';
import { AlertCircle, AlertTriangle, Loader2 } from 'lucide-react';
import { Button } from '@opc/ui/button';
import { useTabState } from '@/hooks/useTabState';
import { RegistrySpecFollowUp } from '@/features/inspect/RegistrySpecFollowUp';
import { useGovernanceCtxEnrichment } from './useGovernanceCtxEnrichment';
import { useGovernanceStatus } from './useGovernanceStatus';

export const GovernanceSurface: React.FC = () => {
  const [repoRoot, setRepoRoot] = useState('');
  const { createSpecMarkdownTab } = useTabState();
  const { state, load, reset } = useGovernanceStatus();
  const enrichmentPath =
    state.status === 'success' || state.status === 'degraded' ? state.data.repoRoot : repoRoot;
  const enrichment = useGovernanceCtxEnrichment(enrichmentPath, state);
  const busy = state.status === 'loading';

  return (
    <div className="p-6 h-full flex flex-col gap-4 text-foreground">
      <header className="flex flex-col gap-1">
        <h1 className="text-2xl font-bold">Featuregraph Governance</h1>
        <p className="text-sm text-muted-foreground">
          Read governance state from compiled registry and featuregraph outputs. Optional gitctx MCP
          enrichment (same T006 bridge as the git panel) is additive only.
        </p>
      </header>

      <div className="flex gap-2 items-center">
        <input
          className="flex-1 px-3 py-2 bg-background border border-input rounded-md text-foreground"
          value={repoRoot}
          onChange={(e) => setRepoRoot(e.target.value)}
          placeholder="Repository root path (leave empty to use current working directory)"
          disabled={busy}
          aria-label="Repository root for governance overview"
        />
        <Button onClick={() => void load(repoRoot)} disabled={busy}>
          {busy ? (
            <span className="inline-flex items-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
              Loading...
            </span>
          ) : (
            'Load governance'
          )}
        </Button>
        {state.status !== 'idle' && state.status !== 'loading' && (
          <Button variant="outline" type="button" onClick={reset} disabled={busy}>
            Clear
          </Button>
        )}
      </div>

      <div className="flex-1 min-h-0 flex flex-col border rounded-md bg-muted/40">
        {state.status === 'idle' && (
          <div className="flex-1 flex items-center justify-center p-6 text-center text-muted-foreground text-sm">
            Load governance overview to hydrate registry and featuregraph panels.
          </div>
        )}

        {state.status === 'loading' && (
          <div className="flex-1 flex flex-col items-center justify-center gap-3 p-6 text-muted-foreground">
            <Loader2 className="h-8 w-8 animate-spin" aria-hidden />
            <span className="text-sm">Reading governance sources...</span>
          </div>
        )}

        {state.status === 'error' && (
          <div className="flex-1 flex flex-col gap-2 p-4 border border-destructive/50 rounded-md m-4 bg-background">
            <div className="flex items-center gap-2 text-destructive font-medium">
              <AlertCircle className="h-5 w-5 shrink-0" aria-hidden />
              Governance load failed
            </div>
            <pre className="text-sm whitespace-pre-wrap font-mono text-foreground">{state.message}</pre>
          </div>
        )}

        {(state.status === 'success' || state.status === 'degraded') && (
          <div className="flex-1 min-h-0 overflow-auto p-4 m-4 bg-background border rounded-md text-foreground flex flex-col gap-3">
            {state.status === 'degraded' && (
              <div className="flex items-center gap-2 text-amber-700 dark:text-amber-400 font-medium">
                <AlertTriangle className="h-5 w-5 shrink-0" aria-hidden />
                Degraded governance state: {state.reason}
              </div>
            )}

            <div className="border rounded-md p-3">
              <div className="text-xs text-muted-foreground">Repository root</div>
              <div className="font-mono text-xs break-all">{state.data.repoRoot}</div>
            </div>

            <div className="border rounded-md p-3 bg-background text-sm">
              <h2 className="font-semibold mb-2">GitHub context (gitctx via MCP bridge)</h2>
              {enrichment.status === 'idle' && (
                <p className="text-muted-foreground text-xs">Enrichment idle.</p>
              )}
              {enrichment.status === 'loading' && (
                <p className="text-muted-foreground text-xs">Loading gitctx enrichment…</p>
              )}
              {enrichment.status === 'degraded' && (
                <p className="text-muted-foreground text-xs">{enrichment.message}</p>
              )}
              {enrichment.status === 'success' && (
                <dl className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs">
                  <div>
                    <dt className="text-muted-foreground uppercase tracking-wide">Repository</dt>
                    <dd className="font-mono mt-1">
                      {enrichment.data.repository?.full_name ?? 'none selected'}
                    </dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground uppercase tracking-wide">Authenticated</dt>
                    <dd className="mt-1">{enrichment.data.authenticated ? 'yes' : 'no'}</dd>
                  </div>
                </dl>
              )}
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div className="border rounded-md p-3">
                <div className="text-sm font-medium mb-2">Compiled registry</div>
                <div className="text-xs text-muted-foreground mb-2">Status: {state.data.registry.status}</div>
                <div className="font-mono text-xs break-all mb-2">{state.data.registry.path}</div>
                {state.data.registry.summary ? (
                  <div className="text-xs space-y-1">
                    <div>Features: {state.data.registry.summary.featureCount}</div>
                    <div>Validation passed: {String(state.data.registry.summary.validationPassed)}</div>
                    <div>Violations: {state.data.registry.summary.violationsCount}</div>
                    <div className="pt-1">
                      Status counts:{' '}
                      {Object.entries(state.data.registry.summary.statusCounts)
                        .map(([k, v]) => `${k}=${v}`)
                        .join(', ') || 'none'}
                    </div>
                  </div>
                ) : (
                  <div className="text-xs text-muted-foreground">{state.data.registry.message ?? 'Unavailable'}</div>
                )}
              </div>

              <div className="border rounded-md p-3">
                <div className="text-sm font-medium mb-2">Featuregraph</div>
                <div className="text-xs text-muted-foreground mb-2">Status: {state.data.featuregraph.status}</div>
                {state.data.featuregraph.summary ? (
                  <div className="text-xs space-y-1">
                    <div>Features: {state.data.featuregraph.summary.featureCount}</div>
                    <div>Violations: {state.data.featuregraph.summary.violationsCount}</div>
                  </div>
                ) : (
                  <div className="text-xs text-muted-foreground">
                    {state.data.featuregraph.message ?? 'Unavailable'}
                  </div>
                )}
              </div>
            </div>

            <RegistrySpecFollowUp
              repoRoot={state.data.repoRoot}
              registry={state.data.registry}
              onViewSpec={(abs, title) => {
                void createSpecMarkdownTab(abs, title);
              }}
            />
          </div>
        )}
      </div>
    </div>
  );
};
