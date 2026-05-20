/**
 * SQLite storage layer for session memory (NF-001).
 *
 * Provides CRUD operations over a project-scoped SQLite database.
 * Each project gets its own .session-memory/memory.db file at the project root.
 */

import { randomUUID } from "node:crypto";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import Database from "better-sqlite3";
import type {
  ImportanceLevel,
  ListMemoryInput,
  MemoryEntry,
  MemoryKind,
  QueryMemoryInput,
  StoreMemoryInput,
} from "../types.js";
import { EXPIRY_DEFAULTS } from "../types.js";
import { applyMigrations } from "./migrations.js";

/** Row shape returned from SQLite (snake_case). */
interface MemoryRow {
  id: string;
  content: string;
  kind: string;
  importance: string;
  expires_at: number | null;
  project_scope: string;
  tags: string;
  source_session_id: string;
  access_count: number;
  created_at: number;
  updated_at: number;
}

function rowToEntry(row: MemoryRow): MemoryEntry {
  return {
    id: row.id,
    content: row.content,
    kind: row.kind as MemoryKind,
    importance: row.importance as ImportanceLevel,
    expiresAt: row.expires_at,
    projectScope: row.project_scope,
    tags: JSON.parse(row.tags) as string[],
    sourceSessionId: row.source_session_id,
    accessCount: row.access_count,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

/** Resolve the database path for a project. */
export function dbPath(projectScope: string): string {
  return join(projectScope, ".session-memory", "memory.db");
}

export class MemoryStorage {
  private db: Database.Database;

  constructor(databasePath: string) {
    mkdirSync(join(databasePath, ".."), { recursive: true });
    this.db = new Database(databasePath);
    this.db.pragma("journal_mode = WAL");
    this.db.pragma("foreign_keys = ON");
    applyMigrations(this.db);
  }

  /** Create from a project root path (convenience). */
  static forProject(projectScope: string): MemoryStorage {
    return new MemoryStorage(dbPath(projectScope));
  }

  /** Store a new memory entry (FR-001 memory_store). */
  store(input: StoreMemoryInput): MemoryEntry {
    const now = Math.floor(Date.now() / 1000);
    const importance = input.importance ?? "medium-term";
    const expiryDelta = EXPIRY_DEFAULTS[importance];
    const expiresAt = expiryDelta === null ? null : now + expiryDelta;

    const entry: MemoryEntry = {
      id: randomUUID(),
      content: input.content,
      kind: input.kind,
      importance,
      expiresAt,
      projectScope: input.projectScope ?? "",
      tags: input.tags ?? [],
      sourceSessionId: input.sourceSessionId ?? "",
      accessCount: 0,
      createdAt: now,
      updatedAt: now,
    };

    this.db.prepare(`
      INSERT INTO memory_entries (id, content, kind, importance, expires_at, project_scope, tags, source_session_id, access_count, created_at, updated_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(
      entry.id,
      entry.content,
      entry.kind,
      entry.importance,
      entry.expiresAt,
      entry.projectScope,
      JSON.stringify(entry.tags),
      entry.sourceSessionId,
      entry.accessCount,
      entry.createdAt,
      entry.updatedAt,
    );

    return entry;
  }

  /** Get a single entry by ID. */
  getById(id: string): MemoryEntry | null {
    const row = this.db.prepare("SELECT * FROM memory_entries WHERE id = ?").get(id) as MemoryRow | undefined;
    return row ? rowToEntry(row) : null;
  }

  /** Query memories with filtering (FR-005). */
  query(input: QueryMemoryInput): MemoryEntry[] {
    const conditions: string[] = ["project_scope = ?"];
    const params: unknown[] = [input.projectScope];

    if (input.kind) {
      conditions.push("kind = ?");
      params.push(input.kind);
    }

    if (input.importance) {
      conditions.push("importance = ?");
      params.push(input.importance);
    }

    if (input.tags && input.tags.length > 0) {
      // Tag matching: at least one tag must match (OR semantics)
      const tagConditions = input.tags.map(() => "tags LIKE ?");
      conditions.push(`(${tagConditions.join(" OR ")})`);
      for (const tag of input.tags) {
        params.push(`%"${tag}"%`);
      }
    }

    if (input.text) {
      conditions.push("content LIKE ?");
      params.push(`%${input.text}%`);
    }

    const limit = input.limit ?? 50;
    params.push(limit);

    const sql = `SELECT * FROM memory_entries WHERE ${conditions.join(" AND ")} ORDER BY updated_at DESC LIMIT ?`;
    const rows = this.db.prepare(sql).all(...params) as MemoryRow[];

    // Bump access count and updatedAt for returned entries (FR-007)
    const now = Math.floor(Date.now() / 1000);
    const updateStmt = this.db.prepare(
      "UPDATE memory_entries SET access_count = access_count + 1, updated_at = ? WHERE id = ?",
    );
    const bumpTransaction = this.db.transaction(() => {
      for (const row of rows) {
        updateStmt.run(now, row.id);
      }
    });
    bumpTransaction();

    return rows.map((row) => ({
      ...rowToEntry(row),
      accessCount: row.access_count + 1,
      updatedAt: now,
    }));
  }

  /** List memories with pagination. */
  list(input: ListMemoryInput): MemoryEntry[] {
    const conditions: string[] = ["project_scope = ?"];
    const params: unknown[] = [input.projectScope];

    if (input.kind) {
      conditions.push("kind = ?");
      params.push(input.kind);
    }

    const limit = input.limit ?? 50;
    const offset = input.offset ?? 0;
    params.push(limit, offset);

    const sql = `SELECT * FROM memory_entries WHERE ${conditions.join(" AND ")} ORDER BY created_at DESC LIMIT ? OFFSET ?`;
    const rows = this.db.prepare(sql).all(...params) as MemoryRow[];
    return rows.map(rowToEntry);
  }

  /** Delete a memory entry by ID. Returns true if deleted. */
  delete(id: string): boolean {
    const result = this.db.prepare("DELETE FROM memory_entries WHERE id = ?").run(id);
    return result.changes > 0;
  }

  /** Delete all expired entries. Returns count of deleted entries (SC-004). */
  sweepExpired(): number {
    const now = Math.floor(Date.now() / 1000);
    const result = this.db.prepare(
      "DELETE FROM memory_entries WHERE expires_at IS NOT NULL AND expires_at <= ?",
    ).run(now);
    return result.changes;
  }

  /** Update importance level for a specific entry. */
  updateImportance(id: string, importance: ImportanceLevel, newExpiresAt: number | null): boolean {
    const now = Math.floor(Date.now() / 1000);
    const result = this.db.prepare(
      "UPDATE memory_entries SET importance = ?, expires_at = ?, updated_at = ? WHERE id = ?",
    ).run(importance, newExpiresAt, now, id);
    return result.changes > 0;
  }

  /** Get entries eligible for importance promotion (access_count >= threshold). */
  getPromotionCandidates(threshold: number): MemoryEntry[] {
    const rows = this.db.prepare(
      `SELECT * FROM memory_entries
       WHERE access_count >= ?
         AND importance NOT IN ('long-term', 'permanent')
       ORDER BY access_count DESC`,
    ).all(threshold) as MemoryRow[];
    return rows.map(rowToEntry);
  }

  /** Count entries for a project scope. */
  count(projectScope: string): number {
    const row = this.db.prepare(
      "SELECT COUNT(*) as cnt FROM memory_entries WHERE project_scope = ?",
    ).get(projectScope) as { cnt: number };
    return row.cnt;
  }

  /** Close the database connection. */
  close(): void {
    this.db.close();
  }
}
