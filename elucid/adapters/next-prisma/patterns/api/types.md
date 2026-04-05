# Types & Validation Pattern

Zod schemas define both runtime validation and TypeScript types. Each entity
gets one types file with schemas for create, update, and response shapes.

## Convention

- One file per entity: `src/lib/types/{entity}.ts`
- Export Zod schemas and inferred TypeScript types
- Schemas match the Prisma model fields
- Create/Update schemas omit auto-generated fields (id, timestamps)

## Template

```ts
import { z } from "zod";

// --- Enum (if entity has status/enum fields) ---
export const {Entity}Status = z.enum(["{value1}", "{value2}", "{value3}"]);
export type {Entity}Status = z.infer<typeof {Entity}Status>;

// --- Create schema ---
export const Create{Entity}Schema = z.object({
  {field}: z.string().min(1, "{Field} is required"),
  {refField}: z.string().uuid("{RefField} must be a valid UUID"),
  {enumField}: {Entity}Status.default("{default}"),
  {optionalField}: z.string().optional(),
});
export type Create{Entity}Input = z.infer<typeof Create{Entity}Schema>;

// --- Update schema (all fields optional) ---
export const Update{Entity}Schema = Create{Entity}Schema.partial();
export type Update{Entity}Input = z.infer<typeof Update{Entity}Schema>;

// --- Response type (matches Prisma model) ---
export interface {Entity} {
  id: string;
  {field}: string;
  {refField}: string;
  status: {Entity}Status;
  createdAt: Date;
  updatedAt: Date;
}
```

## Type Mapping

| Build Spec | Zod | TypeScript |
|-----------|-----|-----------|
| uuid | z.string().uuid() | string |
| string | z.string() | string |
| text | z.string() | string |
| integer | z.number().int() | number |
| decimal | z.string().regex(/^\d+\.\d{2}$/) | string |
| boolean | z.boolean() | boolean |
| date | z.string().date() | string |
| timestamp | z.string().datetime() | string |
| enum | z.enum([...]) | union type |
| reference | z.string().uuid() | string |

## Example

From `src/lib/types/site.ts`:

```ts
import { z } from "zod";

export const CreateSiteSchema = z.object({
  url: z.string().url("Must be a valid URL").max(2048),
});
export type CreateSiteInput = z.infer<typeof CreateSiteSchema>;

export interface Site {
  id: number;
  url: string;
}
```

## Rules

1. Zod is the only validation library — no Joi, Yup, or class-validator.
2. Create schemas validate user input; they omit `id`, `createdAt`, `updatedAt`.
3. Update schemas use `.partial()` on the Create schema.
4. Use `.min(1)` for required string fields, `.uuid()` for reference fields.
5. Export both the Zod schema and the inferred TypeScript type.
6. Response interfaces match the Prisma model shape exactly.
