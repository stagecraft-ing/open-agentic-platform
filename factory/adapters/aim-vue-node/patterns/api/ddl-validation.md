# DDL Column Validation Test Pattern

Static tests that parse SQL query strings in service files and verify every
referenced column exists in the DDL. No database or Docker required.

## Why This Exists

The most common class of runtime failure in factory-generated applications:
a service defines a local type with shortened property names (e.g., `status`),
while the DDL uses the full name (`application_status`). The service builds SQL
using its local property name. TypeScript compiles, mocked unit tests pass
(mocks don't validate column names), but every query against a real database
fails with `column "status" does not exist`.

These tests catch that class of bug at build time.

## Shared Utility

Generate once per project at `tests/utils/ddl-column-validator.ts`:

```ts
// tests/utils/ddl-column-validator.ts
import { readFileSync, readdirSync } from 'fs'
import { join } from 'path'

const MIGRATION_DIR = 'packages/db/migrations'

/** Parse CREATE TABLE and ALTER TABLE statements to extract column names per table. */
export function parseDDLColumns(migrationDir = MIGRATION_DIR): Map<string, Set<string>> {
  const tables = new Map<string, Set<string>>()
  const files = readdirSync(migrationDir).filter(f => f.endsWith('.sql')).sort()

  for (const file of files) {
    const sql = readFileSync(join(migrationDir, file), 'utf-8')

    // CREATE TABLE
    const createRe = /CREATE TABLE (?:IF NOT EXISTS )?(\w+)\s*\(([\s\S]*?)\);/gi
    for (const match of sql.matchAll(createRe)) {
      const table = match[1]!
      const body = match[2]!
      const cols = new Set<string>()
      for (const line of body.split(',')) {
        const colMatch = line.trim().match(/^(\w+)\s+(VARCHAR|TEXT|INTEGER|BIGINT|BOOLEAN|UUID|TIMESTAMP|DATE|NUMERIC|DECIMAL|SERIAL)/i)
        if (colMatch) cols.add(colMatch[1]!)
      }
      tables.set(table, cols)
    }

    // ALTER TABLE ADD COLUMN
    const alterRe = /ALTER TABLE (\w+)\s+ADD COLUMN (\w+)/gi
    for (const match of sql.matchAll(alterRe)) {
      const table = match[1]!
      const col = match[2]!
      if (!tables.has(table)) tables.set(table, new Set())
      tables.get(table)!.add(col)
    }
  }

  return tables
}

/** Extract SQL column references from a service file's query strings. */
export function extractSQLColumns(serviceSource: string): Array<{ table: string; columns: string[] }> {
  const results: Array<{ table: string; columns: string[] }> = []
  // Match pool.query template literals and string literals
  const queryRe = /query[^(]*\(\s*(?:`([^`]+)`|'([^']+)'|"([^"]+)")/g
  for (const match of serviceSource.matchAll(queryRe)) {
    const sql = match[1] ?? match[2] ?? match[3] ?? ''
    const cols = new Set<string>()

    // SELECT columns
    const selectRe = /SELECT\s+([\s\S]*?)\s+FROM/gi
    for (const s of sql.matchAll(selectRe)) {
      if (s[1]!.trim() !== '*') {
        for (const col of s[1]!.split(',')) {
          const name = col.trim().split(/\s+AS\s+/i)[0]!.split('.').pop()!.trim()
          if (name && !name.startsWith('$') && !name.match(/^\d/)) cols.add(name)
        }
      }
    }

    // WHERE, ORDER BY, GROUP BY columns
    for (const clause of ['WHERE', 'ORDER BY', 'GROUP BY']) {
      const re = new RegExp(`${clause}\\s+([\\s\\S]*?)(?:LIMIT|OFFSET|RETURNING|ORDER|GROUP|HAVING|$)`, 'gi')
      for (const m of sql.matchAll(re)) {
        for (const part of m[1]!.split(/\s+AND\s+|\s+OR\s+|,/i)) {
          const col = part.trim().split(/\s+/)[0]!.split('.').pop()!.trim()
          if (col && !col.startsWith('$') && !col.match(/^\d|^'|^"/) && col !== '') cols.add(col)
        }
      }
    }

    // INSERT INTO columns
    const insertRe = /INSERT INTO (\w+)\s*\(([^)]+)\)/gi
    for (const ins of sql.matchAll(insertRe)) {
      const table = ins[1]!
      for (const col of ins[2]!.split(',')) {
        const name = col.trim()
        if (name) cols.add(name)
      }
      results.push({ table, columns: [...cols] })
      cols.clear()
    }

    // UPDATE SET columns
    const updateRe = /UPDATE (\w+)\s+SET\s+([\s\S]*?)\s+WHERE/gi
    for (const upd of sql.matchAll(updateRe)) {
      const table = upd[1]!
      for (const assign of upd[2]!.split(',')) {
        const col = assign.trim().split(/\s*=/)[0]!.trim()
        if (col && !col.startsWith('$')) cols.add(col)
      }
      results.push({ table, columns: [...cols] })
      cols.clear()
    }

    // Collect remaining if not already pushed
    if (cols.size > 0) {
      const fromRe = /FROM\s+(\w+)/i
      const tableMatch = sql.match(fromRe)
      results.push({ table: tableMatch?.[1] ?? 'UNKNOWN', columns: [...cols] })
    }
  }

  return results
}

/** Validate that all SQL column references exist in DDL. Returns mismatches. */
export function validateColumns(
  serviceFile: string,
  serviceSource: string,
  ddl: Map<string, Set<string>>
): Array<{ serviceFile: string; table: string; column: string; ddlColumns: string[] }> {
  const mismatches: Array<{ serviceFile: string; table: string; column: string; ddlColumns: string[] }> = []
  const refs = extractSQLColumns(serviceSource)

  for (const { table, columns } of refs) {
    const ddlCols = ddl.get(table)
    if (!ddlCols) continue // table not found — separate check
    for (const col of columns) {
      if (!ddlCols.has(col)) {
        mismatches.push({ serviceFile, table, column: col, ddlColumns: [...ddlCols] })
      }
    }
  }

  return mismatches
}
```

## Per-Service Test

Generate one per service at `apps/{stack}/src/services/{entity}.columns.test.ts`:

```ts
// apps/api-internal/src/services/{entity}.columns.test.ts
import { describe, it, expect } from 'vitest'
import { readFileSync } from 'fs'
import { parseDDLColumns, validateColumns } from '../../../../tests/utils/ddl-column-validator.js'

const SERVICE_PATH = 'apps/api-internal/src/services/{entity}.service.ts'

describe('{entity} service SQL column alignment', () => {
  const ddl = parseDDLColumns()
  const source = readFileSync(SERVICE_PATH, 'utf-8')

  it('references only columns that exist in the DDL', () => {
    const mismatches = validateColumns(SERVICE_PATH, source, ddl)
    if (mismatches.length > 0) {
      const report = mismatches.map(m =>
        `  ${m.column} not in ${m.table} (available: ${m.ddlColumns.join(', ')})`
      ).join('\n')
      throw new Error(`SQL column mismatches:\n${report}`)
    }
  })
})
```

## Rules

1. **No database required.** Tests read DDL migration files and service source at test time.
2. **One test file per service.** Named `{entity}.columns.test.ts` next to the service.
3. **Shared utility generated once.** On the first service scaffolded, create `tests/utils/ddl-column-validator.ts`.
4. **Assert against live DDL.** Tests parse migration files at runtime — never hardcode expected column names, as migrations evolve.
5. **Run with `npm test`.** These are standard Vitest tests — they execute alongside unit tests in CI.
