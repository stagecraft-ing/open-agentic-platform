// SPDX-License-Identifier: AGPL-3.0-or-later
// Spec 112 §7 / Phase 8 — connects the duplex-synced project catalog
// store to the presentational ProjectsPanel and surfaces "open" /
// "clone & open" actions wired to the existing factory tab handler.

import React, { useEffect, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { Card } from '@opc/ui/card';
import { useTabState } from '@/hooks/useTabState';
import {
  selectProjectsList,
  subscribeProjectCatalog,
  useProjectCatalogStore,
} from '@/stores/projectCatalogStore';
import { ProjectsPanel, type ProjectCatalogEntry } from './ProjectsPanel';

// If the duplex hasn't pushed an upsert (including an empty handshake snapshot)
// within this window, fall back to a "not connected" message rather than
// spinning forever. Stagecraft may be offline, unreachable, or unconfigured.
const STAGECRAFT_STALL_TIMEOUT_MS = 5000;

export const WorkspaceProjectsPanel: React.FC = () => {
  const projects = useProjectCatalogStore(useShallow(selectProjectsList));
  const hydrated = useProjectCatalogStore((s) => s.hydrated);
  const { createFactoryTab } = useTabState();
  const [stalled, setStalled] = useState(false);

  // Subscribe to the duplex catalog stream once. The store is global, so the
  // subscription survives panel unmounts and remounts — but we still need
  // *some* component to wire it up at app load. The Projects tab is a fine
  // owner because it is the only consumer today.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void subscribeProjectCatalog().then((u) => {
      if (cancelled) {
        u();
      } else {
        unlisten = u;
      }
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (hydrated) {
      setStalled(false);
      return;
    }
    const timer = window.setTimeout(
      () => setStalled(true),
      STAGECRAFT_STALL_TIMEOUT_MS,
    );
    return () => window.clearTimeout(timer);
  }, [hydrated]);

  const handleOpen = (project: ProjectCatalogEntry) => {
    if (!project.localPath) return;
    createFactoryTab(project.localPath);
  };

  // Until §6.4 hands the desktop a clone token directly from the panel,
  // "Clone & open" routes the user through the deep-link path: opening the
  // opc:// URL triggers the same handoff that stagecraft's success page
  // uses, which already resolves the bundle, clones, and activates the
  // cockpit. Once a local clone path is recorded somewhere the desktop
  // owns, this handler can shortcut directly into the cockpit.
  const handleClone = (project: ProjectCatalogEntry) => {
    window.location.assign(project.opcDeepLink);
  };

  if (!hydrated) {
    if (stalled) {
      return (
        <div className="h-full flex items-center justify-center text-muted-foreground">
          <Card className="p-4 text-sm max-w-md space-y-2">
            <p className="font-medium text-foreground">
              No project list received from stagecraft.
            </p>
            <p>
              The workspace project list is broadcast by the stagecraft duplex
              stream. Either stagecraft is unreachable, the stagecraft base URL
              is not configured, or there are no projects in your org yet.
              Check Settings → Stagecraft and verify the desktop can reach it.
            </p>
          </Card>
        </div>
      );
    }
    return (
      <div className="h-full flex items-center justify-center text-muted-foreground">
        <Card className="p-4 text-sm">
          Connecting to stagecraft… the project list will arrive once the
          duplex handshake completes.
        </Card>
      </div>
    );
  }

  return (
    <ProjectsPanel
      projects={projects}
      onOpen={handleOpen}
      onClone={handleClone}
    />
  );
};

export default WorkspaceProjectsPanel;
