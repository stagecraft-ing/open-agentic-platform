// Spec: specs/076-factory-desktop-panel/spec.md
// React context for Factory pipeline state management.

import React, {
  createContext,
  useState,
  useContext,
  useCallback,
  useEffect,
} from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { apiCall } from '@/lib/apiAdapter';
import {
  FactoryPipelineState,
  ArtifactEntry,
  GateAction,
  AuditEntry,
  AgentOutputLine,
  FactoryStepStartedEvent,
  FactoryStepCompletedEvent,
  FactoryStepFailedEvent,
  FactoryGateReachedEvent,
  FactoryScaffoldProgressEvent,
  FactoryAgentOutputEvent,
  createInitialPipelineState,
} from './types';

// ── Constants ────────────────────────────────────────────────────────────────

const MAX_AGENT_OUTPUT_LINES = 500;

// ── Context shape ────────────────────────────────────────────────────────────

interface FactoryPipelineContextType {
  state: FactoryPipelineState;
  agentOutput: AgentOutputLine[];
  startPipeline: (
    projectPath: string,
    adapterName: string,
    businessDocPaths: string[],
    stagecraftProjectId?: string,
  ) => Promise<string>;
  confirmStage: (stageId: string) => Promise<void>;
  rejectStage: (stageId: string, feedback: string) => Promise<void>;
  skipStep: (stepId: string) => Promise<void>;
  cancelPipeline: (reason: string) => Promise<void>;
  selectStep: (stepId: string | null) => void;
  loadPipelineStatus: (runId: string) => Promise<void>;
  loadArtifacts: (stepId: string) => Promise<ArtifactEntry[]>;
  dismissGate: () => void;
}

// ── Context creation ─────────────────────────────────────────────────────────

const FactoryPipelineContext = createContext<
  FactoryPipelineContextType | undefined
>(undefined);

// ── Helper ───────────────────────────────────────────────────────────────────

function nowIso(): string {
  return new Date().toISOString();
}

// ── Provider ─────────────────────────────────────────────────────────────────

