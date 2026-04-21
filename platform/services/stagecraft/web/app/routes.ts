import {
  type RouteConfig,
  index,
  route,
} from "@react-router/dev/routes";

export default [
  index("routes/_index.tsx"),
  route("signin", "routes/signin.tsx"),
  route("signup", "routes/signup.tsx"),
  route("auth/no-org", "routes/auth.no-org.tsx"),
  route("auth/org-select", "routes/auth.org-select.tsx"),
  route("admin/signin", "routes/admin.signin.tsx"),
  route("app", "routes/app.tsx", [
    index("routes/app._index.tsx"),
    route("factory", "routes/app.factory.tsx", [
      index("routes/app.factory._index.tsx"),
      route("upstreams", "routes/app.factory.upstreams.tsx"),
      route("adapters", "routes/app.factory.adapters.tsx"),
      route("contracts", "routes/app.factory.contracts.tsx"),
      route("processes", "routes/app.factory.processes.tsx"),
    ]),
    route("projects/new", "routes/app.projects.new.tsx"),
    route("project/:projectId", "routes/app.project.$projectId.tsx", [
      index("routes/app.project.$projectId._index.tsx"),
      route("knowledge", "routes/app.project.$projectId.knowledge.tsx"),
      route("knowledge/:id", "routes/app.project.$projectId.knowledge.$id.tsx"),
      route("pipelines", "routes/app.project.$projectId.pipelines.tsx"),
      route("deploys", "routes/app.project.$projectId.deploys.tsx"),
      route("settings", "routes/app.project.$projectId.settings.tsx", [
        index("routes/app.project.$projectId.settings._index.tsx"),
        route("connectors", "routes/app.project.$projectId.settings.connectors.tsx"),
        route("connectors/new", "routes/app.project.$projectId.settings.connectors.new.tsx"),
        route("connectors/:id", "routes/app.project.$projectId.settings.connectors.$id.tsx"),
        route("github-pat", "routes/app.project.$projectId.settings.github-pat.tsx"),
      ]),
    ]),
  ]),
  route("admin", "routes/admin.tsx", [
    index("routes/admin._index.tsx"),
    route("users", "routes/admin.users.tsx"),
    route("audit", "routes/admin.audit.tsx"),
    route("sessions", "routes/admin.sessions.tsx"),
    route("oidc-providers", "routes/admin.oidc-providers.tsx"),
    route("projects", "routes/admin.projects.tsx", [
      index("routes/admin.projects._index.tsx"),
      route("new", "routes/admin.projects.new.tsx"),
      route(":id", "routes/admin.projects.$id.tsx"),
    ]),
  ]),
] satisfies RouteConfig;
