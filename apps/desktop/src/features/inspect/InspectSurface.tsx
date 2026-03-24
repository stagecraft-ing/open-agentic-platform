import React, { useState } from 'react';
import { AlertCircle, AlertTriangle, Loader2 } from 'lucide-react';
import { Button } from '@opc/ui/button';
import { useInspectFlow } from './useInspectFlow';

/**
 * Feature 032 — T003: inspect shell for xray scan (explicit loading / success / error / degraded).
 */
export const InspectSurface: React.FC = () => {
  const [path, setPath] = useState('');
  const { state, scan, reset } = useInspectFlow();

  const handleScan = () => {
    void scan(path);
  };

  const busy = state.status === 'loading';

  return (
    <div className="p-6 h-full flex flex-col gap-4 text-foreground">
      <header className="flex flex-col gap-1">
        <h1 className="text-2xl font-bold">Inspect — Xray architecture analysis</h1>
        <p className="text-sm text-muted-foreground">
          Scan a project directory to produce a deterministic architecture index (Feature 032 T003).
        </p>
      </header>

      <div className="flex gap-2 items-center">
        <input
          className="flex-1 px-3 py-2 bg-background border border-input rounded-md text-foreground"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="Absolute path to project root…"
          disabled={busy}
          aria-label="Project path for inspect scan"
        />
        <Button onClick={handleScan} disabled={busy || !path.trim()}>
          {busy ? (
            <span className="inline-flex items-center gap-2">
              <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
              Scanning…
            </span>
          ) : (
            'Scan project'
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
            Enter an absolute project path and run a scan to load inspect results.
          </div>
        )}

        {state.status === 'loading' && (
          <div
            className="flex-1 flex flex-col items-center justify-center gap-3 p-6 text-muted-foreground"
            role="status"
            aria-live="polite"
          >
            <Loader2 className="h-8 w-8 animate-spin" aria-hidden />
            <span className="text-sm">Running xray scan…</span>
          </div>
        )}

        {state.status === 'error' && (
          <div
            className="flex-1 flex flex-col gap-2 p-4 border border-destructive/50 rounded-md m-4 bg-background"
            role="alert"
          >
            <div className="flex items-center gap-2 text-destructive font-medium">
              <AlertCircle className="h-5 w-5 shrink-0" aria-hidden />
              Scan failed
            </div>
            <pre className="text-sm whitespace-pre-wrap font-mono text-foreground">{state.message}</pre>
          </div>
        )}

        {state.status === 'degraded' && (
          <div className="flex-1 flex flex-col gap-2 p-4 m-4 border border-amber-500/50 rounded-md bg-amber-500/5">
            <div className="flex items-center gap-2 text-amber-700 dark:text-amber-400 font-medium">
              <AlertTriangle className="h-5 w-5 shrink-0" aria-hidden />
              Degraded result
            </div>
            <p className="text-sm text-muted-foreground">{state.reason}</p>
            <div className="flex-1 overflow-auto max-h-[50vh] bg-muted p-3 rounded border text-foreground">
              <pre className="text-xs whitespace-pre-wrap font-mono">
                {JSON.stringify(state.payload, null, 2)}
              </pre>
            </div>
          </div>
        )}

        {state.status === 'success' && (
          <div className="flex-1 overflow-auto p-4 m-4 bg-background border rounded-md text-foreground">
            <pre className="text-sm whitespace-pre-wrap font-mono">
              {JSON.stringify(state.payload, null, 2)}
            </pre>
          </div>
        )}
      </div>
    </div>
  );
};
