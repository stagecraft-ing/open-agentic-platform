# View Component Pattern

Route files export a default function component, optional `loader`/`action` for server-side work, and use TanStack Query for client-side data fetching.

## Convention

- File lives in `web/app/routes/` with dot-notation naming (e.g., `app._index.tsx`).
- Server-side: `loader` fetches data before render; `action` handles form POST mutations.
- Client-side: `useQuery` for polling/fetching; `useMutation` for writes.
- Styling uses Tailwind CSS utility classes with dark-mode variants.

## Template

```tsx
import { useLoaderData, Form, redirect } from "react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { requireUser } from "../lib/auth.server";
import { createEncoreClient } from "../lib/encore.server";
import Client from "../lib/client";
import { useEffect, useState } from "react";

// --- Server-side (runs on Node, has access to request) ---

export async function loader({ request }: { request: Request }) {
  const user = await requireUser(request);
  const client = createEncoreClient(request);
  const data = await client.{service}.{method}();
  return { user, {items}: data.{items} };
}

export async function action({ request }: { request: Request }) {
  await requireUser(request);
  const formData = await request.formData();
  const client = createEncoreClient(request);
  await client.{service}.{mutationMethod}({ /* fields from formData */ });
  return redirect("/{currentRoute}");
}

// --- Client-side component ---

export default function {ViewName}() {
  const { {items} } = useLoaderData() as { {items}: {ItemType}[] };

  // Client-side polling (when real-time data is needed)
  const [baseURL, setBaseURL] = useState("");
  useEffect(() => setBaseURL(window.location.origin), []);
  const client = baseURL ? new Client(baseURL) : null;

  const { data: liveData } = useQuery({
    queryKey: ["{queryKey}"],
    queryFn: () => client!.{service}.{method}(),
    refetchInterval: {intervalMs},
    enabled: !!client,
  });

  const queryClient = useQueryClient();
  const doAction = useMutation({
    mutationFn: (item: {ItemType}) => client!.{service}.{mutationMethod}(item.id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["{queryKey}"] }),
  });

  return (
    <div className="min-h-full container px-4 mx-auto my-8">
      <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-4">
        {Title}
      </h3>
      {/* Render items with Tailwind classes */}
    </div>
  );
}
```

## Example (from `app._index.tsx`)

The dashboard uses `useQuery` with `refetchInterval: 10000` to poll the site list, a second query polling status every 1 second, and `useMutation` for delete. No server-side `loader` -- all data flows through TanStack Query with a browser-instantiated `Client`.

## Rules

1. `loader`/`action` run server-side only. Import `.server.ts` files only there.
2. The browser `Client` needs `window.location.origin`; guard with a `useState`/`useEffect` pair.
3. After a mutation, call `queryClient.invalidateQueries` to refresh related queries.
4. Use `useLoaderData()` with an `as` type assertion to type loader return values.
5. Tailwind classes always include `dark:` variants for dark-mode support.
6. `action` functions handle `<Form method="post">` submissions and typically `redirect` after success.
7. Return `{ error: "message" }` from `action` for validation errors; read with `useActionData()`.
