# Login Page Pattern

## Convention
Sign-in page using React Router `<Form>` with action for server-side credential validation. Redirects authenticated users. Cookie-based session.

## Template
```tsx
import { Form, redirect, useActionData } from "react-router";
import { signin, setSessionCookie } from "~/lib/auth-api.server";

export async function action({ request }: { request: Request }) {
  const formData = await request.formData();
  const email = formData.get("email") as string;
  const password = formData.get("password") as string;

  if (!email || !password) {
    return { error: "Email and password are required" };
  }

  try {
    const result = await signin(email, password);
    const headers = new Headers();
    setSessionCookie(headers, result.token);
    return redirect("/app", { headers });
  } catch {
    return { error: "Invalid email or password" };
  }
}

export default function SignIn() {
  const actionData = useActionData<typeof action>();

  return (
    <div className="min-h-screen flex items-center justify-center">
      <div className="max-w-md w-full space-y-8 p-8">
        <h2 className="text-3xl font-bold text-center">Sign In</h2>

        {actionData?.error && (
          <div className="bg-red-50 text-red-700 p-3 rounded">{actionData.error}</div>
        )}

        <Form method="post" className="space-y-4">
          <input name="email" type="email" placeholder="Email"
            className="block w-full rounded border-gray-300 p-2" required />
          <input name="password" type="password" placeholder="Password"
            className="block w-full rounded border-gray-300 p-2" required />
          <button type="submit"
            className="w-full bg-indigo-600 text-white py-2 rounded hover:bg-indigo-700">
            Sign In
          </button>
        </Form>
      </div>
    </div>
  );
}
```

## Rules
1. Server-side action handles auth — never expose credentials to client
2. Set session cookie via response headers on success
3. Redirect to `/app` on success
4. Display error message on failure without revealing specifics
