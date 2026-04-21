# Middleware Pattern

Express middleware for cross-cutting concerns. Each middleware is a standalone file
with a corresponding plugin file that handles registration.

## Convention

- Middleware: `apps/{stack}/src/middleware/{name}.middleware.ts`
- Plugin (registration): `apps/{stack}/src/middleware/{name}.plugin.ts`
- Registry: `apps/{stack}/src/middleware/registry.ts`

## Template — Middleware Function

```typescript
import type { Request, Response, NextFunction } from 'express';

export function {middlewareName}(req: Request, res: Response, next: NextFunction): void {
  // Guard/transform logic
  next();
}
```

## Template — Factory Middleware (parameterised)

```typescript
import type { Request, Response, NextFunction, RequestHandler } from 'express';

export function {middlewareName}(options: {OptionsType}): RequestHandler {
  return (req: Request, res: Response, next: NextFunction) => {
    // Guard/transform logic using options
    next();
  };
}
```

## Template — Plugin (Registration)

```typescript
import type { Express } from 'express';
import { {middlewareName} } from './{name}.middleware.js';

export function register(app: Express): void {
  app.use({middlewareName}());
}
```

## Known Middleware

- `requireAuth()` — blocks unauthenticated requests; checks `req.session.user`
- `requireRole(role)` — factory; blocks if `req.session.user.role !== role`
- `requireSessionOrServiceAuth` — accepts session OR `x-service-token` header (for BFF proxy)
- `extractUserContext` — attaches `req.userContext` from session data
- `requireUserContext` — blocks if `req.userContext` is absent
- `parsePagination()` — attaches `req.pagination` with `page`/`limit` from query params
- `validate(schema, target)` — factory; validates `req[target]` against Zod schema

## Rules

1. One concern per middleware file. Combine via plugin registration order.
2. Middleware that needs configuration uses the factory pattern (returns `RequestHandler`).
3. Auth middleware reads from `req.session` — never from headers (except service-to-service token).
4. Validation middleware uses Zod schemas from `packages/shared/src/schemas/`.
5. Error responses use `buildErrorResponse()` from `utils/error-response.ts`.
6. Plugin priority determines execution order — lower priority numbers run first.
7. All middleware is registered in `apps/{stack}/src/modules.ts` via the module system.
