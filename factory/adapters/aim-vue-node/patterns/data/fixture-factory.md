# Fixture Factory Pattern

In-memory factory functions for test data. Zero external dependencies.

## Convention

- Single file: `packages/shared/src/fixtures/index.ts`
- One function per entity: `createSample{Entity}(overrides?)`
- Returns a complete `{Entity}Row` with realistic defaults
- Defaults match the first dev-fixture SQL row (consistency)

## Template

```ts
import type { {Entity}Row } from '../types/{entity}.types.js';

export function createSample{Entity}(
  overrides?: Partial<{Entity}Row>
): {Entity}Row {
  return {
    {pk_field}: '{deterministic_test_id}',
    // ... all fields with realistic default values ...
    created_at: '2026-01-15T10:00:00.000Z',
    updated_at: '2026-01-15T10:00:00.000Z',
    ...overrides,
  };
}
```

## Rules

1. Import only from `../types/` — no external dependencies.
2. Every field present — no optional fields omitted.
3. FK values reference other factory defaults (e.g., `organization_id: 'test-org-001'`).
4. User ID fields reference mock auth driver IDs (e.g., `submitted_by: 'mock-applicant-1'`).
5. Deterministic values only — no `Date.now()`, no `crypto.randomUUID()`.
6. Export all functions from a single barrel `index.ts`.
