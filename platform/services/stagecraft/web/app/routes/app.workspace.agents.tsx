/**
 * Spec 111 Phase 4 — Workspace agent catalog layout.
 *
 * Scoped to the user's current workspace (derived from the session cookie by
 * `requireWorkspaceAuth` on the API side); there is no workspaceId in the URL
 * because an OPC session is bound to a single workspace at a time.
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
          Organisational agents shared with every OPC connected to this
          workspace. Published agents are pushed over the duplex channel;
          retirements propagate automatically.
        </p>
      </div>
      <Outlet />
    </div>
  );
}
