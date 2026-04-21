# Profile Page Pattern

## Convention
Displays authenticated user info from the loader. Includes sign-out action.

## Template
```tsx
import { Form, useLoaderData, redirect } from "react-router";
import { requireUser } from "~/lib/auth.server";

export async function loader({ request }: { request: Request }) {
  const { user } = await requireUser(request);
  return { user };
}

export async function action({ request }: { request: Request }) {
  // Sign out: clear session cookie
  const headers = new Headers();
  headers.append("Set-Cookie", "__session=; Path=/; Max-Age=0");
  return redirect("/", { headers });
}

export default function Profile() {
  const { user } = useLoaderData<typeof loader>();

  return (
    <div className="max-w-2xl mx-auto p-8">
      <h1 className="text-2xl font-bold mb-6">Profile</h1>
      <dl className="space-y-4">
        <div><dt className="text-sm text-gray-500">Name</dt><dd>{user.name}</dd></div>
        <div><dt className="text-sm text-gray-500">Email</dt><dd>{user.email}</dd></div>
        <div><dt className="text-sm text-gray-500">Role</dt><dd>{user.role}</dd></div>
      </dl>
      <Form method="post" className="mt-8">
        <button type="submit" className="text-red-600 hover:underline">Sign Out</button>
      </Form>
    </div>
  );
}
```

## Rules
1. Auth enforced in loader
2. User data from loader, not a separate API call
3. Sign out via form action (clears cookie)
