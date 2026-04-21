# Route Definition Pattern

Routes are defined explicitly in `web/app/routes.ts` using React Router's `route`, `index`, and `layout` helpers. Route files live in `web/app/routes/` with flat dot-notation naming.

## Convention

- `routes.ts` exports a `RouteConfig` array that maps URL paths to route files.
- Layout routes wrap children with shared UI (nav, auth) and render `<Outlet />`.
- Index routes render when the parent path matches exactly.
- Dot notation in filenames indicates nesting: `app._index.tsx` is the index inside `app.tsx`.

## Template

```ts
import {
  type RouteConfig,
  index,
  layout,
  route,
} from "@react-router/dev/routes";

export default [
  // Public routes (no layout wrapper)
  index("routes/_index.tsx"),
  route("{path}", "routes/{filename}.tsx"),

  // Protected layout with nested children
  route("{layoutPath}", "routes/{layout}.tsx", [
    index("routes/{layout}._index.tsx"),
    route("{childPath}", "routes/{layout}.{child}.tsx"),
  ]),
] satisfies RouteConfig;
```

## Example (from `routes.ts`)

```ts
export default [
  index("routes/_index.tsx"),                       // GET /
  route("pricing", "routes/pricing.tsx"),            // GET /pricing
  route("signin", "routes/signin.tsx"),              // GET /signin
  route("signup", "routes/signup.tsx"),               // GET /signup
  route("admin/signin", "routes/admin.signin.tsx"),  // GET /admin/signin (no layout)
  route("app", "routes/app.tsx", [                   // Layout: /app/*
    index("routes/app._index.tsx"),                  //   GET /app
    route("settings", "routes/app.settings.tsx"),    //   GET /app/settings
  ]),
  route("admin", "routes/admin.tsx", [               // Layout: /admin/*
    index("routes/admin._index.tsx"),                //   GET /admin
    route("users", "routes/admin.users.tsx"),         //   GET /admin/users
    route("audit", "routes/admin.audit.tsx"),         //   GET /admin/audit
  ]),
] satisfies RouteConfig;
```

## File Naming to URL Mapping

| File                      | URL              | Role         |
|---------------------------|------------------|--------------|
| `_index.tsx`              | `/`              | Landing page |
| `signin.tsx`              | `/signin`        | Public page  |
| `app.tsx`                 | `/app/*`         | Layout       |
| `app._index.tsx`          | `/app`           | Index child  |
| `app.settings.tsx`        | `/app/settings`  | Nested child |
| `admin.tsx`               | `/admin/*`       | Layout       |
| `admin.users.tsx`         | `/admin/users`   | Nested child |
| `admin.signin.tsx`        | `/admin/signin`  | Standalone   |

## Layout Route Anatomy

```tsx
import { Outlet, useLoaderData, Link } from "react-router";
import { requireUser } from "../lib/auth.server";

export async function loader({ request }: { request: Request }) {
  const user = await requireUser(request);
  return { user };
}

export default function AppLayout() {
  const { user } = useLoaderData() as { user: { name: string } };
  return (
    <div className="min-h-full container px-4 mx-auto my-8">
      <nav className="flex gap-4 mb-8 border-b border-gray-200 pb-4">
        <Link to="/app">Dashboard</Link>
        <Link to="/app/settings">Settings</Link>
      </nav>
      <main>
        <Outlet />   {/* Child routes render here */}
      </main>
    </div>
  );
}
```

## Rules

1. All route mappings go in `routes.ts`; do not rely on filesystem-based auto-routing.
2. Layout routes must render `<Outlet />` for children to appear.
3. Layout loaders run before child loaders -- use them for shared auth gates.
4. A route outside a layout's children array (e.g., `admin.signin.tsx`) bypasses that layout.
5. Index routes use `index()`, not `route("", ...)`.
6. The `satisfies RouteConfig` assertion provides type checking without widening.
