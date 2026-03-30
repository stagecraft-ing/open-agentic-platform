import { describe, expect, it } from "vitest";
import {
  DEFAULT_COMPACTION_THRESHOLD,
  DEFAULT_PRESERVE_RECENT_TURNS,
  MAX_COMPACTION_THRESHOLD,
  MIN_COMPACTION_THRESHOLD,
  ProgrammaticCompactor,
  TokenBudgetMonitor,
  readCompactionThresholdFromEnv,
  resolveContextCompactionConfig,
  stableSerializeHistory,
  type CompactionHistory,
  type GitSnapshot,
} from "./contextCompaction";

describe("context compaction config", () => {
  it("uses defaults when env and config are unset", () => {
    const config = resolveContextCompactionConfig(undefined, {});
    expect(config).toEqual({
      threshold: DEFAULT_COMPACTION_THRESHOLD,
      preserveRecentTurns: DEFAULT_PRESERVE_RECENT_TURNS,
    });
  });

  it("prefers environment threshold over config threshold", () => {
    const config = resolveContextCompactionConfig(
      { compaction: { threshold: 0.6 } },
      { OAP_COMPACTION_THRESHOLD: "0.9" },
    );
    expect(config.threshold).toBe(0.9);
  });

  it("uses config threshold when environment threshold is missing", () => {
    const config = resolveContextCompactionConfig(
      { compaction: { threshold: 0.65, preserve_recent_turns: 8 } },
      {},
    );
    expect(config.threshold).toBe(0.65);
    expect(config.preserveRecentTurns).toBe(8);
  });

  it("falls back to default when env threshold is invalid", () => {
    const config = resolveContextCompactionConfig(
      undefined,
      { OAP_COMPACTION_THRESHOLD: "not-a-number" },
    );
    expect(config.threshold).toBe(DEFAULT_COMPACTION_THRESHOLD);
  });

  it("accepts threshold boundaries", () => {
    const min = resolveContextCompactionConfig(
      { compaction: { threshold: MIN_COMPACTION_THRESHOLD } },
      {},
    );
    const max = resolveContextCompactionConfig(
      { compaction: { threshold: MAX_COMPACTION_THRESHOLD } },
      {},
    );
    expect(min.threshold).toBe(MIN_COMPACTION_THRESHOLD);
    expect(max.threshold).toBe(MAX_COMPACTION_THRESHOLD);
  });

  it("rejects threshold values outside accepted range", () => {
    const tooLow = resolveContextCompactionConfig(
      { compaction: { threshold: MIN_COMPACTION_THRESHOLD - 0.01 } },
      {},
    );
    const tooHigh = resolveContextCompactionConfig(
      { compaction: { threshold: MAX_COMPACTION_THRESHOLD + 0.01 } },
      {},
    );
    expect(tooLow.threshold).toBe(DEFAULT_COMPACTION_THRESHOLD);
    expect(tooHigh.threshold).toBe(DEFAULT_COMPACTION_THRESHOLD);
  });

  it("normalizes preserve_recent_turns when invalid", () => {
    const config = resolveContextCompactionConfig(
      { compaction: { preserve_recent_turns: 0 } },
      {},
    );
    expect(config.preserveRecentTurns).toBe(DEFAULT_PRESERVE_RECENT_TURNS);
  });
});

describe("readCompactionThresholdFromEnv", () => {
  it("returns undefined for missing or invalid values", () => {
    expect(readCompactionThresholdFromEnv({})).toBeUndefined();
    expect(
      readCompactionThresholdFromEnv({ OAP_COMPACTION_THRESHOLD: "" }),
    ).toBeUndefined();
    expect(
      readCompactionThresholdFromEnv({ OAP_COMPACTION_THRESHOLD: "abc" }),
    ).toBeUndefined();
    expect(
      readCompactionThresholdFromEnv({ OAP_COMPACTION_THRESHOLD: "0.3" }),
    ).toBeUndefined();
  });

  it("returns parsed threshold when value is valid", () => {
    expect(
      readCompactionThresholdFromEnv({ OAP_COMPACTION_THRESHOLD: "0.95" }),
    ).toBe(0.95);
  });
});

describe("stableSerializeHistory", () => {
  it("serializes equivalent history deterministically", () => {
    const historyA: CompactionHistory = {
      messages: [
        {
          id: "m-1",
          role: "user",
          content: "Implement context compaction",
          meta: { z: "last", a: "first" },
        },
      ],
    };
    const historyB: CompactionHistory = {
      messages: [
        {
          id: "m-1",
          role: "user",
          content: "Implement context compaction",
          meta: { a: "first", z: "last" },
        },
      ],
    };

    expect(stableSerializeHistory(historyA)).toBe(stableSerializeHistory(historyB));
  });
});

