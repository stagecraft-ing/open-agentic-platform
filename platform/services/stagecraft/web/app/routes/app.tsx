import { Outlet, useLoaderData, NavLink } from "react-router";
import { requireUser } from "../lib/auth.server";
import { getDefaultWorkspace } from "../lib/workspace-api.server";
import type { WorkspaceRow } from "../lib/workspace-api.server";

export async function loader({ request }: { request: Request }) {
  const user = await requireUser(request);
  let workspace: WorkspaceRow | null = null;
  try {
    const res = await getDefaultWorkspace(request);
    workspace = res.workspace;
  } catch {
    // Workspace may not be available yet
  }
  return { user, workspace };
}

const NAV_ITEMS = [
  { to: "/app", label: "Dashboard", end: true },
  { to: "/app/knowledge", label: "Knowledge", end: false },
  { to: "/app/pipelines", label: "Pipelines", end: false },
  { to: "/app/deploys", label: "Deploys", end: false },
  { to: "/app/settings", label: "Settings", end: false },
];

export default function AppLayout() {
  const { user, workspace } = useLoaderData() as {
    user: { name: string; email: string; orgSlug?: string };
    workspace: WorkspaceRow | null;
  };

  return (
    <div className="min-h-full">
      <header className="border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900">
        <div className="container px-4 mx-auto">
          <div className="flex items-center justify-between h-14">
            <div className="flex items-center gap-4">
              <span className="text-sm font-semibold text-gray-900 dark:text-gray-100 tracking-tight">
                stagecraft
              </span>
              {workspace && (
                <span className="text-xs text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-800 px-2 py-0.5 rounded">
                  {workspace.name}
                </span>
              )}
            </div>
            <div className="flex items-center gap-3">
              <span className="text-sm text-gray-600 dark:text-gray-400">
                {user.name}
              </span>
              {user.orgSlug && (
                <span className="text-xs text-gray-400 dark:text-gray-500">
                  {user.orgSlug}
                </span>
              )}
            </div>
          </div>

          <nav className="flex gap-1 -mb-px">
            {NAV_ITEMS.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.end}
                className={({ isActive }) =>
                  `px-3 py-2 text-sm font-medium border-b-2 transition-colors ${
                    isActive
                      ? "border-indigo-500 text-indigo-600 dark:text-indigo-400"
                      : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
                  }`
                }
              >
                {item.label}
              </NavLink>
            ))}
          </nav>
        </div>
      </header>

      <main className="container px-4 mx-auto py-6">
        <Outlet context={{ workspace }} />
      </main>
    </div>
  );
}
