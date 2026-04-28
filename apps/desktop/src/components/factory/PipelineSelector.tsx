// Spec: specs/076-factory-desktop-panel/spec.md
// Pipeline selector — start a new pipeline or display the running run ID.

import React, { useEffect, useState } from 'react';
import { FolderOpen, Play, Square } from 'lucide-react';
import { Button } from '@opc/ui/button';
import { Input } from '@opc/ui/input';
import { open } from '@tauri-apps/plugin-dialog';
import { exists, readDir } from '@tauri-apps/plugin-fs';
import type { OpcBundle } from '@/types/factoryBundle';
import { useFactoryPipeline } from './FactoryPipelineContext';

const ADAPTER_FALLBACK = 'next-prisma';
const RAW_ARTIFACTS_SUBDIR = '.artifacts/raw';
const BUSINESS_DOC_EXTENSIONS = new Set(['md', 'txt', 'pdf', 'docx']);

interface PipelineSelectorProps {
  projectPath?: string;
  bundle?: OpcBundle;
}

function joinPath(parent: string, child: string): string {
  const sep = parent.includes('\\') && !parent.includes('/') ? '\\' : '/';
  return parent.endsWith(sep) ? `${parent}${child}` : `${parent}${sep}${child}`;
}

function hasBusinessDocExt(name: string): boolean {
  const dot = name.lastIndexOf('.');
  if (dot < 0) return false;
  return BUSINESS_DOC_EXTENSIONS.has(name.slice(dot + 1).toLowerCase());
}

export const PipelineSelector: React.FC<PipelineSelectorProps> = ({
  projectPath,
  bundle,
}) => {
  const { state, startPipeline, cancelPipeline } = useFactoryPipeline();

  const initialAdapter = bundle?.adapter?.name ?? ADAPTER_FALLBACK;
  const [adapterName, setAdapterName] = useState(initialAdapter);
  const [businessDocs, setBusinessDocs] = useState<string[]>([]);
  const [rawArtifactsDir, setRawArtifactsDir] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);

  // Resync adapter when the bundle resolves (e.g. after deep-link handoff).
  useEffect(() => {
    const next = bundle?.adapter?.name;
    if (next) setAdapterName(next);
  }, [bundle?.adapter?.name]);

  // Auto-discover Business Documents from the project's hydrated raw
  // artefacts. Spec 113 / spec 087: clone & import write business inputs
  // into `<project>/.artifacts/raw/`. Pre-populating that here means a
  // freshly cloned project can start its pipeline without manual picking.
  useEffect(() => {
    let cancelled = false;
    if (!projectPath) {
      setBusinessDocs([]);
      setRawArtifactsDir(null);
      return;
    }
    const dir = joinPath(projectPath, RAW_ARTIFACTS_SUBDIR);
    (async () => {
      try {
        if (!(await exists(dir))) {
          if (!cancelled) {
            setRawArtifactsDir(null);
            setBusinessDocs([]);
          }
          return;
        }
        const entries = await readDir(dir);
        const picks: string[] = [];
        for (const entry of entries) {
          if (entry.isDirectory) continue;
          if (!hasBusinessDocExt(entry.name)) continue;
          picks.push(joinPath(dir, entry.name));
        }
        if (!cancelled) {
          setRawArtifactsDir(dir);
          setBusinessDocs(picks);
        }
      } catch {
        if (!cancelled) {
          setRawArtifactsDir(null);
          setBusinessDocs([]);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [projectPath]);

  const handlePickDocs = async () => {
    // Open the picker inside `.artifacts/raw/` when present so the native
    // macOS dialog displays the hydrated documents directly. The dot-prefix
    // on the parent is irrelevant once the dialog is already inside it.
    const defaultPath = rawArtifactsDir ?? projectPath ?? undefined;
    const selected = await open({
      multiple: true,
      defaultPath,
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
      await startPipeline(
        projectPath ?? '',
        adapterName.trim() || ADAPTER_FALLBACK,
        businessDocs,
      );
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
          placeholder={ADAPTER_FALLBACK}
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
