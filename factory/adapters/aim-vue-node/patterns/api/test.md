# Service Test Pattern

## Convention

Unit tests use Vitest with `vi.mock` to replace the database pool. Tests verify SQL
correctness and audit behavior without a real database. One file per service, `describe`
per method, `beforeEach` resets mocks.

## Template

```ts
import { describe, it, expect, vi, beforeEach } from 'vitest'

const { mockQuery } = vi.hoisted(() => ({ mockQuery: vi.fn() }))
vi.mock('../db.js', () => ({ pool: { query: mockQuery } }))

import { {entity}Service } from './{entity}.service.js'

import { createSample{Entity} } from '@shared/fixtures/index.js'

const ctx = { userId: 'user-1', email: 'test@example.com', roles: ['admin'], organizationId: null }
const sample{Entity} = createSample{Entity}()

describe('{entity}Service.findById', () => {
  beforeEach(() => vi.clearAllMocks())
  it('returns the entity when found', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [sample{Entity}] })
    const result = await {entity}Service.findById('{entity}-1')
    expect(result).toEqual(sample{Entity})
    expect(mockQuery).toHaveBeenCalledWith(expect.stringContaining('WHERE {entity}_id = $1'), ['{entity}-1'])
  })
  it('returns null when not found', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [] })
    expect(await {entity}Service.findById('no-such')).toBeNull()
  })
})

describe('{entity}Service.create', () => {
  beforeEach(() => vi.clearAllMocks())
  it('inserts row and writes audit entry', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [sample{Entity}] }).mockResolvedValueOnce({ rows: [] })
    const result = await {entity}Service.create({ /* input */ }, ctx)
    expect(result.{entity}_id).toBeDefined()
    expect(mockQuery).toHaveBeenCalledTimes(2)
    expect(mockQuery.mock.calls[1]![0]).toContain('audit_entry')
  })
})

describe('{entity}Service.update', () => {
  beforeEach(() => vi.clearAllMocks())
  it('returns unchanged entity when no fields provided', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [sample{Entity}] })
    expect(await {entity}Service.update('{entity}-1', {}, ctx)).toEqual(sample{Entity})
  })
  it('updates provided fields and writes audit', async () => {
    const updated = { ...sample{Entity}, {field}: 'New' }
    mockQuery.mockResolvedValueOnce({ rows: [updated] }).mockResolvedValueOnce({ rows: [] })
    const result = await {entity}Service.update('{entity}-1', { {fieldName}: 'New' }, ctx)
    expect(result?.{field}).toBe('New')
    expect(mockQuery).toHaveBeenCalledTimes(2)
  })
})
```

## Example

```ts
// apps/api-internal/src/services/organization.service.test.ts
const { mockQuery } = vi.hoisted(() => ({ mockQuery: vi.fn() }))
vi.mock('../db.js', () => ({ pool: { query: mockQuery } }))
import { organizationService } from './organization.service.js'

import { createSampleOrganization } from '@shared/fixtures/index.js'

const ctx = { userId: 'user-1', email: 'a@b.com', roles: ['applicant'], organizationId: null }
const sampleOrg = createSampleOrganization()

describe('organizationService.findById', () => {
  beforeEach(() => vi.clearAllMocks())
  it('returns the organization when found', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [sampleOrg] })
    expect(await organizationService.findById('org-1')).toEqual(sampleOrg)
    expect(mockQuery).toHaveBeenCalledWith(expect.stringContaining('WHERE organization_id = $1'), ['org-1'])
  })
})

describe('organizationService.create', () => {
  beforeEach(() => vi.clearAllMocks())
  it('inserts org and writes audit entry', async () => {
    mockQuery.mockResolvedValueOnce({ rows: [sampleOrg] }).mockResolvedValueOnce({ rows: [] })
    const result = await organizationService.create({
      organizationName: 'Test Shelter Society', legalEntityType: 'Registered Society',
      yearEstablished: 2010, registrationNumber: 'RS-12345',
      orgAddrStreet: '123 Main St', orgAddrCity: 'Edmonton', orgAddrProvince: 'AB', orgAddrPostalCode: 'T5A 0A1',
    }, ctx)
    expect(result.organization_name).toBe('Test Shelter Society')
    expect(mockQuery).toHaveBeenCalledTimes(2)
    expect(mockQuery.mock.calls[1]![0]).toContain('audit_entry')
  })
})
```

## Naming

- File: `apps/{stack}/src/services/{entity}.service.test.ts` (next to the service)
- Describe blocks: `'{entity}Service.{method}'` -- Runner: Vitest (`npm test`)

## Rules

1. **vi.hoisted for mockQuery.** Declare inside `vi.hoisted()` so it exists before `vi.mock`.
2. **vi.mock replaces pool.** `vi.mock('../db.js', () => ({ pool: { query: mockQuery } }))`.
3. **Import service after mock.** The mock must be set up before the service module loads.
4. **beforeEach clears mocks.** `vi.clearAllMocks()` in every describe block.
5. **mockResolvedValueOnce per query.** Chain in call order: data query, then audit.
6. **Mock shape: `{ rows: [...] }`.** Matches `pg` QueryResult.
7. **Assert SQL with stringContaining.** Never match full SQL literally.
8. **Assert audit writes.** Verify `mockQuery` called N+1 times; last call contains `'audit_entry'`.
9. **Import sample data from fixture module.** Use `createSample{Entity}()` from `@shared/fixtures/index.js`. Override fields per test with `createSample{Entity}({ field: 'value' })`.
10. **Unit tests only.** No database; integration tests live separately.
