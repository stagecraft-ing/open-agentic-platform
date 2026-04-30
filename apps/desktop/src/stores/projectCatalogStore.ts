// SPDX-License-Identifier: AGPL-3.0-or-later
// Spec 112 §7 / Phase 8 — workspace project catalog store.
//
// Subscribes to the `project-catalog-upsert` Tauri event emitted by
// `commands::project_catalog_sync` and maintains an in-memory list of
// projects keyed on `projectId`. Tombstones drop the row.
//
// Restart and reconnect both replay through the duplex handshake
// snapshot, so the store does not need to persist — a missed upsert
// becomes a clean rebuild on the next handshake.

import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import type { ProjectCatalogEntry } from '@/routes/factory/ProjectsPanel';

interface ProjectCatalogUpsertEventPayload {
  projectId: string;
  orgId: string;
  name: string;
  slug: string;
  description: string;
  factoryAdapterId: string | null;
  detectionLevel: ProjectCatalogEntry['detectionLevel'];
  repo: ProjectCatalogEntry['repo'];
  opcDeepLink: string;
  tombstone: boolean;
  updatedAt: string;
}

interface ProjectCatalogState {
  /** Map of projectId -> entry. Kept as an object for fast lookups; the
   *  panel sorts when it renders. */
  byId: Record<string, ProjectCatalogEntry>;
  /** Set when the desktop has received at least one upsert (or an
   *  empty handshake snapshot). Lets the panel distinguish "no
   *  projects yet" from "haven't heard from stagecraft yet". */
  hydrated: boolean;
  applyUpsert: (payload: ProjectCatalogUpsertEventPayload) => void;
  reset: () => void;
}

export const useProjectCatalogStore = create<ProjectCatalogState>()(
  subscribeWithSelector((set) => ({
    byId: {},
    hydrated: false,
    applyUpsert: (payload) =>
      set((state) => {
        if (payload.tombstone) {
          if (!state.byId[payload.projectId]) {
            return { hydrated: true };
          }
          const next = { ...state.byId };
          delete next[payload.projectId];
          return { byId: next, hydrated: true };
        }
        const entry: ProjectCatalogEntry = {
          projectId: payload.projectId,
          orgId: payload.orgId,
          name: payload.name,
          slug: payload.slug,
          description: payload.description,
          factoryAdapterId: payload.factoryAdapterId,
          detectionLevel: payload.detectionLevel,
          repo: payload.repo,
          opcDeepLink: payload.opcDeepLink,
          updatedAt: payload.updatedAt,
          localPath: state.byId[payload.projectId]?.localPath,
        };
        return {
          byId: { ...state.byId, [payload.projectId]: entry },
          hydrated: true,
        };
      }),
    reset: () => set({ byId: {}, hydrated: false }),
  }))
);

/**
 * Subscribe to the Tauri event stream and dispatch upserts into the
 * store. Returns an unsubscribe function. No-op outside Tauri so the
 * webview build doesn't crash on `import("@tauri-apps/api/event")`.
 */
export async function subscribeProjectCatalog(): Promise<() => void> {
  if (!(window as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    return () => {};
  }
  const { listen } = await import('@tauri-apps/api/event');
  const apply = useProjectCatalogStore.getState().applyUpsert;
  const unlisten = await listen<ProjectCatalogUpsertEventPayload>(
    'project-catalog-upsert',
    (event) => {
      apply(event.payload);
    }
  );
  return unlisten;
}

/** Selector helper — the panel wants a sorted array, not the map. */
export function selectProjectsList(state: ProjectCatalogState): ProjectCatalogEntry[] {
  return Object.values(state.byId);
}
