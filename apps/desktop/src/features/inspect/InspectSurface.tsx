import React, { useEffect, useRef, useState } from 'react';
import { AlertCircle, AlertTriangle, Loader2 } from 'lucide-react';
import { Button } from '@opc/ui/button';
import { useTabState } from '@/hooks/useTabState';
import { apiCall } from '@/lib/apiAdapter';
import type { GovernanceOverview } from '@/features/governance/useGovernanceStatus';
import { RegistrySpecFollowUp } from './RegistrySpecFollowUp';
import { useInspectFlow } from './useInspectFlow';

interface XrayFileNode {
  path?: string;
  size?: number;
  lang?: string;
  loc?: number;
}

interface XrayViewModel {
  digest?: string;
  root?: string;
  target?: string;
  fileCount: number;
  totalSize?: number;
  files: XrayFileNode[];
}

function toXrayViewModel(payload: unknown): XrayViewModel | null {
  if (!payload || typeof payload !== 'object') return null;
  const record = payload as Record<string, unknown>;
  const files = Array.isArray(record.files) ? (record.files as XrayFileNode[]) : [];
  const stats =
    record.stats && typeof record.stats === 'object'
      ? (record.stats as Record<string, unknown>)
      : undefined;

  return {
    digest: typeof record.digest === 'string' ? record.digest : undefined,
    root: typeof record.root === 'string' ? record.root : undefined,
    target: typeof record.target === 'string' ? record.target : undefined,
    fileCount:
      typeof stats?.fileCount === 'number'
        ? stats.fileCount
        : typeof stats?.file_count === 'number'
          ? stats.file_count
          : files.length,
    totalSize:
      typeof stats?.totalSize === 'number'
        ? stats.totalSize
        : typeof stats?.total_size === 'number'
          ? stats.total_size
          : undefined,
    files,
  };
}

interface InspectSurfaceProps {
  /** When provided, the panel pre-fills the path and auto-scans on mount. */
  projectPath?: string;
}

/**
 * Feature 032 — T003: inspect shell for xray scan (explicit loading / success / error / degraded).
 */
