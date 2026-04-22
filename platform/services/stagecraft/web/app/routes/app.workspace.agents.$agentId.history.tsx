/**
 * Spec 111 Phase 4 — Agent audit/history view.
 *
 * Renders the append-only `agent_catalog_audit` trail for a single agent
 * scoped to the caller's workspace. The list is the compliance view behind
 * "every publish/retire is audited" (spec 111 §2.2/§2.6).
 */

import { Link, useLoaderData } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getAgent,
  listAgentAudit,
  type CatalogAgent,
  type CatalogAuditEntry,
} from "../lib/agents-api.server";

const ACTION_COLORS: Record<string, string> = {
  create:
    "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300",
  edit: "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300",
  publish:
    "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
  retire:
    "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300",
  fork:
    "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300",
};

export async function loader({
  request,
  params,
}: {
  request: Request;
  params: { agentId: string };
}) {
  await requireUser(request);
  const [{ agent }, { entries }] = await Promise.all([
    getAgent(request, params.agentId),
    listAgentAudit(request, params.agentId),
  ]);
  return { agent, entries };
}

export default function AgentHistory() {
  const { agent, entries } = useLoaderData() as {
    agent: CatalogAgent;
    entries: CatalogAuditEntry[];
  };

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
            History: <span className="font-mono">{agent.name}</span>
          </h3>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Audit trail from `agent_catalog_audit` (append-only).
          </p>
        </div>
        <Link
          to={`/app/workspace/agents/${agent.id}`}
          className="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200"
        >
          ← Back to agent
        </Link>
      </div>

      {entries.length === 0 ? (
        <div className="border border-dashed border-gray-300 dark:border-gray-600 rounded-lg px-4 py-12 text-center">
          <p className="text-sm text-gray-500 dark:text-gray-400">
            No audit entries for this agent yet.
          </p>
        </div>
      ) : (
        <ol className="space-y-3">
          {entries.map((e) => {
            const versionBefore =
              (e.before?.version as number | undefined) ?? null;
            const versionAfter =
              (e.after?.version as number | undefined) ?? null;
            const statusBefore =
              (e.before?.status as string | undefined) ?? null;
            const statusAfter =
              (e.after?.status as string | undefined) ?? null;
            const hashAfter =
              (e.after?.content_hash as string | undefined) ?? null;

            return (
              <li
                key={e.id}
                className="border border-gray-200 dark:border-gray-700 rounded-md px-4 py-3"
              >
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2">
                    <span
                      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${ACTION_COLORS[e.action] ?? ""}`}
                    >
                      {e.action}
                    </span>
                    {versionAfter !== null && (
                      <span className="text-xs text-gray-500 dark:text-gray-400">
                        v{versionAfter}
                        {versionBefore !== null &&
                        versionBefore !== versionAfter
                          ? ` (was v${versionBefore})`
                          : ""}
                      </span>
                    )}
                    {statusAfter && (
                      <span className="text-xs text-gray-500 dark:text-gray-400">
                        {statusBefore && statusBefore !== statusAfter
                          ? `${statusBefore} → ${statusAfter}`
                          : statusAfter}
                      </span>
                    )}
                  </div>
                  <time
                    className="text-xs text-gray-500 dark:text-gray-400"
                    dateTime={e.created_at}
                  >
                    {new Date(e.created_at).toLocaleString()}
                  </time>
                </div>
                <div className="mt-1 text-xs text-gray-500 dark:text-gray-400 font-mono break-all">
                  actor {e.actor_user_id}
                  {hashAfter && <> · hash {hashAfter.slice(0, 16)}…</>}
                </div>
              </li>
            );
          })}
        </ol>
      )}
    </div>
  );
}
