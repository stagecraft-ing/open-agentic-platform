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
  route("auth/no-org", "routes/auth.no-org.tsx"),
  route("auth/org-select", "routes/auth.org-select.tsx"),
  route("admin/signin", "routes/admin.signin.tsx"),
  route("app", "routes/app.tsx", [
    index("routes/app._index.tsx"),
    route("knowledge", "routes/app.knowledge.tsx"),
    route("knowledge/:id", "routes/app.knowledge.$id.tsx"),
    route("pipelines", "routes/app.pipelines.tsx"),
    route("pipelines/:projectId", "routes/app.pipelines.$projectId.tsx"),
    route("deploys", "routes/app.deploys.tsx"),
    route("settings", "routes/app.settings.tsx", [
      index("routes/app.settings._index.tsx"),
      route("connectors", "routes/app.settings.connectors.tsx"),
      route("connectors/new", "routes/app.settings.connectors.new.tsx"),
      route("connectors/:id", "routes/app.settings.connectors.$id.tsx"),
    ]),
  ]),
  route("admin", "routes/admin.tsx", [
    index("routes/admin._index.tsx"),
    route("users", "routes/admin.users.tsx"),
    route("audit", "routes/admin.audit.tsx"),
    route("projects", "routes/admin.projects.tsx", [
      index("routes/admin.projects._index.tsx"),
      route("new", "routes/admin.projects.new.tsx"),
      route(":id", "routes/admin.projects.$id.tsx"),
    ]),
  ]),
] satisfies RouteConfig;
