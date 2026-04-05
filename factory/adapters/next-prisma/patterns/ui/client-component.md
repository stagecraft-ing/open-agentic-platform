# Client Component Pattern

Client Components handle interactivity: forms, event handlers, browser APIs,
and client-side state. They are marked with `"use client"` at the top.

## Convention

- File: `src/components/{Name}.client.tsx`
- First line: `"use client";`
- Use React hooks for local state (`useState`, `useTransition`)
- Call Server Actions for mutations — not API routes
- Receive server data via props — don't re-fetch

## Template

```tsx
"use client";

import { useState, useTransition } from "react";
import { create{Entity} } from "@/app/(app)/{resource}/actions";

interface {Component}Props {
  initialData?: {Entity}[];
}

export function {Component}({ initialData }: {Component}Props) {
  const [isPending, startTransition] = useTransition();
  const [{field}, set{Field}] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);

    startTransition(async () => {
      const result = await create{Entity}({ {field} });
      if (result?.error) {
        setError(result.error);
      } else {
        set{Field}("");
      }
    });
  }

  return (
    <form onSubmit={handleSubmit} className="mt-6 space-y-4">
      {error && (
        <p className="text-sm text-red-600 dark:text-red-400">{error}</p>
      )}
      <div className="flex gap-4">
        <input
          type="text"
          value={{field}}
          onChange={(e) => set{Field}(e.target.value)}
          placeholder="{Placeholder}"
          className="flex-1 rounded-md border border-gray-300 px-3 py-2
                     dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
          disabled={isPending}
        />
        <button
          type="submit"
          disabled={isPending || !{field}.trim()}
          className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white
                     hover:bg-indigo-700 disabled:opacity-50"
        >
          {isPending ? "Saving..." : "Save"}
        </button>
      </div>
    </form>
  );
}
```

## Example

From `src/components/AddSiteForm.client.tsx`:

```tsx
"use client";

import { useState, useTransition } from "react";
import { addSite } from "@/app/(app)/sites/actions";

export function AddSiteForm() {
  const [isPending, startTransition] = useTransition();
  const [url, setUrl] = useState("");

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    startTransition(async () => {
      await addSite({ url });
      setUrl("");
    });
  }

  return (
    <form onSubmit={handleSubmit} className="flex gap-4">
      <input type="url" value={url} onChange={(e) => setUrl(e.target.value)}
        placeholder="https://example.com" className="flex-1 rounded-md border px-3 py-2" />
      <button type="submit" disabled={isPending || !url.trim()}
        className="rounded-md bg-indigo-600 px-4 py-2 text-white">
        {isPending ? "Adding..." : "Add Site"}
      </button>
    </form>
  );
}
```

## Rules

1. `"use client"` must be the very first line — before any imports.
2. Call Server Actions for mutations — never fetch to `/api/` routes from client.
3. Use `useTransition` for pending states — not manual loading booleans.
4. Receive server data as props — don't duplicate data fetching.
5. Keep Client Components small — extract interactivity, keep rendering in Server Components.
6. Tailwind CSS for all styling — include `dark:` variants.
