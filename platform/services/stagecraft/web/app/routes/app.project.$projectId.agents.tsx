/**
 * Spec 111 + 119 — Project-scoped agent catalog layout.
 *
 * The catalog is scoped to the project in the URL. Detail endpoints
 * (`/api/agents/:id`) resolve the project from the agent row; list/create
 * (`/api/projects/:projectId/agents`) take the projectId from this route.
 */

import { Outlet } from "react-router";
import { requireUser } from "../lib/auth.server";

export async function loader({ request }: { request: Request }) {
  await requireUser(request);
  return null;
}

export default function AgentsLayout() {
  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Agent Catalog
        </h2>
        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
          Project agents shared with every OPC bound to this project.
          Published agents are pushed over the duplex channel; retirements
          propagate automatically.
        </p>
      </div>
      <Outlet />
    </div>
  );
}
