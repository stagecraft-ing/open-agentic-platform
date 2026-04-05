# Service Pattern

## Convention

A service is a plain object (not a class) with async methods. It owns all SQL for one
entity. Direct `pool.query<RowType>()` with parameterized SQL -- no ORM. Every mutation
takes `UserContext` and writes an audit trail. IDs from `randomUUID()`. camelCase inputs
mapped to snake_case columns via `fieldMap`.

## Template

```ts
import { randomUUID } from 'crypto'
import { pool } from '../db.js'
import type { UserContext } from '../middleware/user-context.middleware.js'

export interface {Entity}Row { {entity}_id: string; /* snake_case cols */ created_at: string; updated_at: string }
export interface Create{Entity}Input { {fieldName}: string; /* camelCase fields */ }
export type Update{Entity}Input = Partial<Create{Entity}Input>

export const {entity}Service = {
  async findById({entity}Id: string): Promise<{Entity}Row | null> {
    const r = await pool.query<{Entity}Row>('SELECT * FROM {table} WHERE {entity}_id = $1', [{entity}Id])
    return r.rows[0] ?? null
  },

  async findAll(filters: Record<string, unknown>, page: number, limit: number) {
    const where: string[] = []; const params: unknown[] = []; let idx = 1
    if (filters.status) { where.push(`status = $${idx++}`); params.push(filters.status) }
    const clause = where.length ? `WHERE ${where.join(' AND ')}` : ''
    const offset = (page - 1) * limit
    const [data, count] = await Promise.all([
      pool.query<{Entity}Row>(`SELECT * FROM {table} ${clause} ORDER BY created_at DESC LIMIT $${idx++} OFFSET $${idx++}`, [...params, limit, offset]),
      pool.query<{ count: string }>(`SELECT COUNT(*) FROM {table} ${clause}`, params),
    ])
    return { rows: data.rows, total: parseInt(count.rows[0]!.count, 10) }
  },

  async create(input: Create{Entity}Input, ctx: UserContext): Promise<{Entity}Row> {
    const id = randomUUID()
    const r = await pool.query<{Entity}Row>(
      `INSERT INTO {table} ({entity}_id, {col}, created_at, updated_at)
       VALUES ($1,$2,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP) RETURNING *`, [id, input.{fieldName}])
    await audit{Entity}(id, 'create', ctx)
    return r.rows[0] as {Entity}Row
  },

  async update({entity}Id: string, input: Update{Entity}Input, ctx: UserContext) {
    const fields: string[] = []; const values: unknown[] = []; let idx = 1
    const fieldMap: Record<keyof Update{Entity}Input, string> = { {fieldName}: '{column_name}' }
    for (const [key, col] of Object.entries(fieldMap)) {
      if (key in input) { fields.push(`${col} = $${idx++}`); values.push((input as Record<string, unknown>)[key] ?? null) }
    }
    if (fields.length === 0) return this.findById({entity}Id)
    fields.push('updated_at = CURRENT_TIMESTAMP'); values.push({entity}Id)
    const r = await pool.query<{Entity}Row>(
      `UPDATE {table} SET ${fields.join(', ')} WHERE {entity}_id = $${idx} RETURNING *`, values)
    if (r.rows[0]) await audit{Entity}({entity}Id, 'update', ctx)
    return r.rows[0] ?? null
  },

  async delete({entity}Id: string, ctx: UserContext): Promise<boolean> {
    const r = await pool.query('DELETE FROM {table} WHERE {entity}_id = $1', [{entity}Id])
    if (r.rowCount && r.rowCount > 0) { await audit{Entity}({entity}Id, 'delete', ctx); return true }
    return false
  },
}

async function audit{Entity}(entityId: string, action: string, ctx: UserContext): Promise<void> {
  await pool.query(
    `INSERT INTO audit_entry (audit_id, user_id, action_code, entity_type, entity_id, action_timestamp, ip_address)
     VALUES ($1,$2,$3,'{Entity}',$4,CURRENT_TIMESTAMP,'server')`, [randomUUID(), ctx.userId, action, entityId])
}
```

## Example

```ts
// apps/api-internal/src/services/organization.service.ts
export const organizationService = {
  async findById(organizationId: string): Promise<OrganizationRow | null> {
    const r = await pool.query<OrganizationRow>(
      'SELECT * FROM organization WHERE organization_id = $1', [organizationId])
    return r.rows[0] ?? null
  },
  async create(input: CreateOrganizationInput, ctx: UserContext) {
    const id = randomUUID()
    const r = await pool.query<OrganizationRow>(
      `INSERT INTO organization (organization_id, organization_name, legal_entity_type,
         year_established, registration_number, created_at, updated_at)
       VALUES ($1,$2,$3,$4,$5,CURRENT_TIMESTAMP,CURRENT_TIMESTAMP) RETURNING *`,
      [id, input.organizationName, input.legalEntityType, input.yearEstablished, input.registrationNumber])
    await auditOrganization(id, 'create', ctx)
    return r.rows[0] as OrganizationRow
  },
  // update uses fieldMap: { organizationName: 'organization_name', ... }
}
```

## Naming

- File: `apps/{stack}/src/services/{entity}.service.ts`
- Export: `export const {entity}Service = { ... }` (object, not class)
- Types: `{Entity}Row`, `Create{Entity}Input`, `Update{Entity}Input`

## Rules

1. **Object, not class.** Callers use `{entity}Service.method()`.
2. **Direct SQL only.** `pool.query<RowType>(sql, params)` -- no ORM, no query builder.
3. **Typed generics.** Always: `pool.query<OrganizationRow>(...)`.
4. **Parameterized queries.** Never interpolate user input into SQL.
5. **randomUUID() for IDs.** Generated in the service, not the database.
6. **Audit every mutation.** Private `audit{Entity}()` called after create/update/delete.
7. **UserContext on mutations.** Last parameter on every write method.
8. **fieldMap for updates.** camelCase keys to snake_case columns; dynamic `$${idx++}` counter.
9. **Parallel COUNT.** `findAll` runs data + COUNT queries via `Promise.all`.
10. **Dual-stack rule.** Direct pool access is `api-internal` only. `api-public` uses `proxyRequest()`.
