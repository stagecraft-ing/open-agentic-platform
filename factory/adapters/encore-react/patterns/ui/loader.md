# Loader Pattern

Server-side `loader` functions run on every GET request before the component renders. They enforce auth, call Encore services, and return data to the component.

## Convention

- `loader` receives `{ request: Request }` and returns serializable data.
- Auth is enforced by calling `requireUser(request)` or `requireAdmin(request)`.
- These helpers parse cookies, validate the session via the Encore auth service, and `throw redirect()` on failure.
- The Encore client is instantiated per-request via `createEncoreClient(request)` to resolve the correct API base URL.

## Template

```tsx
import { redirect } from "react-router";
import { requireUser } from "../lib/auth.server";       // or requireAdmin
import { createEncoreClient } from "../lib/encore.server";

export async function loader({ request }: { request: Request }) {
  // 1. Enforce authentication (throws redirect on failure)
  const user = await require{Role}(request);

  // 2. Create request-scoped Encore client
  const client = createEncoreClient(request);

  // 3. Fetch data from Encore backend services
  const res = await client.{service}.{method}();

  // 4. Return data to component
  return { user, {dataKey}: res.{dataKey} };
}
```

## Auth Enforcement (`auth.server.ts`)

```ts
// requireUser: parses __session cookie, validates via auth.session(),
//              throws redirect("/signin") if missing or invalid.
export async function requireUser(request: Request) { ... }

// requireAdmin: parses __admin_session cookie, validates via auth.adminSession(),
//               checks role === "admin", throws redirect("/admin/signin") if invalid.
export async function requireAdmin(request: Request) { ... }
```

## Encore Client Factory (`encore.server.ts`)

```ts
export function createEncoreClient(request: Request): Client {
  // Resolves base URL from request origin,
  // falls back to ENCORE_API_BASE_URL env var or localhost:4000.
  return new Client(baseUrl);
}
```

## Example (from `admin.users.tsx`)

```tsx
export async function loader({ request }: { request: Request }) {
  const admin = await requireAdmin(request);
  const client = createEncoreClient(request);
  const res = await client.admin.listUsers();
  return { admin, users: res.users };
}
```

## Example: Layout Loader (from `app.tsx`)

```tsx
export async function loader({ request }: { request: Request }) {
  const user = await requireUser(request);
  return { user };
}
```

Layout loaders run before any child route loader. The child component accesses its own loader data, not the parent's.

## Rules

1. Every protected route must call `requireUser` or `requireAdmin` as the first line.
2. Auth helpers throw `redirect()` -- do not catch it. React Router handles the redirect.
3. Always create the Encore client via `createEncoreClient(request)`, never with a hardcoded URL.
4. Loaders must return plain serializable objects (no class instances, no functions).
5. Files ending in `.server.ts` are server-only; never import them in client components.
6. Layout loaders (e.g., `app.tsx`) gate all nested child routes behind auth.
