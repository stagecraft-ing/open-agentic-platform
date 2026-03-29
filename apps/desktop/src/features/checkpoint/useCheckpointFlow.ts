import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Checkpoint, CheckpointDiff, VerificationReport } from './types';

export type FlowStatus = 'idle' | 'initializing' | 'ready' | 'error';

export interface CheckpointFlowState {
  status: FlowStatus;
  projectRoot: string;
  checkpoints: Checkpoint[];
  error: string | null;
  /** Per-operation busy flags so the list remains visible during sub-ops. */
  busy: {
    creating: boolean;
    restoring: string | null; // checkpoint id being restored
    verifying: string | null;
    diffing: boolean;
  };
  /** Last diff result, cleared on new diff request. */
  diff: CheckpointDiff | null;
  /** Last verification result keyed by checkpoint id. */
  verifications: Record<string, VerificationReport>;
}

const INITIAL_BUSY = { creating: false, restoring: null, verifying: null, diffing: false };

const initialState: CheckpointFlowState = {
  status: 'idle',
  projectRoot: '',
  checkpoints: [],
  error: null,
  busy: { ...INITIAL_BUSY },
  diff: null,
  verifications: {},
};

export function useCheckpointFlow() {
  const [state, setState] = useState<CheckpointFlowState>(initialState);

  const setError = useCallback((error: string) => {
    setState(prev => ({ ...prev, status: 'error', error, busy: { ...INITIAL_BUSY } }));
  }, []);

  const refreshList = useCallback(async (rootPath: string) => {
    try {
      const list = await invoke<Checkpoint[]>('titor_list', { rootPath });
      // Sort most recent first
      const sorted = [...list].sort(
        (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
      );
      setState(prev => ({ ...prev, checkpoints: sorted }));
    } catch (err) {
      setError(String(err));
    }
  }, [setError]);

  const initialize = useCallback(async (rootPath: string) => {
    setState(prev => ({ ...prev, status: 'initializing', projectRoot: rootPath, error: null }));
    try {
      await invoke<string>('titor_init', { rootPath, storagePath: null });
      const list = await invoke<Checkpoint[]>('titor_list', { rootPath });
      const sorted = [...list].sort(
        (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
      );
      setState(prev => ({
        ...prev,
        status: 'ready',
        projectRoot: rootPath,
        checkpoints: sorted,
        error: null,
      }));
    } catch (err) {
      setError(String(err));
    }
  }, [setError]);

  const createCheckpoint = useCallback(async (message?: string) => {
    setState(prev => ({ ...prev, busy: { ...prev.busy, creating: true } }));
    try {
      await invoke<Checkpoint>('titor_checkpoint', {
        rootPath: state.projectRoot,
        message: message || null,
      });
      await refreshList(state.projectRoot);
    } catch (err) {
      setError(String(err));
    } finally {
      setState(prev => ({ ...prev, busy: { ...prev.busy, creating: false } }));
    }
  }, [state.projectRoot, refreshList, setError]);

  const restore = useCallback(async (checkpointId: string) => {
    setState(prev => ({ ...prev, busy: { ...prev.busy, restoring: checkpointId } }));
    try {
      await invoke('titor_restore', { rootPath: state.projectRoot, checkpointId });
      await refreshList(state.projectRoot);
    } catch (err) {
      setError(String(err));
    } finally {
      setState(prev => ({ ...prev, busy: { ...prev.busy, restoring: null } }));
    }
  }, [state.projectRoot, refreshList, setError]);

  const diff = useCallback(async (id1: string, id2: string) => {
    setState(prev => ({ ...prev, busy: { ...prev.busy, diffing: true }, diff: null }));
    try {
      const result = await invoke<CheckpointDiff>('titor_diff', {
        rootPath: state.projectRoot,
        id1,
        id2,
      });
      setState(prev => ({ ...prev, diff: result, busy: { ...prev.busy, diffing: false } }));
    } catch (err) {
      setError(String(err));
    }
  }, [state.projectRoot, setError]);

  const verify = useCallback(async (checkpointId: string) => {
    setState(prev => ({ ...prev, busy: { ...prev.busy, verifying: checkpointId } }));
    try {
      const report = await invoke<VerificationReport>('titor_verify', {
        rootPath: state.projectRoot,
        checkpointId,
      });
      setState(prev => ({
        ...prev,
        verifications: { ...prev.verifications, [checkpointId]: report },
        busy: { ...prev.busy, verifying: null },
      }));
    } catch (err) {
      setError(String(err));
    }
  }, [state.projectRoot, setError]);

  const reset = useCallback(() => {
    setState(initialState);
  }, []);

  return { state, initialize, createCheckpoint, restore, diff, verify, reset };
}
