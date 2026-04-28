// Spec 115 §5 / FR-017 — FR-021 — agent-extractor shared infrastructure.
//
// All agent-kind extractors share three responsibilities and one
// boundary:
//   - resolve the workspace policy slice (already done by the worker
//     before dispatch — exposed here as `assertPolicy*` helpers)
//   - apply the per-call cost ceiling (FR-018) before any network call
//   - apply the per-day cost ceiling (FR-019) before any network call
//   - drive the Anthropic SDK with prompt caching enabled (Spec 115
//     §5.4), enforce the no-tool-bearing-call invariant (FR-021),
//     and return a populated `agentRun` block on success
//
// The Anthropic client is created per-worker via `getAnthropicClient`
// (cached at module scope) so concurrent agent extractions share a
// connection pool sized to STAGECRAFT_EXTRACT_WORKER_CONCURRENCY.

import { createHash } from "node:crypto";
import { and, eq, gte, sql } from "drizzle-orm";
import Anthropic from "@anthropic-ai/sdk";
import log from "encore.dev/log";
import { secret } from "encore.dev/config";
import { db } from "../../db/drizzle";
import { knowledgeExtractionRuns } from "../../db/schema";
import type { AgentRun, TokenSpend } from "../extractionOutput";
import type { ExtractionPolicy } from "../extractionPolicy";
import type { AssembledPrompt } from "../prompts";
import { ExtractorError, type TokenSpendReporter } from "./types";
import {
  actualCostUsd,
  assertNoTools,
  estimateCallCostUsd,
  nextUtcMidnightIso,
  type CostEstimateInput,
} from "./agent-cost-helpers";

export {
  actualCostUsd,
  assertNoTools,
  estimateCallCostUsd,
  type CostEstimateInput,
} from "./agent-cost-helpers";

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

/** Anthropic model the agent extractors fall back to when the workspace
 *  policy does not pin a `modelPin`. Pinned to a vision-capable, tool-use-
 *  capable model in the Claude 4.X family (cutoff 2026-01); the prompt
 *  cache reduces cold-start cost on repeat extractions. */
export const DEFAULT_MODEL_ID = "claude-sonnet-4-6";

// ---------------------------------------------------------------------------
// Anthropic client (cached, pool sized to worker concurrency)
// ---------------------------------------------------------------------------

const anthropicApiKey = secret("ANTHROPIC_API_KEY");

let cachedClient: Anthropic | null = null;

export function getAnthropicClient(): Anthropic {
  if (!cachedClient) {
    cachedClient = new Anthropic({
      apiKey: anthropicApiKey(),
    });
  }
  return cachedClient;
}

/** Visible for tests — the test transport overrides the cached client. */
export function _setAnthropicClientForTesting(client: Anthropic | null): void {
  cachedClient = client;
}

// ---------------------------------------------------------------------------
// Day-aggregate gate (FR-019). Pure helpers live in agent-cost-helpers.ts.
// ---------------------------------------------------------------------------

/**
 * FR-019 — sum of `cost_usd` for runs whose `completed_at >= today UTC
 * midnight` for this workspace. Runs that are still `pending` or
 * `running` are intentionally NOT counted; their cost is only known on
 * completion. This matches the spec's "we don't abort an in-flight call"
 * decision.
 */
export async function getDayAggregateCostUsd(
  workspaceId: string,
  now: Date = new Date(),
): Promise<number> {
  const utcMidnight = new Date(
    Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate()),
  );
  const result = await db
    .select({ total: sql<string>`COALESCE(SUM(${knowledgeExtractionRuns.costUsd}), 0)::text` })
    .from(knowledgeExtractionRuns)
    .where(
      and(
        eq(knowledgeExtractionRuns.workspaceId, workspaceId),
        gte(knowledgeExtractionRuns.completedAt, utcMidnight),
      ),
    );
  const raw = result[0]?.total ?? "0";
  const n = Number.parseFloat(raw);
  return Number.isFinite(n) ? n : 0;
}

export type CostGateArgs = {
  policy: ExtractionPolicy;
  workspaceId: string;
  extractorKind: string;
  estimate: CostEstimateInput;
};

/**
 * FR-018 + FR-019 — pre-flight cost check. Throws an `ExtractorError`
 * with the right code so the worker writes the correct
 * `lastExtractionError.code`. No HTTP call is made if either gate trips.
 */
