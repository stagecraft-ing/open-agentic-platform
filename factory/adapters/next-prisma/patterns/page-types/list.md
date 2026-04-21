# List Page Pattern

Data table with server-side pagination, filtering, and search. Server Component
reads URL search params for pagination state.

## Template

```tsx
import { prisma } from "@/lib/db";
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { redirect } from "next/navigation";
import Link from "next/link";

interface Props {
  searchParams: Promise<{ page?: string; q?: string }>;
}

export default async function {Entity}ListPage({ searchParams }: Props) {
  const session = await getServerSession(authOptions);
  if (!session) redirect("/auth/signin");

  const { page: pageStr, q } = await searchParams;
  const page = Number(pageStr) || 1;
  const pageSize = 20;

  const where = q ? { {field}: { contains: q, mode: "insensitive" as const } } : {};

  const [items, total] = await Promise.all([
    prisma.{entity}.findMany({
      where,
      orderBy: { createdAt: "desc" },
      skip: (page - 1) * pageSize,
      take: pageSize,
    }),
    prisma.{entity}.count({ where }),
  ]);

  const totalPages = Math.ceil(total / pageSize);

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold">{Entities}</h1>
        <Link href="/{resource}/new"
          className="rounded-md bg-indigo-600 px-4 py-2 text-sm text-white hover:bg-indigo-700">
          Add {Entity}
        </Link>
      </div>

      <table className="min-w-full divide-y divide-gray-200">
        <thead className="bg-gray-50 dark:bg-gray-800">
          <tr>
            <th className="px-4 py-3 text-left text-sm font-semibold">{Field}</th>
            <th className="px-4 py-3 text-left text-sm font-semibold">Created</th>
            <th className="relative px-4 py-3"><span className="sr-only">Actions</span></th>
          </tr>
        </thead>
        <tbody className="divide-y bg-white dark:bg-gray-900">
          {items.length === 0 ? (
            <tr><td colSpan={3} className="px-4 py-8 text-center text-gray-500">No {entities} found.</td></tr>
          ) : (
            items.map((item) => (
              <tr key={item.id}>
                <td className="px-4 py-3 text-sm">{item.{field}}</td>
                <td className="px-4 py-3 text-sm text-gray-500">{item.createdAt.toLocaleDateString()}</td>
                <td className="px-4 py-3 text-right text-sm">
                  <Link href={`/{resource}/${item.id}`} className="text-indigo-600 hover:text-indigo-800">View</Link>
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>

      {totalPages > 1 && (
        <div className="flex gap-2">
          {Array.from({ length: totalPages }, (_, i) => (
            <Link key={i} href={`/{resource}?page=${i + 1}`}
              className={`px-3 py-1 rounded ${page === i + 1 ? "bg-indigo-600 text-white" : "border"}`}>
              {i + 1}
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
```

## Rules

1. Read pagination from `searchParams` — no client-side state.
2. Use `Promise.all` for parallel count + data fetch.
3. Handle empty state in the table body.
4. Pagination links use `<Link>` with query params.
5. Use `skip`/`take` on Prisma — never load unbounded result sets.
