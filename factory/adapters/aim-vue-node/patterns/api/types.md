# Shared Types Pattern

TypeScript interfaces shared between API stacks and the frontend. Defines the
camelCase API response shapes distinct from database row types.

## Convention

- Types file: `packages/shared/src/types/{entity}.types.ts`
- Re-export from: `packages/shared/src/index.ts`

## Template

```typescript
// packages/shared/src/types/{entity}.types.ts

/** API response shape for {Entity} (camelCase, frontend-friendly). */
export interface {Entity} {
  {fieldName}: {type};
  // ... all fields from Build Spec, camelCase
}

/** Input for creating a new {Entity}. Omits server-generated fields. */
export interface Create{Entity}Input {
  {fieldName}: {type};
  // ... writable fields only (no id, createdAt, updatedAt)
}

/** Input for updating an existing {Entity}. All fields optional. */
export interface Update{Entity}Input {
  {fieldName}?: {type};
  // ... updatable fields only
}

/** Database row shape (snake_case, as returned by pg driver). */
export interface {Entity}Row {
  {field_name}: {type};
  // ... all columns, snake_case
}
```

## Type Mapping

| Build Spec type | TypeScript type |
|---|---|
| string | string |
| integer | number |
| boolean | boolean |
| uuid | string |
| timestamp | string (ISO 8601) |
| date | string (YYYY-MM-DD) |
| decimal | string (pg returns NUMERIC as string) |
| text | string |
| reference | string (the FK value) |

## Rules

1. API response interfaces use camelCase field names.
2. Database row interfaces use snake_case field names matching DDL columns exactly.
3. Service layer maps between Row and API types.
4. All types re-exported from `packages/shared/src/index.ts`.
5. No runtime validation in type files — that belongs in schemas.
6. Reference fields become `string` (the UUID/ID value), not the full related object.
7. **No local type divergence.** Service files MUST import types from `packages/shared` — they MUST NOT define local interfaces with property names that differ from the shared type. A service-local `status` field when the shared type defines `application_status` causes SQL column name mismatches that compile but fail at runtime.
8. **Row type field names MUST match DDL column names.** If the DDL defines `application_status VARCHAR(50)`, the Row type must use `application_status` — not `status`, not `appStatus`. This is the single most common source of runtime errors.
9. **Enum/union values MUST match DDL CHECK constraints.** If the DDL defines `CHECK (status IN ('draft', 'submitted'))`, the corresponding TypeScript union or Zod enum must contain exactly `'draft' | 'submitted'` — no more, no less.
