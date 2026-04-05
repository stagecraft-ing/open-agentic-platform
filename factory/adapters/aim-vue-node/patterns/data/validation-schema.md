# Validation Schema Pattern

## Convention

All request validation uses Zod. Schemas live in
`packages/shared/src/schemas/{entity}.schema.ts` and are imported by both
API middleware and frontend forms. Naming: `{Entity}CreateSchema`,
`{Entity}UpdateSchema`.

## Type Mapping

| Build Spec Type | Zod Validator                    |
|-----------------|----------------------------------|
| string          | `z.string().min(1)`              |
| string?         | `z.string().optional()`          |
| text            | `z.string().max(10000)`          |
| integer         | `z.number().int()`               |
| decimal         | `z.number().positive()`          |
| boolean         | `z.boolean()`                    |
| uuid            | `z.string().uuid()`              |
| email           | `z.string().email()`             |
| date            | `z.string().date()`              |
| datetime        | `z.string().datetime()`          |
| enum(a,b,c)     | `z.enum(['a', 'b', 'c'])`       |
| url             | `z.string().url()`               |
| phone           | `z.string().regex(/^\+?[\d\s-]+$/)` |

## Template

```typescript
// packages/shared/src/schemas/{entity}.schema.ts
import { z } from 'zod';

export const {Entity}CreateSchema = z.object({
  {field}: {zodValidator},
  // ...
});

export const {Entity}UpdateSchema = {Entity}CreateSchema.partial();

export type {Entity}CreateInput = z.infer<typeof {Entity}CreateSchema>;
export type {Entity}UpdateInput = z.infer<typeof {Entity}UpdateSchema>;
```

**Middleware helper:**

```typescript
// middleware/validate.ts
import { ZodSchema } from 'zod';
import { Request, Response, NextFunction } from 'express';

export function validate(schema: ZodSchema, source: 'body' | 'query') {
  return (req: Request, _res: Response, next: NextFunction) => {
    const result = schema.safeParse(req[source]);
    if (!result.success) {
      return next({ status: 400, errors: result.error.flatten().fieldErrors });
    }
    req[source] = result.data;
    next();
  };
}
```

**Route usage:**

```typescript
router.post('/', validate(FundingRequestCreateSchema, 'body'), controller.create);
router.patch('/:id', validate(FundingRequestUpdateSchema, 'body'), controller.update);
```

## Example

```typescript
import { z } from 'zod';

export const FundingRequestCreateSchema = z.object({
  applicantId: z.string().uuid(),
  programId: z.string().uuid(),
  title: z.string().min(1).max(255),
  amountRequested: z.number().positive(),
  description: z.string().max(10000).optional(),
});

export const FundingRequestUpdateSchema =
  FundingRequestCreateSchema.partial();

export type FundingRequestCreateInput =
  z.infer<typeof FundingRequestCreateSchema>;
export type FundingRequestUpdateInput =
  z.infer<typeof FundingRequestUpdateSchema>;
```

## Rules

1. One schema file per entity in `packages/shared/src/schemas/`.
2. `CreateSchema` has all required fields; `UpdateSchema` uses `.partial()`.
3. Export inferred types alongside schemas (`z.infer<typeof ...>`).
4. Validate in route chain via `validate(schema, 'body')` middleware.
5. Schemas are shared between API and frontend -- never duplicate.
6. Use `.min(1)` on required strings to reject empty strings.
7. Use `.uuid()` for all ID reference fields.
8. Never put business logic in schemas -- only shape and type validation.
