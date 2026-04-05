# Form Page Pattern

Create/edit form using a Server Action for mutation. The page is a Server Component;
the form itself is a Client Component for interactivity.

## Template

### Server Action (`actions.ts`)

```ts
"use server";

import { prisma } from "@/lib/db";
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import { Create{Entity}Schema } from "@/lib/types/{entity}";

export async function create{Entity}(formData: FormData) {
  const session = await getServerSession(authOptions);
  if (!session) throw new Error("Unauthorized");

  const parsed = Create{Entity}Schema.safeParse({
    {field}: formData.get("{field}"),
  });

  if (!parsed.success) {
    return { error: parsed.error.flatten().fieldErrors };
  }

  await prisma.{entity}.create({ data: parsed.data });
  revalidatePath("/{resource}");
  redirect("/{resource}");
}
```

### Page (Server Component)

```tsx
import { {Entity}Form } from "@/components/{Entity}Form.client";
import { create{Entity} } from "./actions";

export default function New{Entity}Page() {
  return (
    <div className="max-w-2xl mx-auto">
      <h1 className="text-2xl font-bold mb-6">Create {Entity}</h1>
      <{Entity}Form action={create{Entity}} />
    </div>
  );
}
```

### Client Component form

```tsx
"use client";

import { useActionState } from "react";

interface Props {
  action: (formData: FormData) => Promise<{ error?: Record<string, string[]> } | void>;
}

export function {Entity}Form({ action }: Props) {
  const [state, formAction, isPending] = useActionState(action, null);

  return (
    <form action={formAction} className="space-y-4">
      <div>
        <label className="block text-sm font-medium mb-1">{Field}</label>
        <input type="text" name="{field}" required
          className="w-full rounded-md border px-3 py-2 dark:bg-gray-800 dark:border-gray-600" />
        {state?.error?.{field} && (
          <p className="text-sm text-red-600 mt-1">{state.error.{field}[0]}</p>
        )}
      </div>
      <button type="submit" disabled={isPending}
        className="rounded-md bg-indigo-600 px-4 py-2 text-white disabled:opacity-50">
        {isPending ? "Saving..." : "Create"}
      </button>
    </form>
  );
}
```

## Rules

1. Server Action uses `"use server"` — validates with Zod, writes with Prisma.
2. Call `revalidatePath()` after mutation to refresh cached data.
3. Call `redirect()` after successful mutation.
4. Return field errors from the action — display them inline.
5. Use `useActionState` for pending state and error handling.
6. Pass the action as a prop to the Client Component.
