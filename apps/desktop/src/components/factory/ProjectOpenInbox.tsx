// SPDX-License-Identifier: AGPL-3.0-or-later
// Spec 112 §6.3 — Open-in-OPC inbox banner.
//
// Renders a dismissible banner when stagecraft hands off a project to OPC
// via an oap:// deep link. "Resolve" calls the bundle endpoint and shows
// what OPC received: project, adapter, contract / process / agent counts.
// The local clone + cockpit activation are separate next-step concerns.

import React from 'react';
import { Inbox, X, RefreshCw, AlertCircle, ExternalLink } from 'lucide-react';
import { Card } from '@opc/ui/card';
import { Badge } from '@opc/ui/badge';
import { Button } from '@opc/ui/button';
import { useProjectOpenInbox } from '@/hooks/useProjectOpenInbox';

export const ProjectOpenInbox: React.FC = () => {
  const inbox = useProjectOpenInbox();
  const { pending, bundle, bundleLoading, bundleError, fetchBundle, dismiss } = inbox;

  if (!pending) return null;

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
