# Controller Pattern

## Convention

A controller is a class with async methods, one per route handler. It delegates to the
service layer, maps HTTP semantics (status codes, response shapes, error codes), and
never contains business logic or SQL. A singleton instance is exported alongside the class.

## Template

```ts
import type { Request, Response } from 'express'
import { {entity}Service } from '../services/{entity}.service.js'
import type { Create{Entity}Input, Update{Entity}Input } from '../services/{entity}.service.js'
import type { UserContext } from '../middleware/user-context.middleware.js'
import { buildErrorResponse } from '../utils/error-response.js'
import { buildPaginatedResponse } from '../middleware/pagination.middleware.js'

export class {Entities}Controller {
  async list(req: Request, res: Response) {
    try {
      const { page, limit } = req.pagination as { page: number; limit: number }
      const { rows, total } = await {entity}Service.findAll(page, limit)
      res.json(buildPaginatedResponse(rows, total, req.pagination as { page: number; limit: number }))
    } catch (err) {
      res.status(500).json(buildErrorResponse(req, { code: 'INTERNAL_ERROR', message: 'Failed to list {entities}', details: err }))
    }
  }
  async get(req: Request, res: Response) {
    try {
      const item = await {entity}Service.findById(req.params.id as string)
      if (!item) return res.status(404).json(buildErrorResponse(req, { code: 'NOT_FOUND', message: '{Entity} not found' }))
      res.json({ success: true, data: item })
    } catch (err) {
      res.status(500).json(buildErrorResponse(req, { code: 'INTERNAL_ERROR', message: 'Failed to get {entity}', details: err }))
    }
  }
  async create(req: Request, res: Response) {
    try {
      const item = await {entity}Service.create(req.body as Create{Entity}Input, req.userContext as UserContext)
      res.status(201).json({ success: true, data: item })
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to create'
      const isConflict = msg.includes('unique') || msg.includes('duplicate')
      res.status(isConflict ? 409 : 400).json(buildErrorResponse(req, { code: isConflict ? 'CONFLICT' : 'BAD_REQUEST', message: msg, details: err }))
    }
  }
  async update(req: Request, res: Response) {
    try {
      const item = await {entity}Service.update(req.params.id as string, req.body as Update{Entity}Input, req.userContext as UserContext)
      if (!item) return res.status(404).json(buildErrorResponse(req, { code: 'NOT_FOUND', message: '{Entity} not found' }))
      res.json({ success: true, data: item })
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to update'
      res.status(400).json(buildErrorResponse(req, { code: 'BAD_REQUEST', message: msg, details: err }))
    }
  }
  async delete(req: Request, res: Response) {
    try {
      const deleted = await {entity}Service.delete(req.params.id as string, req.userContext as UserContext)
      if (!deleted) return res.status(404).json(buildErrorResponse(req, { code: 'NOT_FOUND', message: '{Entity} not found' }))
      res.json({ success: true, message: '{Entity} deleted' })
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to delete'
      res.status(400).json(buildErrorResponse(req, { code: 'BAD_REQUEST', message: msg, details: err }))
    }
  }
}
export const {entities}Controller = new {Entities}Controller()
```

## Example

```ts
// apps/api-internal/src/controllers/organizations.controller.ts
export class OrganizationsController {
  async create(req: Request, res: Response) {
    try {
      const org = await organizationService.create(req.body as CreateOrganizationInput, req.userContext as UserContext)
      res.status(201).json({ success: true, data: org })
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'Failed to create organization'
      const isConflict = msg.includes('unique') || msg.includes('uq_')
      res.status(isConflict ? 409 : 400).json(buildErrorResponse(req, {
        code: isConflict ? 'CONFLICT' : 'BAD_REQUEST', message: msg, details: err }))
    }
  }
}
export const organizationsController = new OrganizationsController()
```

## Naming

- File: `apps/{stack}/src/controllers/{entities}.controller.ts` (plural)
- Class: `{Entities}Controller` -- Export: `export const {entities}Controller = new ...`
- Methods: `list`, `get`, `create`, `update`, `delete`

## Rules

1. **Class with singleton.** `export class` + `export const instance = new Class()`.
2. **Signature: `async (req: Request, res: Response)`.** No return type needed.
3. **try/catch every method.** Never let exceptions bubble unhandled.
4. **buildErrorResponse for errors.** `buildErrorResponse(req, { code, message, details })`.
5. **Success: `{ success: true, data }`.** Created: `res.status(201)`.
6. **Pagination from middleware.** `req.pagination` via `parsePagination`; return via `buildPaginatedResponse()`.
7. **UserContext from middleware.** `req.userContext` via `extractUserContext`; pass to service mutations.
8. **Status mapping:** 404 not found, 409 conflict/invalid transition, 400 bad input, 500 internal.
9. **No business logic.** Controllers map HTTP to service. Validation lives in the service.
10. **Error message sniffing for status.** Check `msg.includes(...)` to pick 409 vs 400.
