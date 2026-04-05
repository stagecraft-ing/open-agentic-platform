# Dashboard Page Pattern (React Router + TanStack Query)

## Convention
Dashboards use TanStack `useQuery` for auto-refreshing data, `useMutation` for actions, and Tailwind for layout. Data flows: loader (auth check) → component → useQuery (live data).

## Template
```tsx
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { getClient } from "~/lib/encore.server";

export async function loader({ request }: { request: Request }) {
  const { user } = await requireUser(request);
  return { user };
}

export default function Dashboard() {
  const client = getClient();
  const queryClient = useQueryClient();

  const { isLoading, error, data } = useQuery({
    queryKey: ["{resource}"],
    queryFn: () => client.{service}.list(),
    refetchInterval: 10000,
  });

  const doDelete = useMutation({
    mutationFn: (id: string) => client.{service}.del(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["{resource}"] }),
  });

  if (isLoading) return <div className="p-8 text-gray-500">Loading...</div>;
  if (error) return <div className="p-8 text-red-600">Error: {error.message}</div>;

  return (
    <div className="max-w-4xl mx-auto p-8">
      <h1 className="text-2xl font-bold mb-6">{Title}</h1>
      {/* Metric cards, tables, action buttons */}
    </div>
  );
}
```

## Rules
1. Use `loader` for auth checks, `useQuery` for live data
2. `refetchInterval` for auto-refresh (10-30s typical)
3. Invalidate queries after mutations
4. Loading/error states before content
5. Tailwind utility classes, no external component library
