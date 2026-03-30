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

    const fixedCompactedAt = new Date(0);
    const outputA = compactor.compact(history, gitSnapshot, fixedCompactedAt);
    const outputB = compactor.compact(history, gitSnapshot, fixedCompactedAt);

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

  it("preserves pinned messages and recent turns byte-identically", () => {
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
          content: "System prompt",
        },
        {
          id: "m-2",
          role: "user",
          content: "Older context",
        },
        {
          id: "m-3",
          role: "assistant",
          content: "Pinned and important",
          pinned: true,
        },
        {
          id: "m-4",
          role: "user",
          content: "Recent user turn A",
        },
        {
          id: "m-5",
          role: "assistant",
          content: "Recent assistant turn A",
        },
        {
          id: "m-6",
          role: "assistant",
          content: "Tool prelude within same turn",
        },
        {
          id: "m-7",
          role: "user",
          content: "Recent user turn B",
        },
        {
          id: "m-8",
          role: "assistant",
          content: "Recent assistant turn B",
        },
      ],
    };

    const output = compactor.compact(history, { ...gitSnapshot, stagedChanges: 0, unstagedChanges: 0 });
    const preservedIds = output.preservedMessages.map((message) => message.id);

    expect(preservedIds).toEqual(["m-1", "m-3", "m-4", "m-5", "m-6", "m-7", "m-8"]);
    expect(output.preservedMessages[1]?.content).toBe("Pinned and important");
    expect(output.preservedMessages[5]?.content).toBe("Recent user turn B");
  });

  it("preserves only active-operation tool messages", () => {
    const config = resolveContextCompactionConfig(
      { compaction: { preserve_recent_turns: 1 } },
      {},
    );
    const compactor = new ProgrammaticCompactor(config);
    const history: CompactionHistory = {
      messages: [
        {
          id: "m-1",
          role: "assistant",
          content: [{ type: "tool_use", id: "tool-old", name: "read_file" }],
        },
        {
          id: "m-2",
          role: "tool",
          content: "old result",
          tool_call_id: "tool-old",
        },
        {
          id: "m-3",
          role: "assistant",
          content: [{ type: "tool_use", id: "tool-active", name: "edit_file" }],
        },
        {
          id: "m-4",
          role: "user",
          content: "continue",
        },
      ],
    };

    const output = compactor.compact(history, { ...gitSnapshot, stagedChanges: 0, unstagedChanges: 0 });
    const preservedIds = output.preservedMessages.map((message) => message.id);
    expect(preservedIds).toContain("m-3");
    expect(preservedIds).not.toContain("m-2");
  });

  it("emits interruption for true-positive multi-signal case", () => {
    const config = resolveContextCompactionConfig(
      { compaction: { preserve_recent_turns: 1 } },
      {},
    );
    const compactor = new ProgrammaticCompactor(config);
    const history: CompactionHistory = {
      messages: [
        {
          id: "m-1",
          role: "user",
          content: "Ship this change",
        },
        {
          id: "m-2",
          role: "assistant",
          content: "Do the next step now?",
        },
      ],
    };

    const output = compactor.compact(history, { ...gitSnapshot, stagedChanges: 1, unstagedChanges: 0 });
    expect(output.interruption).not.toBeNull();
    expect(output.sessionContextBlock).toContain("<interruption detected=\"true\">");
  });

  it("does not emit interruption for false-positive single-signal case", () => {
    const config = resolveContextCompactionConfig(
      { compaction: { preserve_recent_turns: 1 } },
      {},
    );
    const compactor = new ProgrammaticCompactor(config);
    const history: CompactionHistory = {
      messages: [
        {
          id: "m-1",
          role: "user",
          content: "Implement docs improvements",
        },
        {
          id: "m-2",
          role: "assistant",
          content: "- [ ] update README",
        },
      ],
    };

    const output = compactor.compact(history, { ...gitSnapshot, stagedChanges: 0, unstagedChanges: 0 });
    expect(output.interruption).toBeNull();
    expect(output.sessionContextBlock).not.toContain("<interruption");
  });
});
