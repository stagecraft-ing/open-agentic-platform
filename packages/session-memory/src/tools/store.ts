/**
 * memory_store tool handler (FR-001).
 */

import type { MemoryStorage } from "../storage/sqlite.js";
import type { StoreMemoryInput, MemoryEntry } from "../types.js";

export interface StoreToolInput {
  content: string;
  kind: string;
  importance?: string;
  tags?: string[];
  projectScope?: string;
  sourceSessionId?: string;
}

export function handleMemoryStore(storage: MemoryStorage, input: StoreToolInput, defaults: { projectScope: string; sourceSessionId: string }): MemoryEntry {
  if (!input.content || typeof input.content !== "string") {
    throw new Error("content is required and must be a string");
  }

  const validKinds = ["decision", "correction", "pattern", "note", "preference"];
  if (!validKinds.includes(input.kind)) {
    throw new Error(`kind must be one of: ${validKinds.join(", ")}`);
  }

  const validImportance = ["ephemeral", "short-term", "medium-term", "long-term", "permanent"];
  if (input.importance && !validImportance.includes(input.importance)) {
    throw new Error(`importance must be one of: ${validImportance.join(", ")}`);
  }

  const storeInput: StoreMemoryInput = {
    content: input.content,
    kind: input.kind as StoreMemoryInput["kind"],
    importance: (input.importance as StoreMemoryInput["importance"]) ?? undefined,
    tags: input.tags,
    projectScope: input.projectScope ?? defaults.projectScope,
    sourceSessionId: input.sourceSessionId ?? defaults.sourceSessionId,
  };

  return storage.store(storeInput);
}
