import { useLoaderData } from "react-router";
import { requireUser } from "../lib/auth.server";
import { listEnvironments } from "../lib/workspace-api.server";
import type { EnvironmentRow } from "../lib/workspace-api.server";

export async function loader({
  request,
  params,
}: {
  request: Request;
  params: { projectId: string };
}) {
  await requireUser(request);

  let environments: EnvironmentRow[] = [];
  try {
    const envRes = await listEnvironments(request, params.projectId);
    environments = envRes.environments;
  } catch {
    // deployd service may not be ready
  }

  return { environments };
}

const ENV_KIND_COLORS: Record<string, string> = {
  preview: "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300",
  development: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300",
  staging: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300",
  production: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
};

export default function DeployStatus() {
  const { environments } = useLoaderData() as { environments: EnvironmentRow[] };

  return (
    <div className="space-y-6">
      <p className="text-sm text-gray-500 dark:text-gray-400">
        Deployment environments for this project. Promotions are triggered from
        the factory pipeline or via the deployment API.
      </p>

      {environments.length === 0 ? (
        <div className="border border-dashed border-gray-300 dark:border-gray-600 rounded-lg px-4 py-12 text-center">
          <p className="text-sm text-gray-500 dark:text-gray-400">
            No environments configured.
          </p>
        </div>
      ) : (
        <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden bg-white dark:bg-gray-900">
          <div className="px-4 py-4">
            <div className="flex items-center gap-2 flex-wrap">
              {sortEnvsByKind(environments).map((env, i, arr) => (
                <div key={env.id} className="flex items-center gap-2">
                  <div className="border border-gray-200 dark:border-gray-700 rounded-lg px-3 py-2 min-w-[140px]">
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
                        {env.name}
                      </span>
                      <span
                        className={`inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium ${ENV_KIND_COLORS[env.kind] ?? "bg-gray-100 text-gray-800"}`}
                      >
                        {env.kind}
                      </span>
                    </div>
                    {env.autoDeployBranch && (
                      <p className="text-xs text-gray-500 dark:text-gray-400">
                        auto: {env.autoDeployBranch}
                      </p>
                    )}
                    {env.requiresApproval && (
                      <p className="text-xs text-yellow-600 dark:text-yellow-400">
                        requires approval
                      </p>
                    )}
                    {env.k8sNamespace && (
                      <p className="text-[10px] text-gray-400 dark:text-gray-500 mt-1 font-mono">
                        ns: {env.k8sNamespace}
                      </p>
                    )}
                  </div>

                  {i < arr.length - 1 && (
                    <svg
                      className="w-5 h-5 text-gray-300 dark:text-gray-600 flex-shrink-0"
                      fill="currentColor"
                      viewBox="0 0 20 20"
                    >
                      <path
                        fillRule="evenodd"
                        d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
                        clipRule="evenodd"
                      />
                    </svg>
                  )}
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

const ENV_KIND_ORDER = ["preview", "development", "staging", "production"];

function sortEnvsByKind(envs: EnvironmentRow[]): EnvironmentRow[] {
  return [...envs].sort((a, b) => {
    const aIdx = ENV_KIND_ORDER.indexOf(a.kind);
    const bIdx = ENV_KIND_ORDER.indexOf(b.kind);
    return (aIdx === -1 ? 99 : aIdx) - (bIdx === -1 ? 99 : bIdx);
  });
}
