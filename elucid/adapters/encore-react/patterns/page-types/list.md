# List Page Pattern

A page displaying a collection in a table with polling, inline add form, and delete. Based on `app._index.tsx`. Uses `useQuery` with `refetchInterval` for live data, `useMutation` + `invalidateQueries` for writes, explicit loading/error/empty states, and a browser-side Encore `Client`.

## Template

```tsx
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import Client from "../lib/client";
import { type FC, useEffect, useState } from "react";

export default function {PageName}() {
  const [baseURL, setBaseURL] = useState("");
  useEffect(() => setBaseURL(window.location.origin), []);
  if (!baseURL) return null;
  return <{ListComponent} client={new Client(baseURL)} />;
}

const {ListComponent}: FC<{ client: Client }> = ({ client }) => {
  const { isLoading, error, data } = useQuery({
    queryKey: ["{items}"],
    queryFn: () => client.{service}.list(),
    refetchInterval: {pollMs},
    retry: false,
  });

  const queryClient = useQueryClient();
  const doDelete = useMutation({
    mutationFn: (item: {ItemType}) => client.{service}.del(item.id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["{items}"] }),
  });

  if (isLoading) return <div>Loading...</div>;
  if (error) {
    return <div className="text-red-600 dark:text-red-400">{(error as Error).message}</div>;
  }

  return (
    <>
      <div className="sm:flex sm:items-center">
        <h4 className="text-base font-semibold text-gray-900 dark:text-gray-100 sm:flex-auto">
          {Title}
        </h4>
        <Add{Item}Form client={client} />
      </div>
      <table className="mt-8 min-w-full divide-y divide-gray-300 dark:divide-gray-600">
        <thead className="bg-gray-50 dark:bg-gray-800">
          <tr>
            <th className="px-3 py-3.5 text-left text-sm font-semibold">Name</th>
            <th className="relative py-3.5 pl-3 pr-4 sm:pr-6"><span className="sr-only">Actions</span></th>
          </tr>
        </thead>
        <tbody className="divide-y divide-gray-200 bg-white dark:divide-gray-700 dark:bg-gray-900">
          {data?.{items}.length === 0 && (
            <tr><td colSpan={2} className="text-center text-gray-400 py-8">No {items} yet.</td></tr>
          )}
          {data!.{items}.map((item) => (
            <tr key={item.id}>
              <td className="px-3 py-4 text-sm text-gray-700 dark:text-gray-300">{item.name}</td>
              <td className="whitespace-nowrap py-4 pl-3 pr-4 text-right text-sm sm:pr-6">
                <button className="text-indigo-600 hover:text-indigo-900 dark:text-indigo-400"
                  onClick={() => doDelete.mutate(item)}>Delete</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
};

// --- Inline Add Form (toggles open/closed with local state) ---

const Add{Item}Form: FC<{ client: Client }> = ({ client }) => {
  const [open, setOpen] = useState(false);
  const [value, setValue] = useState("");
  const queryClient = useQueryClient();

  const save = useMutation({
    mutationFn: async (val: string) => {
      await client.{service}.add({ {field}: val });
      setOpen(false);
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["{items}"] }),
  });

  if (!open) {
    return (
      <button className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700"
        onClick={() => setOpen(true)}>Add {item}</button>
    );
  }

  return (
    <form onSubmit={(e) => { e.preventDefault(); save.mutate(value); }}>
      <div className="flex items-end gap-4">
        <input type="text" value={value} onChange={(e) => setValue(e.target.value)}
          className="block w-full rounded-md border-gray-300 p-2 border shadow-sm dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100" />
        <button type="submit" disabled={!value.trim()}
          className="rounded-md bg-indigo-600 py-2 px-4 text-sm text-white hover:bg-indigo-700 disabled:opacity-75">
          Save
        </button>
      </div>
    </form>
  );
};
```

## Rules

1. Guard `Client` instantiation behind a `baseURL` state check to avoid SSR errors.
2. Set `retry: false` on queries that should surface errors immediately.
3. After any mutation, invalidate all related query keys.
4. Handle three states explicitly: loading, error, empty list.
5. The add form toggles visibility with local state; it does not use a modal.
6. Table uses Tailwind's `divide-y` pattern with `dark:` variants throughout.
