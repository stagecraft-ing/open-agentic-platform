export const DEFAULT_COMPACTION_THRESHOLD = 0.75;
export const DEFAULT_PRESERVE_RECENT_TURNS = 4;
export const MIN_COMPACTION_THRESHOLD = 0.5;
export const MAX_COMPACTION_THRESHOLD = 0.95;

export interface ContextCompactionConfigInput {
  compaction?: {
    threshold?: number | string;
    preserve_recent_turns?: number;
  };
}

export interface ContextCompactionConfig {
  threshold: number;
  preserveRecentTurns: number;
}

export type CompactionMessageRole = "system" | "user" | "assistant" | "tool";

export interface CompactionMessageUsage {
  input_tokens?: number;
  output_tokens?: number;
}

export interface CompactionMessage {
  id: string;
  role: CompactionMessageRole;
  content: string;
  timestamp?: string;
  pinned?: boolean;
  usage?: CompactionMessageUsage;
  tool_name?: string;
  tool_call_id?: string;
  meta?: Record<string, unknown>;
}

export interface CompactionHistory {
  messages: CompactionMessage[];
}

export interface TokenBudgetUsageTotals {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
}

export interface CompactionTriggerDecision {
  shouldCompact: boolean;
  reason: string;
  usageRatio: number;
  thresholdRatio: number;
  usedTokens: number;
  contextWindowTokens: number;
}

export class TokenBudgetMonitor {
  private promptTokens = 0;
  private completionTokens = 0;
  private readonly threshold: number;

  constructor(config: ContextCompactionConfig) {
    this.threshold = config.threshold;
  }

  reportUsage(promptTokens: number, completionTokens: number): void {
    this.promptTokens += sanitizeTokenDelta(promptTokens);
    this.completionTokens += sanitizeTokenDelta(completionTokens);
  }

  getTotals(): TokenBudgetUsageTotals {
    const totalTokens = this.promptTokens + this.completionTokens;
    return {
      promptTokens: this.promptTokens,
      completionTokens: this.completionTokens,
      totalTokens,
    };
  }

  shouldCompact(contextWindowTokens: number): CompactionTriggerDecision {
    const safeContextWindow = sanitizeContextWindow(contextWindowTokens);
    const totals = this.getTotals();
    const usageRatio =
      safeContextWindow === 0 ? 0 : totals.totalTokens / safeContextWindow;
    const shouldCompact = usageRatio >= this.threshold;
    const comparison = shouldCompact ? ">=" : "<";
    const reason = `usage ratio ${usageRatio.toFixed(4)} ${comparison} threshold ${this.threshold.toFixed(4)} (${totals.totalTokens}/${safeContextWindow} tokens)`;

    return {
      shouldCompact,
      reason,
      usageRatio,
      thresholdRatio: this.threshold,
      usedTokens: totals.totalTokens,
      contextWindowTokens: safeContextWindow,
    };
  }
}

export function readCompactionThresholdFromEnv(
  env = safeProcessEnv(),
): number | undefined {
  const raw = env["OAP_COMPACTION_THRESHOLD"];
  if (raw === undefined || raw === "") return undefined;
  const parsed = Number.parseFloat(raw);
  if (!Number.isFinite(parsed)) return undefined;
  return isValidCompactionThreshold(parsed) ? parsed : undefined;
}

export function resolveContextCompactionConfig(
  input?: ContextCompactionConfigInput,
  env = safeProcessEnv(),
): ContextCompactionConfig {
  const envThreshold = readCompactionThresholdFromEnv(env);
  const configThreshold = parseMaybeNumber(input?.compaction?.threshold);

  const threshold = chooseFirstValidThreshold([
    envThreshold,
    configThreshold,
    DEFAULT_COMPACTION_THRESHOLD,
  ]);

  const preserveRecentTurns = sanitizePreserveRecentTurns(
    input?.compaction?.preserve_recent_turns,
  );

  return { threshold, preserveRecentTurns };
}

export function stableSerializeHistory(history: CompactionHistory): string {
  const normalized = history.messages.map((message) => ({
    id: message.id,
    role: message.role,
    content: message.content,
    timestamp: message.timestamp ?? null,
    pinned: message.pinned ?? false,
    usage: message.usage
      ? {
          input_tokens: message.usage.input_tokens ?? null,
          output_tokens: message.usage.output_tokens ?? null,
        }
      : null,
    tool_name: message.tool_name ?? null,
    tool_call_id: message.tool_call_id ?? null,
    meta: stableSortRecord(message.meta),
  }));

  return JSON.stringify({ messages: normalized });
}

function safeProcessEnv(): Record<string, string | undefined> {
  if (typeof process === "undefined" || !process.env) return {};
  return process.env;
}

function parseMaybeNumber(value: number | string | undefined): number | undefined {
  if (value === undefined) return undefined;
  if (typeof value === "number") return value;
  if (value.trim() === "") return undefined;
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function chooseFirstValidThreshold(
  values: Array<number | undefined>,
): number {
  for (const value of values) {
    if (value !== undefined && isValidCompactionThreshold(value)) {
      return value;
    }
  }
  return DEFAULT_COMPACTION_THRESHOLD;
}

function isValidCompactionThreshold(value: number): boolean {
  return value >= MIN_COMPACTION_THRESHOLD && value <= MAX_COMPACTION_THRESHOLD;
}

function sanitizePreserveRecentTurns(value: number | undefined): number {
  if (value === undefined || !Number.isFinite(value)) {
    return DEFAULT_PRESERVE_RECENT_TURNS;
  }
  const rounded = Math.floor(value);
  return rounded >= 1 ? rounded : DEFAULT_PRESERVE_RECENT_TURNS;
}

function stableSortRecord(
  value: Record<string, unknown> | undefined,
): Record<string, unknown> | null {
  if (!value) return null;
  const keys = Object.keys(value).sort();
  const output: Record<string, unknown> = {};
  for (const key of keys) {
    output[key] = value[key];
  }
  return output;
}

function sanitizeTokenDelta(value: number): number {
  if (!Number.isFinite(value) || value <= 0) return 0;
  return Math.floor(value);
}

function sanitizeContextWindow(value: number): number {
  if (!Number.isFinite(value) || value <= 0) return 0;
  return Math.floor(value);
}
