import {
  type RouteConfig,
  index,
  layout,
  route,
} from "@react-router/dev/routes";

export default [
  index("routes/_index.tsx"),
  route("pricing", "routes/pricing.tsx"),
  route("signin", "routes/signin.tsx"),
  route("signup", "routes/signup.tsx"),
  route("admin/signin", "routes/admin.signin.tsx"),
  route("app", "routes/app.tsx", [
    index("routes/app._index.tsx"),
    route("settings", "routes/app.settings.tsx"),
  ]),
  route("admin", "routes/admin.tsx", [
    index("routes/admin._index.tsx"),
    route("users", "routes/admin.users.tsx"),
    route("audit", "routes/admin.audit.tsx"),
  ]),
] satisfies RouteConfig;