describe("TokenBudgetMonitor", () => {
  it("does not trigger compaction below threshold", () => {
    const config = resolveContextCompactionConfig(undefined, {});
    const monitor = new TokenBudgetMonitor(config);
    monitor.reportUsage(200, 100);
    const decision = monitor.shouldCompact(1000);

    expect(decision.shouldCompact).toBe(false);
    expect(decision.reason).toContain("< threshold");
    expect(decision.usedTokens).toBe(300);
  });

  it("triggers compaction at threshold boundary", () => {
    const config = resolveContextCompactionConfig(undefined, {});
    const monitor = new TokenBudgetMonitor(config);
    monitor.reportUsage(500, 250);
    const decision = monitor.shouldCompact(1000);

    expect(decision.shouldCompact).toBe(true);
    expect(decision.reason).toContain(">= threshold");
    expect(decision.usageRatio).toBe(0.75);
  });

  it("triggers compaction above threshold", () => {
    const config = resolveContextCompactionConfig(undefined, {});
    const monitor = new TokenBudgetMonitor(config);
    monitor.reportUsage(600, 200);
    const decision = monitor.shouldCompact(1000);

    expect(decision.shouldCompact).toBe(true);
    expect(decision.reason).toContain("800/1000");
  });

  it("respects 0.5 threshold override for earlier compaction", () => {
    const config = resolveContextCompactionConfig(undefined, {
      OAP_COMPACTION_THRESHOLD: "0.5",
    });
    const monitor = new TokenBudgetMonitor(config);
    monitor.reportUsage(300, 220);
    const decision = monitor.shouldCompact(1000);

    expect(config.threshold).toBe(0.5);
    expect(decision.shouldCompact).toBe(true);
  });

  it("respects 0.95 threshold override for later compaction", () => {
    const config = resolveContextCompactionConfig(undefined, {
      OAP_COMPACTION_THRESHOLD: "0.95",
    });
    const monitor = new TokenBudgetMonitor(config);
    monitor.reportUsage(500, 400);
    const decision = monitor.shouldCompact(1000);

    expect(config.threshold).toBe(0.95);
    expect(decision.shouldCompact).toBe(false);
    monitor.reportUsage(40, 20);
    expect(monitor.shouldCompact(1000).shouldCompact).toBe(true);
  });
});

describe("ProgrammaticCompactor", () => {
  const gitSnapshot: GitSnapshot = {
    branch: "main",
    stagedChanges: 1,
    unstagedChanges: 2,
    lastCommitHash: "abc1234",
    lastCommitMessage: "feat: baseline compaction",
    diffStats: {
      insertions: 42,
      deletions: 7,
      filesChanged: 3,
    },
  };

  it("builds deterministic session_context xml with required sections", () => {
    const config = resolveContextCompactionConfig(
      { compaction: { preserve_recent_turns: 2 } },
      {},
    );
    const compactor = new ProgrammaticCompactor(config);
    const history: CompactionHistory = {
      messages: [
        {
          id: "m-1",
          role: "system",
          content: "You are a coding assistant.",
          usage: { input_tokens: 100, output_tokens: 0 },
        },
        {
          id: "m-2",
          role: "user",
          content: "Implement context compaction for session resumption.",
          usage: { input_tokens: 20, output_tokens: 5 },
        },
        {
          id: "m-3",
          role: "assistant",
          content:
            "- [x] Added TokenBudgetMonitor\n- [ ] Build ProgrammaticCompactor\nDecision: Use deterministic XML output.",
          usage: { input_tokens: 30, output_tokens: 40 },
        },
        {
          id: "m-4",
          role: "assistant",
          content:
            "Created apps/desktop/src/lib/contextCompaction.ts and modified apps/desktop/src/lib/contextCompaction.test.ts",
        },
        {
          id: "m-5",
          role: "assistant",
          content: "Next step: implement interruption detection?",
        },
      ],
    };

    const outputA = compactor.compact(history, gitSnapshot);
    const outputB = compactor.compact(history, gitSnapshot);

    expect(outputA.sessionContextBlock).toBe(outputB.sessionContextBlock);
    expect(outputA.sessionContextBlock).toContain("<session_context");
    expect(outputA.sessionContextBlock).toContain("<task_summary>");
    expect(outputA.sessionContextBlock).toContain("<completed_steps>");
    expect(outputA.sessionContextBlock).toContain("<pending_steps>");
    expect(outputA.sessionContextBlock).toContain("<file_modifications>");
    expect(outputA.sessionContextBlock).toContain("<git_state>");
    expect(outputA.sessionContextBlock).toContain("<key_decisions>");
    expect(outputA.sessionContextBlock).toContain("<interruption detected=\"true\">");
  });

  it("omits interruption section when no interruption signal exists", () => {
    const config = resolveContextCompactionConfig(
      { compaction: { preserve_recent_turns: 1 } },
      {},
    );
    const compactor = new ProgrammaticCompactor(config);
    const cleanGit: GitSnapshot = {
      ...gitSnapshot,
      stagedChanges: 0,
      unstagedChanges: 0,
    };
    const history: CompactionHistory = {
      messages: [
        {
          id: "m-1",
          role: "user",
          content: "Add docs improvements",
        },
        {
          id: "m-2",
          role: "assistant",
          content: "All requested docs updates are complete.",
        },
      ],
    };

    const output = compactor.compact(history, cleanGit);
    expect(output.sessionContextBlock).not.toContain("<interruption");
    expect(output.interruption).toBeNull();
  });
});