export const InspectSurface: React.FC<InspectSurfaceProps> = ({ projectPath }) => {
  const [path, setPath] = useState(projectPath ?? '');
  const { createSpecMarkdownTab } = useTabState();
  const [inspectFollowUp, setInspectFollowUp] = useState<GovernanceOverview | null>(null);
  const { state, scan, reset } = useInspectFlow();
  const autoLoaded = useRef(false);

  // Auto-scan when projectPath is provided
  useEffect(() => {
    if (projectPath && !autoLoaded.current) {
      autoLoaded.current = true;
      setPath(projectPath);
      void scan(projectPath);
    }
  }, [projectPath, scan]);

  useEffect(() => {
    if (state.status !== 'success' || !path.trim()) {
      setInspectFollowUp(null);
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const data = await apiCall<GovernanceOverview>('featuregraph_overview', {
          featuresYamlPath: path.trim(),
        });
        if (!cancelled) setInspectFollowUp(data);
      } catch {
        if (!cancelled) setInspectFollowUp(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [state.status, path]);

  const handleScan = () => {
    void scan(path);
  };

  const busy = state.status === 'loading';
  const successData = state.status === 'success' ? toXrayViewModel(state.payload) : null;
  const degradedData = state.status === 'degraded' ? toXrayViewModel(state.payload) : null;

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
            {degradedData ? (
              <>
                <div className="grid grid-cols-1 md:grid-cols-4 gap-2 text-sm">
                  <div className="border rounded-md bg-background p-3">
                    <div className="text-xs text-muted-foreground">Root</div>
                    <div className="font-mono break-all">{degradedData.root ?? 'n/a'}</div>
                  </div>
                  <div className="border rounded-md bg-background p-3">
                    <div className="text-xs text-muted-foreground">Target</div>
                    <div className="font-mono break-all">{degradedData.target ?? 'n/a'}</div>
                  </div>
                  <div className="border rounded-md bg-background p-3">
                    <div className="text-xs text-muted-foreground">Files</div>
                    <div>{degradedData.fileCount}</div>
                  </div>
                  <div className="border rounded-md bg-background p-3">
                    <div className="text-xs text-muted-foreground">Total bytes</div>
                    <div>{degradedData.totalSize ?? 'n/a'}</div>
                  </div>
                </div>
                <div className="border rounded-md bg-background p-3">
                  <div className="text-xs text-muted-foreground mb-2">Digest</div>
                  <div className="font-mono text-xs break-all">{degradedData.digest ?? 'n/a'}</div>
                </div>
              </>
            ) : (
              <div className="flex-1 overflow-auto max-h-[50vh] bg-muted p-3 rounded border text-foreground">
                <pre className="text-xs whitespace-pre-wrap font-mono">
                  {JSON.stringify(state.payload, null, 2)}
                </pre>
              </div>
            )}
          </div>
        )}

        {state.status === 'success' && (
          <div className="flex-1 min-h-0 flex flex-col gap-3 p-4 m-4 bg-background border rounded-md text-foreground">
            {!successData ? (
              <div className="flex items-center gap-2 text-amber-700 dark:text-amber-400">
                <AlertTriangle className="h-4 w-4" aria-hidden />
                Scan succeeded but payload format is unexpected.
              </div>
            ) : (
              <>
                <div className="grid grid-cols-1 md:grid-cols-4 gap-2 text-sm">
                  <div className="border rounded-md bg-muted/40 p-3">
                    <div className="text-xs text-muted-foreground">Root</div>
                    <div className="font-mono break-all">{successData.root ?? 'n/a'}</div>
                  </div>
                  <div className="border rounded-md bg-muted/40 p-3">
                    <div className="text-xs text-muted-foreground">Target</div>
                    <div className="font-mono break-all">{successData.target ?? 'n/a'}</div>
                  </div>
                  <div className="border rounded-md bg-muted/40 p-3">
                    <div className="text-xs text-muted-foreground">Files</div>
                    <div>{successData.fileCount}</div>
                  </div>
                  <div className="border rounded-md bg-muted/40 p-3">
                    <div className="text-xs text-muted-foreground">Total bytes</div>
                    <div>{successData.totalSize ?? 'n/a'}</div>
                  </div>
                </div>
                <div className="border rounded-md p-3">
                  <div className="text-xs text-muted-foreground mb-2">Digest</div>
                  <div className="font-mono text-xs break-all">{successData.digest ?? 'n/a'}</div>
                </div>
                {inspectFollowUp && (
                  <RegistrySpecFollowUp
                    repoRoot={inspectFollowUp.repoRoot}
                    registry={inspectFollowUp.registry}
                    onViewSpec={(abs, title) => {
                      void createSpecMarkdownTab(abs, title);
                    }}
                  />
                )}

                <div className="flex-1 min-h-0 border rounded-md">
                  <div className="px-3 py-2 border-b text-xs text-muted-foreground">
                    Indexed files ({successData.files.length})
                  </div>
                  <div className="max-h-[40vh] overflow-auto">
                    {successData.files.length === 0 ? (
                      <div className="p-3 text-sm text-muted-foreground">No files indexed.</div>
                    ) : (
                      <table className="w-full text-xs">
                        <thead className="sticky top-0 bg-background">
                          <tr className="border-b">
                            <th className="text-left font-medium p-2">Path</th>
                            <th className="text-left font-medium p-2">Lang</th>
                            <th className="text-right font-medium p-2">LOC</th>
                            <th className="text-right font-medium p-2">Bytes</th>
                          </tr>
                        </thead>
                        <tbody>
                          {successData.files.slice(0, 200).map((file, idx) => (
                            <tr key={`${file.path ?? 'unknown'}-${idx}`} className="border-b last:border-b-0">
                              <td className="p-2 font-mono break-all">{file.path ?? 'n/a'}</td>
                              <td className="p-2">{file.lang ?? 'n/a'}</td>
                              <td className="p-2 text-right">{file.loc ?? '-'}</td>
                              <td className="p-2 text-right">{file.size ?? '-'}</td>
                            </tr>
                          ))}
                        </tbody>
                      </table>
                    )}
                  </div>
                </div>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