export const FactoryPipelineProvider: React.FC<{
  children: React.ReactNode;
}> = ({ children }) => {
  const [state, setState] = useState<FactoryPipelineState>(
    createInitialPipelineState,
  );
  const [agentOutput, setAgentOutput] = useState<AgentOutputLine[]>([]);

  // ── Tauri event listeners ──────────────────────────────────────────────────

  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    async function setupListeners() {
      // factory:step_started
      unlisteners.push(
        await listen<FactoryStepStartedEvent>('factory:step_started', (event) => {
          const { stepId } = event.payload;
          // Clear output when a new step starts
          setAgentOutput([]);
          setState((prev) => ({
            ...prev,
            stages: prev.stages.map((s) =>
              s.id === stepId
                ? { ...s, status: 'in_progress', startedAt: nowIso() }
                : s,
            ),
          }));
        }),
      );

      // factory:step_completed
      unlisteners.push(
        await listen<FactoryStepCompletedEvent>(
          'factory:step_completed',
          (event) => {
            const { stepId, artifacts, tokenSpend } = event.payload;
            setState((prev) => ({
              ...prev,
              stages: prev.stages.map((s) =>
                s.id === stepId
                  ? {
                      ...s,
                      status: 'completed',
                      completedAt: nowIso(),
                      artifacts,
                      tokenSpend: s.tokenSpend + tokenSpend,
                    }
                  : s,
              ),
            }));
          },
        ),
      );

      // factory:step_failed
      unlisteners.push(
        await listen<FactoryStepFailedEvent>('factory:step_failed', (event) => {
          const { stepId } = event.payload;
          setState((prev) => ({
            ...prev,
            stages: prev.stages.map((s) =>
              s.id === stepId ? { ...s, status: 'failed' } : s,
            ),
          }));
        }),
      );

      // factory:gate_reached
      unlisteners.push(
        await listen<FactoryGateReachedEvent>(
          'factory:gate_reached',
          (event) => {
            const { runId, stageId, stageName, gateType, summary, timeoutMs } =
              event.payload;
            const gateAction: GateAction = {
              runId,
              stageId,
              stageName,
              gateType,
              summary,
              timeoutMs,
              openedAt: new Date().toISOString(),
            };
            setState((prev) => ({
              ...prev,
              stages: prev.stages.map((s) =>
                s.id === stageId ? { ...s, status: 'awaiting_gate' } : s,
              ),
              gateAction,
            }));
          },
        ),
      );

      // factory:scaffold_progress
      unlisteners.push(
        await listen<FactoryScaffoldProgressEvent>(
          'factory:scaffold_progress',
          (event) => {
            const {
              category,
              stepId,
              featureName,
              status,
              error,
              retryCount,
            } = event.payload;

            // Clear output when a new scaffold step starts
            if (status === 'started') {
              setAgentOutput([]);
            }

            setState((prev) => {
              if (!prev.scaffolding) return prev;

              const updatedCategories = prev.scaffolding.categories.map(
                (cat) => {
                  if (cat.category !== category) return cat;

                  const existingStep = cat.steps.find((s) => s.id === stepId);

                  let updatedSteps;
                  if (existingStep) {
                    updatedSteps = cat.steps.map((s) =>
                      s.id === stepId
                        ? {
                            ...s,
                            status:
                              status === 'started'
                                ? ('in_progress' as const)
                                : status === 'completed'
                                  ? ('completed' as const)
                                  : ('failed' as const),
                            lastError: error,
                            retryCount: retryCount ?? s.retryCount,
                          }
                        : s,
                    );
                  } else {
                    updatedSteps = [
                      ...cat.steps,
                      {
                        id: stepId,
                        category,
                        featureName,
                        status:
                          status === 'started'
                            ? ('in_progress' as const)
                            : status === 'completed'
                              ? ('completed' as const)
                              : ('failed' as const),
                        retryCount: retryCount ?? 0,
                        maxRetries: 3,
                        lastError: error,
                        tokenSpend: 0,
                      },
                    ];
                  }

                  const completed = updatedSteps.filter(
                    (s) => s.status === 'completed',
                  ).length;
                  const failed = updatedSteps.filter(
                    (s) => s.status === 'failed',
                  ).length;
                  const inProgress = updatedSteps.filter(
                    (s) => s.status === 'in_progress',
                  ).length;

                  return {
                    ...cat,
                    steps: updatedSteps,
                    total: updatedSteps.length,
                    completed,
                    failed,
                    inProgress,
                  };
                },
              );

              return {
                ...prev,
                scaffolding: {
                  ...prev.scaffolding,
                  categories: updatedCategories,
                  activeStepId:
                    status === 'started' ? stepId : prev.scaffolding.activeStepId,
                },
              };
            });
          },
        ),
      );

      // factory:token_update
      unlisteners.push(
        await listen<{ runId: string; stageId: string; promptTokens: number; completionTokens: number }>(
          'factory:token_update',
          (event) => {
            const { stageId, promptTokens, completionTokens } = event.payload;
            const totalTokens = promptTokens + completionTokens;

            setState((prev) => {
              const existingStageEntry = prev.tokenSpend.stages.find(
                (s) => s.stageId === stageId,
              );
              const stageName =
                prev.stages.find((s) => s.id === stageId)?.name ?? stageId;

              const updatedStages = existingStageEntry
                ? prev.tokenSpend.stages.map((s) =>
                    s.stageId === stageId
                      ? {
                          ...s,
                          promptTokens: s.promptTokens + promptTokens,
                          completionTokens:
                            s.completionTokens + completionTokens,
                          totalTokens: s.totalTokens + totalTokens,
                        }
                      : s,
                  )
                : [
                    ...prev.tokenSpend.stages,
                    {
                      stageId,
                      stageName,
                      promptTokens,
                      completionTokens,
                      totalTokens,
                    },
                  ];

              const newTotal = updatedStages.reduce(
                (sum, s) => sum + s.totalTokens,
                0,
              );

              return {
                ...prev,
                tokenSpend: {
                  ...prev.tokenSpend,
                  stages: updatedStages,
                  totalTokens: newTotal,
                },
              };
            });
          },
        ),
      );

      // factory:agent_output — stream lines into agentOutput state
      unlisteners.push(
        await listen<FactoryAgentOutputEvent>('factory:agent_output', (event) => {
          const { stepId, line } = event.payload;
          const entry: AgentOutputLine = {
            stepId,
            line,
            timestamp: nowIso(),
          };
          setAgentOutput((prev) => {
            const next = [...prev, entry];
            // Keep last MAX_AGENT_OUTPUT_LINES lines to avoid memory growth
            return next.length > MAX_AGENT_OUTPUT_LINES
              ? next.slice(next.length - MAX_AGENT_OUTPUT_LINES)
              : next;
          });
        }),
      );

      // factory:workflow_started — FR-009: initialize DAG display
      unlisteners.push(
        await listen<{ runId: string }>('factory:workflow_started', (event) => {
          const { runId } = event.payload;
          setAgentOutput([]);
          setState((prev) => ({
            ...createInitialPipelineState(),
            runId,
            phase: 'process',
            artifacts: prev.artifacts,
          }));
        }),
      );
    }

    setupListeners().catch((err) => {
      console.error('[FactoryPipelineContext] Failed to set up event listeners:', err);
    });

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, []);

  // ── Actions ────────────────────────────────────────────────────────────────

  const startPipeline = useCallback(
    async (
      projectPath: string,
      adapterName: string,
      businessDocPaths: string[],
      stagecraftProjectId?: string,
    ): Promise<string> => {
      const resp = await apiCall<{ run_id: string }>('start_factory_pipeline', {
        projectPath,
        adapterName,
        businessDocPaths,
        stagecraftProjectId,
      });
      const runId = resp.run_id;

      setAgentOutput([]);
      setState((_prev) => {
        const auditEntry: AuditEntry = {
          timestamp: nowIso(),
          action: 'pipeline_started',
          details: `adapter=${adapterName} project=${projectPath}`,
        };
        return {
          ...createInitialPipelineState(),
          runId,
          phase: 'process',
          auditTrail: [auditEntry],
        };
      });

      return runId;
    },
    [],
  );

  const confirmStage = useCallback(
    async (stageId: string): Promise<void> => {
      const runId = state.runId;
      if (!runId) return;

      await apiCall<void>('confirm_factory_stage', { runId, stageId });

      setState((prev) => {
        const auditEntry: AuditEntry = {
          timestamp: nowIso(),
          action: 'stage_confirmed',
          stageId,
        };
        return {
          ...prev,
          gateAction: null,
          auditTrail: [...prev.auditTrail, auditEntry],
        };
      });
    },
    [state.runId],
  );

  const rejectStage = useCallback(
    async (stageId: string, feedback: string): Promise<void> => {
      const runId = state.runId;
      if (!runId) return;

      await apiCall<void>('reject_factory_stage', { runId, stageId, feedback });

      setState((prev) => {
        const auditEntry: AuditEntry = {
          timestamp: nowIso(),
          action: 'stage_rejected',
          stageId,
          feedback,
        };
        return {
          ...prev,
          gateAction: null,
          auditTrail: [...prev.auditTrail, auditEntry],
        };
      });
    },
    [state.runId],
  );

  const skipStep = useCallback(
    async (stepId: string): Promise<void> => {
      const runId = state.runId;
      if (!runId) return;

      await apiCall<void>('skip_factory_step', { runId, stepId });
    },
    [state.runId],
  );

  const cancelPipeline = useCallback(
    async (reason: string): Promise<void> => {
      const runId = state.runId;
      if (!runId) return;

      await apiCall<void>('cancel_factory_pipeline', { runId, reason });
      setState((prev) => ({ ...prev, phase: 'failed' }));
    },
    [state.runId],
  );

  const selectStep = useCallback((stepId: string | null): void => {
    setState((prev) => ({ ...prev, selectedStepId: stepId }));
  }, []);

  const loadPipelineStatus = useCallback(
    async (runId: string): Promise<void> => {
      // Tauri command returns snake_case fields; map to our camelCase state.
      const resp = await apiCall<any>('get_factory_pipeline_status', { runId });
      setState((prev) => ({
        ...prev,
        runId: resp.run_id ?? runId,
        phase: resp.phase ?? prev.phase,
        stages: (resp.stages ?? []).map((s: any) => ({
          id: s.id,
          name: s.name,
          index: prev.stages.find((ps) => ps.id === s.id)?.index ?? 0,
          status: s.status,
          startedAt: s.started_at,
          completedAt: s.completed_at,
          tokenSpend: s.token_spend ?? 0,
          artifacts: s.artifacts ?? [],
        })),
        tokenSpend: {
          ...prev.tokenSpend,
          totalTokens: resp.total_tokens ?? 0,
        },
        auditTrail: (resp.audit_trail ?? []).map((a: any) => ({
          timestamp: a.timestamp,
          action: a.action,
          stageId: a.stage_id,
          details: a.details,
          feedback: a.feedback,
        })),
        // Preserve locally-cached artifacts already loaded this session.
        artifacts: prev.artifacts,
      }));
    },
    [],
  );

  const loadArtifacts = useCallback(
    async (stepId: string): Promise<ArtifactEntry[]> => {
      const runId = state.runId;
      if (!runId) return [];

      const entries = await apiCall<ArtifactEntry[]>('get_factory_artifacts', {
        runId,
        stepId,
      });

      setState((prev) => {
        const updated = new Map(prev.artifacts);
        updated.set(stepId, entries);
        return { ...prev, artifacts: updated };
      });

      return entries;
    },
    [state.runId],
  );

  const dismissGate = useCallback((): void => {
    setState((prev) => ({ ...prev, gateAction: null }));
  }, []);

  // ── Context value ──────────────────────────────────────────────────────────

  const value: FactoryPipelineContextType = {
    state,
    agentOutput,
    startPipeline,
    confirmStage,
    rejectStage,
    skipStep,
    cancelPipeline,
    selectStep,
    loadPipelineStatus,
    loadArtifacts,
    dismissGate,
  };

  return (
    <FactoryPipelineContext.Provider value={value}>
      {children}
    </FactoryPipelineContext.Provider>
  );
};

// ── Custom hook ───────────────────────────────────────────────────────────────

export const useFactoryPipeline = (): FactoryPipelineContextType => {
  const context = useContext(FactoryPipelineContext);
  if (!context) {
    throw new Error(
      'useFactoryPipeline must be used within an FactoryPipelineProvider',
    );
  }
  return context;
};
