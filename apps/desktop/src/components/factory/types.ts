// Spec: specs/076-factory-desktop-panel/spec.md
// TypeScript types for the Factory Pipeline panel.

/** Pipeline execution phase. */
export type FactoryPhase = 'idle' | 'process' | 'scaffolding' | 'complete' | 'failed';

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

/** A completed pipeline run. */
export interface PipelineRun {
  runId: string;
  adapter: string;
  projectPath: string;
  startedAt: string;
  completedAt?: string;
  duration?: number;
  phase: FactoryPhase;
  totalTokens: number;
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

// ── Process stage constants ──────────────────────────────────────────

export const PROCESS_STAGES: { id: string; name: string }[] = [
  { id: 's0', name: 'Pre-flight' },
  { id: 's1', name: 'Business Requirements' },
  { id: 's2', name: 'Service Requirements' },
  { id: 's3', name: 'Data Model' },
  { id: 's4', name: 'API Specification' },
  { id: 's5', name: 'UI Specification' },
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
  };
}