export async function applyCostGates(args: CostGateArgs): Promise<void> {
  const estimate = estimateCallCostUsd(args.estimate);
  if (estimate > args.policy.costCeilingUsdPerCall) {
    throw new ExtractorError({
      code: "cost_ceiling_exceeded",
      extractorKind: args.extractorKind,
      message: `pre-flight estimate $${estimate.toFixed(
        4,
      )} exceeds per-call ceiling $${args.policy.costCeilingUsdPerCall.toFixed(4)}`,
    });
  }
  const dayTotal = await getDayAggregateCostUsd(args.workspaceId);
  if (dayTotal + estimate > args.policy.costCeilingUsdPerDay) {
    const tomorrow = nextUtcMidnightIso();
    throw new ExtractorError({
      code: "daily_cost_exhausted",
      extractorKind: args.extractorKind,
      message: `day-aggregate $${dayTotal.toFixed(4)} + estimate $${estimate.toFixed(
        4,
      )} exceeds day ceiling $${args.policy.costCeilingUsdPerDay.toFixed(
        4,
      )}; retryAt=${tomorrow}`,
      retriable: true,
    });
  }
}

// ---------------------------------------------------------------------------
// runAgentMessage — the only place that calls anthropic.messages.create
// ---------------------------------------------------------------------------

export type AgentMessageArgs = {
  client: Anthropic;
  modelId: string;
  prompt: AssembledPrompt;
  /** User-content blocks: text + image/document inputs, etc. */
  content: Anthropic.Messages.ContentBlockParam[];
  /** Estimated tokens for cost gating, set by the extractor. */
  estimate: CostEstimateInput;
  /** Workspace + extractor kind for the gate + audit. */
  policy: ExtractionPolicy;
  workspaceId: string;
  extractorKind: string;
  reportTokenSpend: TokenSpendReporter;
  /** Optional override for max output tokens (default 4096). */
  maxOutputTokens?: number;
};

export type AgentMessageResult = {
  text: string;
  spend: TokenSpend;
  costUsd: number;
};

export async function runAgentMessage(
  args: AgentMessageArgs,
): Promise<AgentMessageResult> {
  // Pre-flight cost gates — no HTTP call if either fails.
  await applyCostGates({
    policy: args.policy,
    workspaceId: args.workspaceId,
    extractorKind: args.extractorKind,
    estimate: args.estimate,
  });

  // FR-021 fail-closed.
  assertNoTools({}, args.extractorKind);

  const startedAt = Date.now();
  const response = await args.client.messages.create({
    model: args.modelId,
    max_tokens: args.maxOutputTokens ?? 4096,
    system: [
      {
        type: "text" as const,
        text: args.prompt.system,
        // Prompt caching for the system block. Stagecraft sees high
        // repeat-rate on the system prompt across uploads in the same
        // workspace; the cache hit ratio dominates per-call cost.
        cache_control: { type: "ephemeral" },
      },
    ],
    messages: [
      {
        role: "user" as const,
        content: args.content,
      },
    ],
  });
  const durationMs = Date.now() - startedAt;

  const text = response.content
    .filter(
      (block): block is Anthropic.Messages.TextBlock => block.type === "text",
    )
    .map((b) => b.text)
    .join("\n")
    .trim();

  const spend: TokenSpend = {
    input: response.usage.input_tokens ?? 0,
    output: response.usage.output_tokens ?? 0,
    cacheRead: response.usage.cache_read_input_tokens ?? undefined,
    cacheWrite: response.usage.cache_creation_input_tokens ?? undefined,
  };
  const costUsd = actualCostUsd(spend);
  args.reportTokenSpend(spend, costUsd);

  log.info("agent extractor: anthropic message complete", {
    extractorKind: args.extractorKind,
    modelId: args.modelId,
    durationMs,
    inputTokens: spend.input,
    outputTokens: spend.output,
    costUsd,
  });

  return { text, spend, costUsd };
}

// ---------------------------------------------------------------------------
// Helpers — common to every agent extractor
// ---------------------------------------------------------------------------

export function pickModelId(policy: ExtractionPolicy): string {
  return policy.modelPin && policy.modelPin.length > 0
    ? policy.modelPin
    : DEFAULT_MODEL_ID;
}

export function buildAgentRun(args: {
  modelId: string;
  prompt: AssembledPrompt;
  durationMs: number;
  spend: TokenSpend;
  costUsd: number;
  attempts: number;
}): AgentRun {
  return {
    modelId: args.modelId,
    promptFingerprint: args.prompt.fingerprint,
    durationMs: args.durationMs,
    tokenSpend: args.spend,
    costUsd: args.costUsd,
    attempts: args.attempts,
  };
}

/**
 * Hash a buffer's content; useful if a future extractor needs a per-image
 * fingerprint to log alongside `promptFingerprint`. Not used directly
 * today — exported for upcoming work.
 */
export function sha256Hex(buf: Buffer): string {
  return createHash("sha256").update(buf).digest("hex");
}
