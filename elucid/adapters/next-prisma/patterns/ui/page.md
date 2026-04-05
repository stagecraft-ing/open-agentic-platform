# Server Component Page Pattern

Pages in Next.js App Router are React Server Components by default. They can
`await` data directly — no `useEffect`, no client-side fetching.

## Convention

- File: `src/app/(app)/{resource}/page.tsx`
- Export a default `async function` — this is the Server Component
- Fetch data with Prisma directly (server-only, no API round-trip)
- Import Client Components for interactive parts
- Tailwind CSS for all styling

## Template

```tsx
import { prisma } from "@/lib/db";
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { redirect } from "next/navigation";
import { {ClientComponent} } from "@/components/{ClientComponent}.client";

export default async function {PageName}Page() {
  const session = await getServerSession(authOptions);
  if (!session) redirect("/auth/signin");

  const {items} = await prisma.{entity}.findMany({
    orderBy: { createdAt: "desc" },
  });

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100">
          {Title}
        </h1>
      </div>

      {/* Server-rendered content */}
      {items.length === 0 ? (
        <p className="text-gray-500 dark:text-gray-400">No {items} yet.</p>
      ) : (
        <div className="space-y-4">
          {{items}.map((item) => (
            <div key={item.id} className="p-4 border rounded-lg dark:border-gray-700">
              {/* render item */}
            </div>
          ))}
        </div>
      )}

      {/* Client Component for interactive parts */}
      <{ClientComponent} />
    </div>
  );
}
```

### Loading state (`loading.tsx`)

```tsx
export default function Loading() {
  return (
    <div className="container mx-auto px-4 py-8">
      <div className="animate-pulse space-y-4">
        <div className="h-8 bg-gray-200 rounded w-1/4 dark:bg-gray-700" />
        <div className="h-32 bg-gray-200 rounded dark:bg-gray-700" />
      </div>
    </div>
  );
}
```

## Example

From `src/app/(app)/dashboard/page.tsx`:

```tsx
import { prisma } from "@/lib/db";
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { redirect } from "next/navigation";

export default async function DashboardPage() {
  const session = await getServerSession(authOptions);
  if (!session) redirect("/auth/signin");

  const sites = await prisma.site.findMany();
  const totalChecks = await prisma.check.count();

  return (
    <div className="container mx-auto px-4 py-8">
      <h1 className="text-2xl font-bold mb-6">Dashboard</h1>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="p-6 border rounded-lg">
          <p className="text-sm text-gray-500">Total Sites</p>
          <p className="text-3xl font-bold">{sites.length}</p>
        </div>
        <div className="p-6 border rounded-lg">
          <p className="text-sm text-gray-500">Total Checks</p>
          <p className="text-3xl font-bold">{totalChecks}</p>
        </div>
      </div>
    </div>
  );
}
```

## Rules

1. Pages are `async function` Server Components — never add `"use client"`.
2. Fetch data with Prisma directly — no `fetch()` to own API routes.
3. Check auth with `getServerSession()` and `redirect()` if unauthorized.
4. Use Tailwind CSS for all styling — include `dark:` variants.
5. Handle empty states explicitly — don't render empty tables.
6. Create a `loading.tsx` sibling for Suspense loading state.
7. Pass server data to Client Components via props — don't re-fetch.
