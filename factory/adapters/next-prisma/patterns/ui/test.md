# Page Test Pattern

Component tests for Next.js pages use Vitest with React Testing Library.
Server Components are tested by mocking Prisma and rendering the component.

## Convention

- Test file: `src/app/(app)/{resource}/__tests__/page.test.tsx`
- Mock Prisma at module level
- Mock NextAuth session
- Test rendered output, not implementation details

## Template

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import Page from "../page";

vi.mock("@/lib/db", () => ({
  prisma: {
    {entity}: {
      findMany: vi.fn(() => [
        { id: "1", {field}: "Test Item", createdAt: new Date(), updatedAt: new Date() },
      ]),
      count: vi.fn(() => 1),
    },
  },
}));

vi.mock("next-auth", () => ({
  getServerSession: vi.fn(() => ({
    user: { id: "user-1", email: "test@example.com" },
  })),
}));

vi.mock("next/navigation", () => ({
  redirect: vi.fn(),
}));

describe("{PageName} Page", () => {
  it("renders {items} list", async () => {
    const Component = await Page();
    render(Component);
    expect(screen.getByText("Test Item")).toBeInTheDocument();
  });

  it("shows empty state when no {items}", async () => {
    vi.mocked((await import("@/lib/db")).prisma.{entity}.findMany).mockResolvedValue([]);
    const Component = await Page();
    render(Component);
    expect(screen.getByText(/no {items}/i)).toBeInTheDocument();
  });
});
```

## Rules

1. Mock `prisma` at module level — never hit a real database.
2. For async Server Components, `await Page()` then `render()` the result.
3. Mock `next/navigation` to capture `redirect()` calls.
4. Test what the user sees — not implementation details.
5. Test empty states explicitly.
