// Spec: specs/076-factory-desktop-panel/spec.md
// TypeScript types for the Factory Pipeline panel.

/** Pipeline execution phase.
 *
 * `paused` means the run was hydrated from disk and is not actively
 * running. It enables Resume in place of Cancel and suppresses the live
 * output panel. The backend uses it for runs loaded from `state.json` /
 * manifest-only directories that aren't in `FACTORY_RUNS`.
 */
export type FactoryPhase =
  | 'idle'
  | 'process'
  | 'scaffolding'
  | 'complete'
  | 'failed'
  | 'paused';

/** Visual status for a stage or step node. */
export type StageStatus = 'pending' | 'in_progress' | 'completed' | 'failed' | 'awaiting_gate' | 'skipped';

/** Process stage definition (s0–s5). */
export interface ProcessStage {
  id: string;
  name: string;
  index: number;
  status: StageStatus;
  startedAt?: string;
  completedAt?: string;
  tokenSpend: number;
  artifacts: string[];
}

/** Scaffold category grouping. */
export type ScaffoldCategory = 'data' | 'api' | 'ui' | 'configure' | 'trim' | 'validate';

/** Individual scaffold step. */
export interface ScaffoldStep {
  id: string;
  category: ScaffoldCategory;
  featureName: string;
  status: StageStatus;
  retryCount: number;
  maxRetries: number;
  lastError?: string;
  tokenSpend: number;
}

/** Category-level progress. */
export interface ScaffoldCategoryProgress {
  category: ScaffoldCategory;
  total: number;
  completed: number;
  failed: number;
  inProgress: number;
  steps: ScaffoldStep[];
}

/** Full scaffolding state. */
export interface ScaffoldingState {
  categories: ScaffoldCategoryProgress[];
  activeStepId: string | null;
}

/** Token usage per stage. */
export interface StageTokenSpend {
  stageId: string;
  stageName: string;
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
}

/** Cumulative token tracking. */
export interface TokenSpend {
  stages: StageTokenSpend[];
  totalTokens: number;
  budgetLimit: number | null;
}

/** Gate action — shown when orchestrator emits gate_reached. */
export interface GateAction {
  runId: string;
  stageId: string;
  stageName: string;
  gateType: 'checkpoint' | 'approval';
  summary?: GateSummary;
  /** Milliseconds until this gate times out (approval gates only). */
  timeoutMs?: number;
  /** ISO timestamp when the gate was opened, used with timeoutMs for countdown. */
  openedAt?: string;
}

/** Summary statistics shown in gate dialogs. */
export interface GateSummary {
  entityCount?: number;
  operationCount?: number;
  pageCount?: number;
  ruleCount?: number;
  description?: string;
}

/** Artifact entry for inspector. */
export interface ArtifactEntry {
  name: string;
  path: string;
  size: number;
  mimeType: 'markdown' | 'json' | 'yaml' | 'text' | 'unknown';
}

/** A pipeline run summary surfaced in the history list. */
export interface PipelineRun {
  runId: string;
  adapter: string;
  projectPath: string;
  startedAt: string;
  completedAt?: string;
  duration?: number;
  phase: FactoryPhase;
  totalTokens: number;
  /** Count of process stages with at least one output artifact on disk. */
  stagesCompleted: number;
  /** Total declared process stages — currently always 6. */
  stagesTotal: number;
  /** Display name of the highest-index completed process stage. */
  lastCompletedStage: string | null;
}

/** Audit trail entry. */
export interface AuditEntry {
  timestamp: string;
  action: 'stage_confirmed' | 'stage_rejected' | 'build_spec_frozen' | 'step_retried' | 'step_skipped' | 'pipeline_started' | 'pipeline_completed' | 'pipeline_failed';
  stageId?: string;
  details?: string;
  feedback?: string;
}

/** Top-level pipeline state managed by FactoryPipelineContext. */
export interface FactoryPipelineState {
  runId: string | null;
  phase: FactoryPhase;
  stages: ProcessStage[];
  scaffolding: ScaffoldingState | null;
  tokenSpend: TokenSpend;
  selectedStepId: string | null;
  artifacts: Map<string, ArtifactEntry[]>;
  gateAction: GateAction | null;
  auditTrail: AuditEntry[];
  /** Project filesystem path tied to this run. Set by `loadPipelineStatus`
   * for disk-loaded runs and used by Resume / artifact lookups so they work
   * after the run has fallen out of `FACTORY_RUNS`. */
  projectPath: string | null;
  /** Adapter recorded in the run's `state.json`. Empty for legacy runs that
   * predate the early state-write — Resume falls back to the bundle adapter
   * when present. */
  adapter: string | null;
}

// ── Tauri event payloads ─────────────────────────────────────────────

export interface FactoryStepStartedEvent {
  runId: string;
  stepId: string;
  stepName: string;
}

export interface FactoryStepCompletedEvent {
  runId: string;
  stepId: string;
  artifacts: string[];
  tokenSpend: number;
}

export interface FactoryStepFailedEvent {
  runId: string;
  stepId: string;
  error: string;
  retryCount: number;
}

export interface FactoryGateReachedEvent {
  runId: string;
  stageId: string;
  stageName: string;
  gateType: 'checkpoint' | 'approval';
  summary?: GateSummary;
  /** Milliseconds until this gate times out (approval gates only). */
  timeoutMs?: number;
}

export interface FactoryScaffoldProgressEvent {
  runId: string;
  category: ScaffoldCategory;
  stepId: string;
  featureName: string;
  status: 'started' | 'completed' | 'failed';
  error?: string;
  retryCount?: number;
}

export interface FactoryTokenUpdateEvent {
  runId: string;
  stageId: string;
  promptTokens: number;
  completionTokens: number;
}

export interface FactoryAgentOutputEvent {
  runId: string;
  stepId: string;
  line: string;
}

/** A single line of streamed agent output. */
export interface AgentOutputLine {
  stepId: string;
  line: string;
  timestamp: string;
}

// ── Process stage constants ──────────────────────────────────────────

// Stage ids must match the backend constants in
// apps/desktop/src-tauri/src/commands/factory.rs (PROCESS_STAGES). Events
// `factory:step_started` / `step_completed` / `gate_reached` arrive with these
// full ids; if they drift the DAG never advances out of `pending`.
export const PROCESS_STAGES: { id: string; name: string }[] = [
  { id: 's0-preflight', name: 'Pre-flight' },
  { id: 's1-business-requirements', name: 'Business Requirements' },
  { id: 's2-service-requirements', name: 'Service Requirements' },
  { id: 's3-data-model', name: 'Data Model' },
  { id: 's4-api-specification', name: 'API Specification' },
  { id: 's5-ui-specification', name: 'UI Specification' },
];

export const SCAFFOLD_CATEGORY_LABELS: Record<ScaffoldCategory, string> = {
  data: 'Data Entities',
  api: 'API Operations',
  ui: 'UI Pages',
  configure: 'Configure',
  trim: 'Trim',
  validate: 'Final Validation',
};

/** Create initial empty pipeline state. */
export function createInitialPipelineState(): FactoryPipelineState {
  return {
    runId: null,
    phase: 'idle',
    stages: PROCESS_STAGES.map((s, i) => ({
      id: s.id,
      name: s.name,
      index: i,
      status: 'pending',
      tokenSpend: 0,
      artifacts: [],
    })),
    scaffolding: null,
    tokenSpend: { stages: [], totalTokens: 0, budgetLimit: null },
    selectedStepId: null,
    artifacts: new Map(),
    gateAction: null,
    auditTrail: [],
    projectPath: null,
    adapter: null,
  };
}
