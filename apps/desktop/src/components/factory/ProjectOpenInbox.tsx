// SPDX-License-Identifier: AGPL-3.0-or-later
// Spec 112 §6.3 — Open-in-OPC inbox banner.
//
// Renders a dismissible banner when stagecraft hands off a project to OPC
// via an oap:// deep link. "Resolve" calls the bundle endpoint and shows
// what OPC received: project, adapter, contract / process / agent counts.
// The local clone + cockpit activation are separate next-step concerns.

import React, { useEffect, useState } from 'react';
import {
  Inbox,
  X,
  RefreshCw,
  AlertCircle,
  ExternalLink,
  FolderDown,
  CheckCircle2,
} from 'lucide-react';
import { Card } from '@opc/ui/card';
import { Badge } from '@opc/ui/badge';
import { Button } from '@opc/ui/button';
import { api } from '@/lib/api';
import { useProjectOpenInbox } from '@/hooks/useProjectOpenInbox';

const PROJECTS_SUBDIR = 'oap-projects';

function joinPath(parts: string[]): string {
  return parts.filter(Boolean).join('/').replace(/\/+/g, '/');
}

export const ProjectOpenInbox: React.FC = () => {
  const inbox = useProjectOpenInbox();
  const {
    pending,
    bundle,
    bundleLoading,
    bundleError,
    clone,
    fetchBundle,
    cloneProject,
    dismiss,
  } = inbox;

  const [homeDir, setHomeDir] = useState<string | null>(null);
  useEffect(() => {
    void api
      .getHomeDirectory()
      .then((p) => setHomeDir(p))
      .catch(() => setHomeDir(null));
  }, []);

  if (!pending) return null;

  const targetDir = bundle && homeDir
    ? joinPath([homeDir, PROJECTS_SUBDIR, bundle.project.slug])
    : null;

  return (
    <Card className="mx-3 my-2 p-3 border-indigo-500/40 bg-indigo-500/5">
      <div className="flex items-start gap-3">
        <Inbox className="h-4 w-4 text-indigo-500 mt-0.5 shrink-0" aria-hidden="true" />
        <div className="flex-1 min-w-0 space-y-1.5">
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-sm font-medium">Project handoff from stagecraft</span>
            {pending.level && (
              <Badge variant="outline" className="text-xs">
                {pending.level.replace(/_/g, ' ')}
              </Badge>
            )}
          </div>

          <div className="text-xs text-muted-foreground font-mono break-all">
            {pending.cloneUrl}
          </div>

          {bundleError && (
            <div className="flex items-start gap-1.5 text-xs text-destructive">
              <AlertCircle className="h-3.5 w-3.5 mt-0.5 shrink-0" />
              <span className="break-words">{bundleError}</span>
            </div>
          )}

          {bundle && (
            <div className="text-xs text-muted-foreground space-y-0.5 pt-1 border-t border-border/40 mt-2">
              <div>
                <span className="text-foreground">Project:</span> {bundle.project.name}
                <span className="font-mono ml-1.5">({bundle.project.slug})</span>
              </div>
              {bundle.adapter && (
                <div>
                  <span className="text-foreground">Adapter:</span>{' '}
                  <span className="font-mono">
                    {bundle.adapter.name} @ {bundle.adapter.version}
                  </span>
                </div>
              )}
              <div className="flex gap-3 pt-0.5">
                <span>{bundle.contracts.length} contracts</span>
                <span>·</span>
                <span>{bundle.processes.length} processes</span>
                <span>·</span>
                <span>{bundle.agents.length} agents</span>
              </div>
            </div>
          )}

          {clone.error && (
            <div className="flex items-start gap-1.5 text-xs text-destructive">
              <AlertCircle className="h-3.5 w-3.5 mt-0.5 shrink-0" />
              <span className="break-words">{clone.error}</span>
            </div>
          )}

          {clone.path && (
            <div className="flex items-start gap-1.5 text-xs text-emerald-600 dark:text-emerald-400">
              <CheckCircle2 className="h-3.5 w-3.5 mt-0.5 shrink-0" />
              <div className="space-y-0.5 min-w-0">
                <div>
                  {clone.alreadyCloned ? 'Already cloned at' : 'Cloned to'}
                </div>
                <div className="font-mono text-muted-foreground break-all">
                  {clone.path}
                </div>
              </div>
            </div>
          )}

          <div className="flex items-center gap-2 pt-1">
            {!bundle && (
              <Button
                size="sm"
                variant="default"
                onClick={() => void fetchBundle()}
                disabled={bundleLoading}
              >
                {bundleLoading ? (
                  <RefreshCw className="h-3.5 w-3.5 mr-1.5 animate-spin" />
                ) : (
                  <ExternalLink className="h-3.5 w-3.5 mr-1.5" />
                )}
                Resolve bundle
              </Button>
            )}
            {bundle && !clone.path && (
              <Button
                size="sm"
                variant="default"
                onClick={() => targetDir && void cloneProject(targetDir)}
                disabled={!targetDir || clone.loading}
                title={
                  targetDir ?? 'Waiting for home directory…'
                }
              >
                {clone.loading ? (
                  <RefreshCw className="h-3.5 w-3.5 mr-1.5 animate-spin" />
                ) : (
                  <FolderDown className="h-3.5 w-3.5 mr-1.5" />
                )}
                Clone locally
              </Button>
            )}
            <Button size="sm" variant="ghost" onClick={dismiss}>
              <X className="h-3.5 w-3.5 mr-1.5" />
              Dismiss
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
};

export default ProjectOpenInbox;
