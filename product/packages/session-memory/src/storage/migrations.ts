/**
 * SQLite schema migrations for session memory (NF-001).
 */

export interface Migration {
  version: number;
  description: string;
  up: string;
}

export const MIGRATIONS: Migration[] = [
  {
    version: 1,
    description: "Initial memory entries table",
    up: `
      CREATE TABLE IF NOT EXISTS memory_entries (
        id            TEXT PRIMARY KEY,
        content       TEXT NOT NULL,
        kind          TEXT NOT NULL CHECK (kind IN ('decision', 'correction', 'pattern', 'note', 'preference')),
        importance    TEXT NOT NULL CHECK (importance IN ('ephemeral', 'short-term', 'medium-term', 'long-term', 'permanent')),
        expires_at    INTEGER,
        project_scope TEXT NOT NULL,
        tags          TEXT NOT NULL DEFAULT '[]',
        source_session_id TEXT NOT NULL,
        access_count  INTEGER NOT NULL DEFAULT 0,
        created_at    INTEGER NOT NULL,
        updated_at    INTEGER NOT NULL
      );

      CREATE INDEX IF NOT EXISTS idx_memory_project_scope ON memory_entries (project_scope);
      CREATE INDEX IF NOT EXISTS idx_memory_kind ON memory_entries (kind);
      CREATE INDEX IF NOT EXISTS idx_memory_importance ON memory_entries (importance);
      CREATE INDEX IF NOT EXISTS idx_memory_expires_at ON memory_entries (expires_at);
      CREATE INDEX IF NOT EXISTS idx_memory_created_at ON memory_entries (created_at DESC);

      CREATE TABLE IF NOT EXISTS schema_version (
        version INTEGER PRIMARY KEY
      );
      INSERT INTO schema_version (version) VALUES (1);
    `,
  },
];

export function getCurrentVersion(db: { prepare: (sql: string) => { get: () => { version: number } | undefined } }): number {
  try {
    const row = db.prepare("SELECT version FROM schema_version ORDER BY version DESC LIMIT 1").get();
    return row?.version ?? 0;
  } catch {
    return 0;
  }
}

export function applyMigrations(db: { exec: (sql: string) => void; prepare: (sql: string) => { get: () => { version: number } | undefined } }): void {
  const current = getCurrentVersion(db);
  for (const migration of MIGRATIONS) {
    if (migration.version > current) {
      db.exec(migration.up);
    }
  }
}
