/**
 * Session Memory types — FR-002 memory entry schema.
 */

/** The kind of knowledge captured in a memory entry. */
export type MemoryKind = "decision" | "correction" | "pattern" | "note" | "preference";

/** Importance tiers with ascending durability. */
export type ImportanceLevel = "ephemeral" | "short-term" | "medium-term" | "long-term" | "permanent";

/** A single persisted memory entry (FR-002). */
export interface MemoryEntry {
  id: string;
  content: string;
  kind: MemoryKind;
  importance: ImportanceLevel;
  expiresAt: number | null;
  projectScope: string;
  tags: string[];
  sourceSessionId: string;
  accessCount: number;
  createdAt: number;
  updatedAt: number;
}

/** Input for creating a new memory entry via memory_store. */
export interface StoreMemoryInput {
  content: string;
  kind: MemoryKind;
  importance?: ImportanceLevel;
  tags?: string[];
  projectScope?: string;
  sourceSessionId?: string;
}

/** Filters for querying memories (FR-005). */
export interface QueryMemoryInput {
  text?: string;
  tags?: string[];
  kind?: MemoryKind;
  importance?: ImportanceLevel;
  projectScope: string;
  limit?: number;
}

/** Input for listing memories with pagination. */
export interface ListMemoryInput {
  projectScope: string;
  kind?: MemoryKind;
  limit?: number;
  offset?: number;
}

/** Default expiry durations in seconds for each importance level. */
export const EXPIRY_DEFAULTS: Record<ImportanceLevel, number | null> = {
  ephemeral: 0,            // expires at session end (caller sets actual timestamp)
  "short-term": 86_400,    // 24 hours
  "medium-term": 604_800,  // 7 days
  "long-term": 7_776_000,  // 90 days
  permanent: null,          // never expires
};

/** Importance promotion order — index determines rank. */
export const IMPORTANCE_ORDER: ImportanceLevel[] = [
  "ephemeral",
  "short-term",
  "medium-term",
  "long-term",
  "permanent",
];

/** Number of accesses required to trigger promotion (FR-007 / SC-005). */
export const PROMOTION_ACCESS_THRESHOLD = 3;
