// Spec: specs/076-factory-desktop-panel/spec.md
// Top-level Factory pipeline panel registered in TabContent.

import React, { useState } from 'react';
import { Layers, History } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Badge } from '@opc/ui/badge';
import { Button } from '@opc/ui/button';
import { FactoryPipelineProvider, useFactoryPipeline } from './FactoryPipelineContext';
import { PipelineSelector } from './PipelineSelector';
import { PipelineDAG } from './PipelineDAG';
import { TokenDashboard } from './TokenDashboard';
import { ArtifactInspector } from './ArtifactInspector';
import { GateDialog } from './GateDialog';
import { ScaffoldMonitor } from './ScaffoldMonitor';
import { PipelineHistory } from './PipelineHistory';

// ── Status badge ─────────────────────────────────────────────────────────────

const PHASE_BADGE_VARIANT: Record<
  string,
  'default' | 'secondary' | 'destructive' | 'outline'
> = {
  idle: 'outline',
  process: 'default',
  scaffolding: 'default',
  complete: 'secondary',
  failed: 'destructive',
};

const PHASE_LABEL: Record<string, string> = {
  idle: 'Idle',
  process: 'Processing',
  scaffolding: 'Scaffolding',
  complete: 'Complete',
  failed: 'Failed',
};

type PanelView = 'pipeline' | 'history';

// ── Inner panel (unwrapped from provider) ────────────────────────────────────

function FactoryPipelinePanelInner({ projectPath }: { projectPath?: string }) {
  const { state } = useFactoryPipeline();
  const [view, setView] = useState<PanelView>('pipeline');

  const phaseVariant = PHASE_BADGE_VARIANT[state.phase] ?? 'outline';
  const phaseLabel = PHASE_LABEL[state.phase] ?? state.phase;

  const showScaffoldMonitor = state.phase === 'scaffolding' && state.scaffolding !== null;

  return (
    <div className="h-full flex flex-col text-foreground relative">
      {/* Header */}
      <header className="flex items-center gap-3 px-4 py-2.5 border-b border-border shrink-0">
        <Layers className="h-4 w-4 text-muted-foreground shrink-0" />
        <h1 className="text-sm font-semibold flex-1">Factory Pipeline</h1>
        {state.runId && (
          <span
            className="font-mono text-xs text-muted-foreground truncate max-w-[160px]"
            title={state.runId}
          >
            {state.runId.slice(0, 12)}…
          </span>
        )}
        <Badge variant={phaseVariant} className="shrink-0 text-xs">
          {phaseLabel}
        </Badge>
        <Button
          variant={view === 'history' ? 'secondary' : 'ghost'}
          size="icon"
          className="h-7 w-7"
          onClick={() => setView(view === 'history' ? 'pipeline' : 'history')}
          title="Pipeline history"
        >
          <History className="h-3.5 w-3.5" />
        </Button>
      </header>

      {/* View: History */}
      {view === 'history' && (
        <div className="flex-1 min-h-0 overflow-auto">
          <PipelineHistory projectPath={projectPath} />
        </div>
      )}

      {/* View: Pipeline */}
      {view === 'pipeline' && (
        <div className="flex-1 min-h-0 flex overflow-hidden">
          {/* Left pane — ~40% */}
          <div
            className={cn(
              'flex flex-col border-r border-border overflow-hidden',
              'w-[40%] min-w-[220px] max-w-[360px]',
            )}
          >
            {/* Selector at top */}
            <PipelineSelector projectPath={projectPath} />

            {/* DAG fills remaining height */}
            <PipelineDAG />

            {/* Token dashboard pinned to bottom */}
            <TokenDashboard compact />
          </div>

          {/* Right pane — ~60% */}
          <div className="flex-1 min-w-0 overflow-hidden flex flex-col">
            {/* Scaffold monitor shown during scaffolding phase */}
            {showScaffoldMonitor && (
              <div className="border-b border-border shrink-0 max-h-[40%] overflow-auto">
                <ScaffoldMonitor />
              </div>
            )}

            {/* Artifact inspector or empty state */}
            <div className="flex-1 min-h-0 overflow-hidden">
              {state.selectedStepId !== null ? (
                <ArtifactInspector />
              ) : (
                <div className="h-full flex flex-col items-center justify-center gap-2 text-muted-foreground p-6 text-center">
                  <Layers className="h-8 w-8 opacity-20" />
                  <p className="text-sm">
                    Select a pipeline stage to inspect its artifacts.
                  </p>
                </div>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Gate dialog overlay */}
      {state.gateAction !== null && <GateDialog />}
    </div>
  );
}

// ── Public export (wraps provider) ───────────────────────────────────────────

export const FactoryPipelinePanel: React.FC<{ projectPath?: string }> = ({
  projectPath,
}) => {
  return (
    <FactoryPipelineProvider>
      <FactoryPipelinePanelInner projectPath={projectPath} />
    </FactoryPipelineProvider>
  );
};
