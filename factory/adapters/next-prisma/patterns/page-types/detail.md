# Detail Page Pattern

Single record view with related data. Server Component fetches by ID
from the dynamic route segment.

## Template

```tsx
import { prisma } from "@/lib/db";
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { redirect, notFound } from "next/navigation";
import Link from "next/link";

interface Props {
  params: Promise<{ id: string }>;
}

export default async function {Entity}DetailPage({ params }: Props) {
  const session = await getServerSession(authOptions);
  if (!session) redirect("/auth/signin");

  const { id } = await params;
  const item = await prisma.{entity}.findUnique({
    where: { id },
    include: { {relation}: true },
  });

  if (!item) notFound();

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <h1 className="text-2xl font-bold">{item.{field}}</h1>
        <Link href={`/{resource}/${item.id}/edit`}
          className="rounded-md bg-indigo-600 px-4 py-2 text-sm text-white">Edit</Link>
      </div>

      <dl className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div className="border rounded-lg p-4">
          <dt className="text-sm font-medium text-gray-500">Status</dt>
          <dd className="mt-1 text-sm">{item.status}</dd>
        </div>
        <div className="border rounded-lg p-4">
          <dt className="text-sm font-medium text-gray-500">Created</dt>
          <dd className="mt-1 text-sm">{item.createdAt.toLocaleDateString()}</dd>
        </div>
      </dl>
    </div>
  );
}
```

## Rules

1. Use `params.id` from the dynamic route `[id]` segment.
2. Call `notFound()` if the record doesn't exist — renders `not-found.tsx`.
3. Use `include` to load related data in one query.
4. Link to edit page for mutation.
5. Use `<dl>` with definition list for field/value pairs.
