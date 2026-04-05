# Layout Pattern

Layouts in the App Router wrap pages and persist across navigation.
The authenticated layout provides the shell (navbar, sidebar, footer).

## Convention

- Root layout: `src/app/layout.tsx` — HTML shell, metadata, providers
- App layout: `src/app/(app)/layout.tsx` — authenticated shell with navigation
- Auth layout: `src/app/auth/layout.tsx` — minimal layout for sign-in/up pages
- Layouts are Server Components by default

## Template

### Root layout (`src/app/layout.tsx`)

```tsx
import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "{AppName}",
  description: "{AppDescription}",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="min-h-screen bg-white dark:bg-gray-900">
        {children}
      </body>
    </html>
  );
}
```

### App layout (`src/app/(app)/layout.tsx`)

```tsx
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { redirect } from "next/navigation";
import { Navbar } from "@/components/Navbar";
import { Sidebar } from "@/components/Sidebar";

export default async function AppLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const session = await getServerSession(authOptions);
  if (!session) redirect("/auth/signin");

  return (
    <div className="min-h-screen flex">
      <Sidebar user={session.user} />
      <div className="flex-1 flex flex-col">
        <Navbar user={session.user} />
        <main className="flex-1 p-6">{children}</main>
      </div>
    </div>
  );
}
```

## Example

From a real app layout:

```tsx
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";
import { redirect } from "next/navigation";

export default async function AppLayout({ children }: { children: React.ReactNode }) {
  const session = await getServerSession(authOptions);
  if (!session) redirect("/auth/signin");

  return (
    <div className="min-h-screen">
      <header className="border-b px-6 py-4">
        <span className="font-semibold">{session.user?.email}</span>
      </header>
      <main className="p-6">{children}</main>
    </div>
  );
}
```

## Rules

1. Route groups `(app)`, `(auth)` create layout boundaries without URL segments.
2. App layout checks auth and redirects — individual pages don't need to.
3. Root layout includes `<html>` and `<body>` tags — inner layouts do not.
4. Layouts are Server Components — no `"use client"` unless interactivity is needed.
5. Pass `session.user` to navigation components via props.
