# Route Pattern

## Convention

Each entity gets two files: a routes file (Express Router with middleware) and a
route plugin (mounts the router on the app). Routes contain zero logic -- they wire
middleware and delegate to controller methods.

## Template

### Routes file

```ts
// apps/{stack}/src/routes/{entities}.routes.ts

import express from 'express'
import { {entities}Controller } from '../controllers/{entities}.controller.js'
import { requireSessionOrServiceAuth } from '../middleware/service-auth.middleware.js'
import { extractUserContext, requireUserContext } from '../middleware/user-context.middleware.js'
import { parsePagination } from '../middleware/pagination.middleware.js'

const router = express.Router()

router.use(requireSessionOrServiceAuth, extractUserContext)

router.get('/', parsePagination, (req, res) => {entities}Controller.list(req, res))
router.post('/', requireUserContext, (req, res) => {entities}Controller.create(req, res))
router.get('/:id', (req, res) => {entities}Controller.get(req, res))
router.patch('/:id', requireUserContext, (req, res) => {entities}Controller.update(req, res))
router.delete('/:id', requireUserContext, (req, res) => {entities}Controller.delete(req, res))

export default router
```

### Route plugin file

```ts
// apps/{stack}/src/routes/{entities}.routes.plugin.ts

import type { Express } from 'express'
import router from './{entities}.routes.js'

export function register{Entity}Routes(app: Express): void {
  app.use('/api/v1/{entities}', router)
}
```

### Registration in modules.ts

```ts
// In apps/{stack}/src/modules.ts -- add to registerAllModules()

import { register{Entity}Routes } from './routes/{entities}.routes.plugin.js'

// Inside registerAllModules(app):
register{Entity}Routes(app) // /api/v1/{entities}
```

## Example

```ts
// apps/api-internal/src/routes/organizations.routes.ts

import express from 'express'
import { organizationsController } from '../controllers/organizations.controller.js'
import { requireSessionOrServiceAuth } from '../middleware/service-auth.middleware.js'
import { extractUserContext, requireUserContext } from '../middleware/user-context.middleware.js'
import { parsePagination } from '../middleware/pagination.middleware.js'

const router = express.Router()

router.use(requireSessionOrServiceAuth, extractUserContext)

router.get('/', parsePagination, (req, res) => organizationsController.list(req, res))
router.post('/', requireUserContext, (req, res) => organizationsController.create(req, res))

router.get('/:id', (req, res) => organizationsController.get(req, res))
router.patch('/:id', requireUserContext, (req, res) => organizationsController.update(req, res))

export default router
```

```ts
// apps/api-internal/src/routes/organizations.routes.plugin.ts

import type { Express } from 'express'
import router from './organizations.routes.js'

export function registerOrganizationRoutes(app: Express): void {
  app.use('/api/v1/organizations', router)
}
```

## Naming

- Routes file: `apps/{stack}/src/routes/{entities}.routes.ts` (plural)
- Plugin file: `apps/{stack}/src/routes/{entities}.routes.plugin.ts`
- Plugin function: `register{Entity}Routes` (singular Entity, plural Routes)
- URL path: `/api/v1/{entities}` (plural, kebab-case)

## Rules

1. **Two files per entity.** The routes file defines the router; the plugin mounts it.
2. **router.use for shared middleware.** `requireSessionOrServiceAuth` and `extractUserContext` apply to all routes in the router via `router.use()`.
3. **parsePagination on GET list endpoints only.** Adds `req.pagination` with `page` and `limit`.
4. **requireUserContext on mutation endpoints.** POST, PATCH, PUT, DELETE require an authenticated user context.
5. **Thin delegation.** Route handlers are arrow functions: `(req, res) => controller.method(req, res)`. No inline logic.
6. **PATCH for partial updates.** Not PUT. PUT is reserved for full-resource replacement (e.g., upsert contact).
7. **Plugin registration.** Every plugin is imported and called in `modules.ts` inside `registerAllModules()`.
8. **Versioned URL prefix.** Always `/api/v1/{entities}`.
9. **No route-level validation.** Validation lives in the service or Zod schemas.
10. **Sub-resource routes.** Nested entities mount on parent: `router.post('/:id/transitions', ...)`.
