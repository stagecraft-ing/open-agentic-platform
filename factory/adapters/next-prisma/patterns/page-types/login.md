# Login Page Pattern

Authentication page using NextAuth.js `signIn()`. Client Component for
the sign-in form with error handling.

## Template

```tsx
"use client";

import { signIn } from "next-auth/react";
import { useState } from "react";
import { useSearchParams } from "next/navigation";

export default function SignInPage() {
  const searchParams = useSearchParams();
  const error = searchParams.get("error");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    await signIn("credentials", { email, password, callbackUrl: "/app" });
    setLoading(false);
  }

  return (
    <div className="min-h-screen flex items-center justify-center">
      <div className="w-full max-w-sm space-y-6">
        <h1 className="text-2xl font-bold text-center">Sign In</h1>

        {error && (
          <p className="text-sm text-red-600 text-center">Invalid credentials</p>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1">Email</label>
            <input type="email" value={email} onChange={(e) => setEmail(e.target.value)}
              required className="w-full rounded-md border px-3 py-2" />
          </div>
          <div>
            <label className="block text-sm font-medium mb-1">Password</label>
            <input type="password" value={password} onChange={(e) => setPassword(e.target.value)}
              required className="w-full rounded-md border px-3 py-2" />
          </div>
          <button type="submit" disabled={loading}
            className="w-full rounded-md bg-indigo-600 py-2 text-white hover:bg-indigo-700 disabled:opacity-50">
            {loading ? "Signing in..." : "Sign In"}
          </button>
        </form>

        <div className="relative">
          <div className="absolute inset-0 flex items-center"><div className="w-full border-t" /></div>
          <div className="relative flex justify-center text-sm"><span className="bg-white px-2 text-gray-500">Or</span></div>
        </div>

        <button onClick={() => signIn("github", { callbackUrl: "/app" })}
          className="w-full rounded-md border py-2 text-sm hover:bg-gray-50">
          Continue with GitHub
        </button>
      </div>
    </div>
  );
}
```

## Rules

1. `"use client"` — sign-in needs client-side interaction.
2. Use `signIn()` from `next-auth/react` — never implement auth manually.
3. Read `error` from search params to show login failures.
4. Support both credentials and OAuth providers via separate buttons.
5. Redirect to `/app` (authenticated area) on success via `callbackUrl`.
