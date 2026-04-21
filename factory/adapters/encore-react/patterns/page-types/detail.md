# Detail Page Pattern

## Convention
Detail pages load a single record by ID from the URL params. Use loader for initial fetch, useQuery for live updates. Show metadata, status, and related data.

## Template
```tsx
import { useLoaderData, useParams } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { getClient } from "~/lib/encore.server";

export async function loader({ request, params }: { request: Request; params: { id: string } }) {
  const { client } = await requireUser(request);
  const item = await client.{service}.get(params.id);
  return { item };
}

export default function {Entity}Detail() {
  const { item: initial } = useLoaderData<typeof loader>();
  const { id } = useParams();
  const client = getClient();

  const { data: item } = useQuery({
    queryKey: ["{resource}", id],
    queryFn: () => client.{service}.get(id!),
    initialData: initial,
  });

  return (
    <div className="max-w-4xl mx-auto p-8">
      <a href="/{resource}" className="text-indigo-600 hover:underline">Back</a>

      <h1 className="text-2xl font-bold mt-4">{item.{titleField}}</h1>
      <span className="px-2 py-1 rounded text-sm bg-{color}-100 text-{color}-800">
        {item.status}
      </span>

      <dl className="mt-6 grid grid-cols-2 gap-4">
        <div><dt className="text-sm text-gray-500">{Label}</dt><dd>{item.{field}}</dd></div>
      </dl>
    </div>
  );
}
```

## Rules
1. Load initial data in loader (SSR), then use `useQuery` with `initialData` for live updates
2. Back link at top
3. Status badge next to title
4. Metadata in definition list grid
