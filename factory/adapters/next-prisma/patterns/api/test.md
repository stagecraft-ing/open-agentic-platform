# API Route Test Pattern

Tests for Route Handlers use Vitest with mocked Prisma Client and mocked
NextAuth session. Each test creates a mock `Request` and calls the handler directly.

## Convention

- Test file: `src/app/api/{resource}/__tests__/route.test.ts`
- Mock Prisma Client at the module level
- Mock `getServerSession` to control auth state
- Test each HTTP method handler independently

## Template

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { GET, POST } from "../route";
import { prisma } from "@/lib/db";

vi.mock("@/lib/db", () => ({
  prisma: {
    {entity}: {
      findMany: vi.fn(),
      findUnique: vi.fn(),
      create: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
    },
    auditEntry: { create: vi.fn() },
  },
}));

vi.mock("next-auth", () => ({
  getServerSession: vi.fn(() => ({
    user: { id: "user-1", email: "test@example.com" },
  })),
}));

import { createSample{Entity} } from "@/lib/fixtures";

describe("{Resource} API", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("GET /api/{resource}", () => {
    it("returns list of {items}", async () => {
      const mockItems = [createSample{Entity}()];
      vi.mocked(prisma.{entity}.findMany).mockResolvedValue(mockItems);

      const response = await GET();
      const data = await response.json();

      expect(response.status).toBe(200);
      expect(data.{items}).toEqual(mockItems);
    });
  });

  describe("POST /api/{resource}", () => {
    it("creates a new {entity}", async () => {
      const input = { {field}: "new item" };
      const created = { id: "2", ...input };
      vi.mocked(prisma.{entity}.create).mockResolvedValue(created);

      const request = new Request("http://localhost/api/{resource}", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });

      const response = await POST(request);
      const data = await response.json();

      expect(response.status).toBe(201);
      expect(data.id).toBe("2");
    });

    it("returns 400 for invalid input", async () => {
      const request = new Request("http://localhost/api/{resource}", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });

      const response = await POST(request);
      expect(response.status).toBe(400);
    });
  });
});
```

## Rules

1. Mock `prisma` at module level — never hit a real database in unit tests.
2. Mock `getServerSession` to return a session for authenticated tests.
3. Create `Request` objects with proper headers and body for POST/PUT tests.
4. Test both success and validation-failure paths.
5. Use `vi.clearAllMocks()` in `beforeEach` to prevent test pollution.
6. Test auth by mocking `getServerSession` to return `null` for unauthorized tests.
7. **Import sample data from fixture module.** Use `createSample{Entity}()` from `@/lib/fixtures`. Override fields per test with `createSample{Entity}({ field: 'value' })`.
