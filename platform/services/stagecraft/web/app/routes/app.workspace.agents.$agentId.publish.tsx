/**
 * Spec 111 Phase 4 — Publish confirmation.
 *
 * Publishing a draft bumps version, auto-retires any prior published sibling
 * with the same (workspace, name), and broadcasts `agent.catalog.updated` to
 * every OPC session on the workspace (spec 111 §2.3). Requires workspace
 * owner/admin per §2.2 RBAC; surfaced as a server error if the caller lacks
 * the role.
 */

import {
  Form,
  Link,
  redirect,
  useActionData,
  useLoaderData,
  useNavigation,
} from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getAgent,
  publishAgent,
  type CatalogAgent,
} from "../lib/agents-api.server";

export async function loader({
  request,
  params,
}: {
  request: Request;
  params: { agentId: string };
}) {
  await requireUser(request);
  const { agent } = await getAgent(request, params.agentId);
  if (agent.status !== "draft") {
    throw redirect(`/app/workspace/agents/${agent.id}`);
  }
  return { agent };
}

export async function action({
  request,
  params,
}: {
  request: Request;
  params: { agentId: string };
}) {
  await requireUser(request);
  try {
    const res = await publishAgent(request, params.agentId);
    return redirect(`/app/workspace/agents/${res.agent.id}`);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    try {
      const parsed = JSON.parse(msg) as { message?: string };
      if (parsed.message) return { error: parsed.message };
    } catch {
      // fallthrough
    }
    return { error: msg };
  }
}

export default function PublishAgent() {
  const { agent } = useLoaderData() as { agent: CatalogAgent };
  const actionData = useActionData() as { error?: string } | undefined;
  const navigation = useNavigation();
  const submitting = navigation.state === "submitting";

  return (
    <div className="max-w-2xl space-y-5">
      <div>
        <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100">
          Publish {agent.name} v{agent.version}
        </h3>
        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
          Publishing propagates this agent to every OPC currently connected to
          the workspace. Prior published versions of{" "}
          <code className="font-mono">{agent.name}</code> will be auto-retired.
        </p>
      </div>

      {actionData?.error && (
        <div className="rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 px-4 py-3">
          <p className="text-sm text-red-700 dark:text-red-400">
            {actionData.error}
          </p>
        </div>
      )}

      <div className="rounded-md border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/40 p-4 space-y-3 text-sm">
        <p className="text-gray-700 dark:text-gray-300 font-medium">
          Before you publish
        </p>
        <ul className="space-y-1 text-gray-600 dark:text-gray-400 list-disc list-inside">
          <li>
            Publication is audited — the current user, the draft's{" "}
            <code className="font-mono">content_hash</code>, and the active
            policy bundle are recorded.
          </li>
          <li>
            The workspace's current policy bundle is referenced at publish
            time. Subsequent bundle changes do not auto-retire this agent; any
            drift is reported on future execution per spec 111 §2.6.
          </li>
          <li>
            Only workspace <strong>owners</strong> and <strong>admins</strong>{" "}
            can publish.
          </li>
        </ul>
      </div>

      <div className="rounded-md border border-gray-200 dark:border-gray-700 px-4 py-3 text-sm space-y-1 font-mono">
        <div>
          <span className="text-gray-500 dark:text-gray-400">name</span>{" "}
          {agent.name}
        </div>
        <div>
          <span className="text-gray-500 dark:text-gray-400">version</span>{" "}
          v{agent.version}
        </div>
        <div>
          <span className="text-gray-500 dark:text-gray-400">type</span>{" "}
          {agent.frontmatter.type}
        </div>
        <div>
          <span className="text-gray-500 dark:text-gray-400">safety_tier</span>{" "}
          {agent.frontmatter.safety_tier ?? "—"}
        </div>
        <div className="break-all">
          <span className="text-gray-500 dark:text-gray-400">content_hash</span>{" "}
          {agent.content_hash}
        </div>
      </div>

      <Form method="post" className="flex items-center gap-4">
        <button
          type="submit"
          disabled={submitting}
          className="inline-flex items-center rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50"
        >
          {submitting ? "Publishing…" : "Publish now"}
        </button>
        <Link
          to={`/app/workspace/agents/${agent.id}`}
          className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
        >
          Cancel
        </Link>
      </Form>
    </div>
  );
}
