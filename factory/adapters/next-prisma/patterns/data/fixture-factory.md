# Fixture Factory Pattern

In-memory factory functions for test data. Zero external dependencies beyond Prisma types.

## Convention

- Single file: `src/lib/fixtures/index.ts`
- One function per entity: `createSample{Entity}(overrides?)`
- Returns a complete Prisma `{Entity}` type with realistic defaults
- Defaults match the first dev-fixture row in `prisma/seed.ts` (consistency)

## Template

```ts
import type { {Entity} } from '@prisma/client';

export function createSample{Entity}(
  overrides?: Partial<{Entity}>
): {Entity} {
  return {
    id: '{deterministic_test_id}',
    // ... all fields with realistic default values ...
    createdAt: new Date('2026-01-15T10:00:00.000Z'),
    updatedAt: new Date('2026-01-15T10:00:00.000Z'),
    ...overrides,
  };
}
```

## Rules

1. Import types from `@prisma/client` — no other external dependencies.
2. Every field present — no optional fields omitted.
3. FK values reference other factory defaults (e.g., `organizationId: 'test-org-001'`).
4. User ID fields reference mock auth driver IDs (e.g., `submittedBy: 'mock-applicant-1'`).
5. Deterministic values only — no `Date.now()`, no `crypto.randomUUID()`.
6. Export all functions from a single barrel `index.ts`.
