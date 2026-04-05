# Dashboard Page Pattern

Overview page with summary metrics and quick-action cards. Server Component
fetches aggregate data directly from Prisma.

## Template

```tsx
import { prisma } from "@/lib/db";
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { redirect } from "next/navigation";

export default async function DashboardPage() {
  const session = await getServerSession(authOptions);
  if (!session) redirect("/auth/signin");

  const [{entity}Count, recent{Entities}] = await Promise.all([
    prisma.{entity}.count(),
    prisma.{entity}.findMany({ orderBy: { createdAt: "desc" }, take: 5 }),
  ]);

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Dashboard</h1>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="p-6 border rounded-lg bg-white dark:bg-gray-800">
          <p className="text-sm text-gray-500">Total {Entities}</p>
          <p className="text-3xl font-bold">{`{${entity}Count}`}</p>
        </div>
      </div>
      <div>
        <h2 className="text-lg font-semibold mb-3">Recent Activity</h2>
        {recent{Entities}.length === 0 ? (
          <p className="text-gray-500">No activity yet.</p>
        ) : (
          <ul className="divide-y">
            {recent{Entities}.map((item) => (
              <li key={item.id} className="py-3">{/* render item */}</li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
```

## Rules

1. Use `Promise.all()` for parallel data fetching.
2. Show aggregate counts in metric cards.
3. Show recent activity list (limit to 5-10 items).
4. Handle empty states for each section.
