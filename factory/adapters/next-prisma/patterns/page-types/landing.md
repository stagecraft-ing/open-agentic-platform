# Landing Page Pattern

Public marketing page — no authentication required. Server Component with
static content, hero section, and call-to-action.

## Template

```tsx
import Link from "next/link";

export default function LandingPage() {
  return (
    <div className="min-h-screen">
      <header className="bg-white border-b">
        <div className="container mx-auto px-4 py-4 flex justify-between items-center">
          <span className="text-xl font-bold">{AppName}</span>
          <Link href="/auth/signin" className="text-indigo-600 hover:text-indigo-800">Sign In</Link>
        </div>
      </header>
      <main className="container mx-auto px-4 py-16 text-center">
        <h1 className="text-4xl font-bold mb-4">{Headline}</h1>
        <p className="text-xl text-gray-600 mb-8">{Description}</p>
        <Link href="/auth/signup"
          className="rounded-md bg-indigo-600 px-6 py-3 text-white font-medium hover:bg-indigo-700">
          Get Started
        </Link>
      </main>
    </div>
  );
}
```

## Rules

1. No auth check — landing pages are public.
2. Server Component (no `"use client"`).
3. Use `Link` from `next/link` for navigation.
4. Include clear call-to-action pointing to signup/signin.
