# Form Page Pattern (React Router Actions)

## Convention
Forms use React Router's `<Form>` component with an `action` function for server-side processing. Validation and submission happen server-side.

## Template
```tsx
import { Form, redirect, useActionData } from "react-router";
import { getClient } from "~/lib/encore.server";

export async function loader({ request }: { request: Request }) {
  await requireUser(request);
  return {};
}

export async function action({ request }: { request: Request }) {
  const { user, client } = await requireUser(request);
  const formData = await request.formData();

  const input = {
    {field}: formData.get("{field}") as string,
    {numberField}: Number(formData.get("{numberField}")),
  };

  // Validation
  const errors: Record<string, string> = {};
  if (!input.{field}) errors.{field} = "{Field} is required";
  if (Object.keys(errors).length > 0) return { errors };

  try {
    await client.{service}.create(input);
    return redirect("/{resource}");
  } catch (e) {
    return { errors: { form: e instanceof Error ? e.message : "Failed to save" } };
  }
}

export default function {Entity}Form() {
  const actionData = useActionData<typeof action>();

  return (
    <div className="max-w-2xl mx-auto p-8">
      <h1 className="text-2xl font-bold mb-6">New {Entity}</h1>

      {actionData?.errors?.form && (
        <div className="bg-red-50 text-red-700 p-4 rounded mb-4">{actionData.errors.form}</div>
      )}

      <Form method="post" className="space-y-4">
        <div>
          <label className="block text-sm font-medium">{Label}</label>
          <input name="{field}" type="text"
            className="mt-1 block w-full rounded border-gray-300 shadow-sm" />
          {actionData?.errors?.{field} && (
            <p className="text-red-600 text-sm mt-1">{actionData.errors.{field}}</p>
          )}
        </div>

        <button type="submit"
          className="bg-indigo-600 text-white px-4 py-2 rounded hover:bg-indigo-700">
          Save
        </button>
      </Form>
    </div>
  );
}
```

## Rules
1. Use React Router `<Form>` — not fetch() or axios
2. Server-side validation in `action` function
3. Return `{ errors }` on validation failure — component reads via `useActionData`
4. Return `redirect()` on success
5. Field-level error display under each input
6. Auth check in both loader and action
