// Spec: specs/076-factory-desktop-panel/spec.md
// Pipeline selector — start a new pipeline or display the running run ID.

import React, { useState } from 'react';
import { FolderOpen, Play, Square } from 'lucide-react';
import { Button } from '@opc/ui/button';
import { Input } from '@opc/ui/input';
import { open } from '@tauri-apps/plugin-dialog';
import { useFactoryPipeline } from './FactoryPipelineContext';

interface PipelineSelectorProps {
  projectPath?: string;
}

export const PipelineSelector: React.FC<PipelineSelectorProps> = ({ projectPath }) => {
  const { state, startPipeline, cancelPipeline } = useFactoryPipeline();

  const [adapterName, setAdapterName] = useState('next-prisma');
  const [businessDocs, setBusinessDocs] = useState<string[]>([]);
  const [starting, setStarting] = useState(false);

  const handlePickDocs = async () => {
    const selected = await open({
      multiple: true,
      defaultPath: projectPath || undefined,
      filters: [
        {
          name: 'Business Documents',
          extensions: ['md', 'txt', 'pdf', 'docx'],
        },
      ],
    });
    if (selected === null) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    setBusinessDocs(paths);
  };

  const handleStart = async () => {
    if (starting) return;
    setStarting(true);
    try {
      await startPipeline(projectPath ?? '', adapterName.trim() || 'next-prisma', businessDocs);
    } finally {
      setStarting(false);
    }
  };

  const isIdle = state.phase === 'idle';

  const [cancelling, setCancelling] = useState(false);

  const handleCancel = async () => {
    if (cancelling) return;
    setCancelling(true);
    try {
      await cancelPipeline('User cancelled from UI');
    } finally {
      setCancelling(false);
    }
  };

  if (!isIdle) {
    // Active pipeline — show run metadata + cancel button.
    const isActive = state.phase === 'process' || state.phase === 'scaffolding';
    return (
      <div className="flex items-center justify-between gap-2 px-3 py-2 border-b border-border">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-xs text-muted-foreground uppercase tracking-wide font-medium shrink-0">
            Run
          </span>
          <span className="font-mono text-xs truncate text-foreground">
            {state.runId ?? '—'}
          </span>
        </div>
        {isActive && (
          <Button
            variant="destructive"
            size="sm"
            className="h-6 text-xs gap-1 shrink-0"
            onClick={handleCancel}
            disabled={cancelling}
          >
            <Square className="h-3 w-3" />
            {cancelling ? 'Cancelling…' : 'Cancel'}
          </Button>
        )}
      </div>
    );
  }

  return (
    <div className="p-3 border-b border-border space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold text-foreground">Start New Pipeline</span>
      </div>

      <div className="space-y-1.5">
        <label className="text-xs text-muted-foreground">Adapter</label>
        <Input
          value={adapterName}
          onChange={(e) => setAdapterName(e.target.value)}
          placeholder="next-prisma"
          className="h-8 text-xs"
          disabled={starting}
        />
      </div>

      <div className="space-y-1.5">
        <label className="text-xs text-muted-foreground">Business Documents</label>
        <Button
          variant="outline"
          size="sm"
          className="w-full h-8 text-xs justify-start gap-2"
          onClick={handlePickDocs}
          disabled={starting}
        >
          <FolderOpen className="h-3.5 w-3.5 shrink-0" />
          {businessDocs.length === 0
            ? 'Pick files…'
            : `${businessDocs.length} file${businessDocs.length === 1 ? '' : 's'} selected`}
        </Button>
      </div>

      <Button
        className="w-full h-8 text-xs gap-2"
        onClick={handleStart}
        disabled={starting || !adapterName.trim()}
      >
        <Play className="h-3.5 w-3.5" />
        {starting ? 'Starting…' : 'Start Pipeline'}
      </Button>
    </div>
  );
};
