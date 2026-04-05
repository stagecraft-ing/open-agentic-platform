# SQL Query Patterns

## Convention
All DB access uses `pg` Pool with parameterized queries. No ORM.
Services call `pool.query<T>()`. Dynamic queries use `$idx++` counters.

## Pattern A -- Select by ID
```typescript
async getById(id: string): Promise<FundingRequest | null> {
  const { rows } = await pool.query<FundingRequest>(
    `SELECT * FROM funding_request WHERE id = $1`, [id]
  );
  return rows[0] ?? null;
}
```

## Pattern B -- Dynamic WHERE + Pagination
```typescript
async list(filters: RequestFilters, page: number, size: number) {
  const conditions: string[] = [];
  const params: unknown[] = [];
  let idx = 1;
  if (filters.status) { conditions.push(`status = $${idx++}`); params.push(filters.status); }
  if (filters.programId) { conditions.push(`program_id = $${idx++}`); params.push(filters.programId); }
  const where = conditions.length ? `WHERE ${conditions.join(' AND ')}` : '';
  const countRes = await pool.query<{ count: string }>(`SELECT COUNT(*) FROM funding_request ${where}`, params);
  const total = parseInt(countRes.rows[0].count, 10);
  params.push(size, (page - 1) * size);
  const { rows } = await pool.query<FundingRequest>(
    `SELECT * FROM funding_request ${where} ORDER BY created_at DESC LIMIT $${idx++} OFFSET $${idx++}`, params
  );
  return { data: rows, total, page, size };
}
```

## Pattern C -- INSERT with RETURNING
```typescript
async create(input: CreateRequest): Promise<FundingRequest> {
  const { rows } = await pool.query<FundingRequest>(
    `INSERT INTO funding_request (applicant_id, program_id, title, amount_requested, status)
     VALUES ($1, $2, $3, $4, $5) RETURNING *`,
    [input.applicantId, input.programId, input.title, input.amountRequested, 'draft']
  );
  return rows[0];
}
```

## Pattern D -- Dynamic UPDATE with fieldMap
```typescript
async update(id: string, input: Partial<UpdateRequest>) {
  const fieldMap: Record<string, string> = {
    title: 'title', amountRequested: 'amount_requested',
    status: 'status', description: 'description',
  };
  const sets: string[] = [];
  const params: unknown[] = [];
  let idx = 1;
  for (const [jsKey, dbCol] of Object.entries(fieldMap)) {
    if ((input as Record<string, unknown>)[jsKey] !== undefined) {
      sets.push(`${dbCol} = $${idx++}`);
      params.push((input as Record<string, unknown>)[jsKey]);
    }
  }
  if (!sets.length) return null;
  sets.push(`updated_at = CURRENT_TIMESTAMP`);
  params.push(id);
  const { rows } = await pool.query<FundingRequest>(
    `UPDATE funding_request SET ${sets.join(', ')} WHERE id = $${idx} RETURNING *`, params
  );
  return rows[0] ?? null;
}
```

## Pattern E -- Transaction
```typescript
async submitWithDocuments(requestId: string, docs: Doc[]) {
  const client = await pool.connect();
  try {
    await client.query('BEGIN');
    await client.query(
      `UPDATE funding_request SET status = 'submitted', submitted_at = CURRENT_TIMESTAMP WHERE id = $1`,
      [requestId]
    );
    for (const doc of docs) {
      await client.query(`INSERT INTO document (request_id, name, url) VALUES ($1, $2, $3)`,
        [requestId, doc.name, doc.url]);
    }
    await client.query('COMMIT');
  } catch (err) {
    await client.query('ROLLBACK');
    throw err;
  } finally { client.release(); }
}
```

## Pattern F -- Upsert
```typescript
await pool.query(
  `INSERT INTO user_preference (user_id, key, value) VALUES ($1, $2, $3)
   ON CONFLICT (user_id, key) DO UPDATE SET value = $3`, [userId, key, value]
);
```

## Pattern G -- JOIN with array_agg
```typescript
const { rows } = await pool.query<RequestWithTags>(
  `SELECT r.*, COALESCE(array_agg(t.name) FILTER (WHERE t.name IS NOT NULL), '{}') AS tags
   FROM funding_request r LEFT JOIN request_tag rt ON rt.request_id = r.id
   LEFT JOIN tag t ON t.id = rt.tag_id WHERE r.id = $1 GROUP BY r.id`, [id]
);
```

## Rules
1. Always parameterize -- never interpolate user input into SQL.
2. Use `$idx++` counter for dynamic parameter positions.
3. Always type the generic: `pool.query<MyType>(...)`.
4. Use `RETURNING *` on INSERT/UPDATE to avoid a second SELECT.
5. Transactions use `pool.connect()` with BEGIN/COMMIT/ROLLBACK in try/catch/finally.
6. Map camelCase TS fields to snake_case DB columns in the fieldMap.
