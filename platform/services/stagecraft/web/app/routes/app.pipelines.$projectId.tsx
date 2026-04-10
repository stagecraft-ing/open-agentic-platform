import { useLoaderData, useFetcher, Link } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getFactoryStatus,
  listFactoryAudit,
  confirmFactoryStage,
  rejectFactoryStage,
  cancelPipeline,
} from "../lib/workspace-api.server";
import { getProject } from "../lib/projects-api.server";
import { useState } from "react";

const PIPELINE_STAGES = [
  { id: "s0-preflight", label: "Pre-flight" },
  { id: "s1-business-requirements", label: "Business Req" },
  { id: "s2-service-requirements", label: "Service Req" },
  { id: "s3-data-model", label: "Data Model" },
  { id: "s4-api-spec", label: "API Spec" },
  { id: "s5-ui-spec", label: "UI Spec" },
  { id: "s6-scaffolding", label: "Scaffolding" },
] as const;

const STATUS_COLORS: Record<string, { bg: string; ring: string; text: string }> = {
  completed: {
    bg: "bg-green-500",
    ring: "ring-green-500",
    text: "text-green-700 dark:text-green-400",
  },
  confirmed: {
    bg: "bg-green-500",
    ring: "ring-green-500",
    text: "text-green-700 dark:text-green-400",
  },
  in_progress: {
    bg: "bg-blue-500",
    ring: "ring-blue-500",
    text: "text-blue-700 dark:text-blue-400",
  },
  pending: {
    bg: "bg-gray-300 dark:bg-gray-600",
    ring: "ring-gray-300 dark:ring-gray-600",
    text: "text-gray-500 dark:text-gray-400",
  },
  rejected: {
    bg: "bg-red-500",
    ring: "ring-red-500",
    text: "text-red-700 dark:text-red-400",
  },
  failed: {
    bg: "bg-red-500",
    ring: "ring-red-500",
    text: "text-red-700 dark:text-red-400",
  },
};

const PIPELINE_STATUS_COLORS: Record<string, string> = {
  active: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300",
  completed: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
  cancelled: "bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-300",
  failed: "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300",
};

type StageData = {
  status: string;
  started_at?: string;
  completed_at?: string;
  confirmed_by?: string;
  confirmed_at?: string;
};

type FactoryStatus = {
  pipeline_id: string;
  status: string;
  adapter: string;
  current_stage: string | null;
  stages: Record<string, StageData>;
  token_spend: { total: number; budget: number; by_stage: Record<string, number> };
  started_at: string | null;
};

export async function loader({
  request,
  params,
}: {
  request: Request;
  params: { projectId: string };
}) {
  await requireUser(request);

  let project: { id: string; name: string; slug: string } | null = null;
  try {
    const pRes = await getProject(request, params.projectId);
    project = pRes.project;
  } catch {
    // project may not exist
  }

  let pipeline: FactoryStatus | null = null;
  try {
    const fRes = await getFactoryStatus(request, params.projectId);
    pipeline = fRes.pipeline as FactoryStatus | null;
  } catch {
    // no active pipeline
  }

  let auditEntries: Array<{
    id: string;
    timestamp: string;
    event: string;
    actor: string | null;
    stageId: string | null;
    details: unknown;
  }> = [];
  try {
    const aRes = await listFactoryAudit(request, params.projectId);
    auditEntries = aRes.entries;
  } catch {
    // audit may not be available
  }

  return { project, pipeline, auditEntries };
}

export async function action({
  request,
  params,
}: {
  request: Request;
  params: { projectId: string };
}) {
  const user = await requireUser(request);
  const form = await request.formData();
  const intent = form.get("intent");

  if (intent === "confirm") {
    const stageId = form.get("stageId") as string;
    const notes = form.get("notes") as string | null;
    await confirmFactoryStage(request, params.projectId, stageId, user.userId, notes ?? undefined);
    return { confirmed: stageId };
  }

  if (intent === "reject") {
    const stageId = form.get("stageId") as string;
    const reason = form.get("reason") as string;
    await rejectFactoryStage(request, params.projectId, stageId, user.userId, reason);
    return { rejected: stageId };
  }

  if (intent === "cancel") {
    const reason = form.get("reason") as string | null;
    await cancelPipeline(request, params.projectId, user.userId, reason ?? undefined);
    return { cancelled: true };
  }

  return null;
}

export default function PipelineDetail() {
  const { project, pipeline, auditEntries } = useLoaderData() as {
    project: { id: string; name: string; slug: string } | null;
    pipeline: FactoryStatus | null;
    auditEntries: Array<{
      id: string;
      timestamp: string;
      event: string;
      actor: string | null;
      stageId: string | null;
      details: unknown;
    }>;
  };

  if (!project) {
    return (
      <div className="text-center py-12">
        <p className="text-gray-500 dark:text-gray-400">Project not found.</p>
        <Link
          to="/app/pipelines"
          className="text-sm text-indigo-600 hover:text-indigo-500 dark:text-indigo-400 mt-2 inline-block"
        >
          Back to pipelines
        </Link>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Breadcrumb */}
      <nav className="text-sm text-gray-500 dark:text-gray-400">
        <Link
          to="/app/pipelines"
          className="hover:text-gray-700 dark:hover:text-gray-300"
        >
          Pipelines
        </Link>
        <span className="mx-1">/</span>
        <span className="text-gray-900 dark:text-gray-100">
          {project.name}
        </span>
      </nav>

      {!pipeline ? (
        <div className="border border-dashed border-gray-300 dark:border-gray-600 rounded-lg px-4 py-12 text-center">
          <p className="text-sm text-gray-500 dark:text-gray-400">
            No active factory pipeline for this project.
          </p>
          <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
            Initialize a pipeline from OPC desktop or via the API.
          </p>
        </div>
      ) : (
        <>
          {/* Pipeline header */}
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
                Pipeline: {project.name}
              </h2>
              <div className="flex items-center gap-3 mt-1">
                <span
                  className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${PIPELINE_STATUS_COLORS[pipeline.status] ?? "bg-gray-100 text-gray-800"}`}
                >
                  {pipeline.status}
                </span>
                <span className="text-sm text-gray-500 dark:text-gray-400">
                  Adapter: {pipeline.adapter}
                </span>
                {pipeline.started_at && (
                  <span className="text-sm text-gray-500 dark:text-gray-400">
                    Started: {new Date(pipeline.started_at).toLocaleString()}
                  </span>
                )}
              </div>
            </div>

            {pipeline.status === "active" && <CancelButton />}
          </div>

          {/* Stage progress visualization */}
          <div className="border border-gray-200 dark:border-gray-700 rounded-lg p-4 bg-white dark:bg-gray-900">
            <div className="flex items-center justify-between mb-4">
              {PIPELINE_STAGES.map((stage, i) => {
                const data = pipeline.stages[stage.id];
                const status = data?.status ?? "pending";
                const colors = STATUS_COLORS[status] ?? STATUS_COLORS.pending;
                const isCurrent = pipeline.current_stage === stage.id;

                return (
                  <div key={stage.id} className="flex-1 flex flex-col items-center relative">
                    {/* Connector line */}
                    {i > 0 && (
                      <div
                        className={`absolute top-3 right-1/2 w-full h-0.5 -translate-y-1/2 ${
                          (data?.status === "completed" || data?.status === "confirmed")
                            ? "bg-green-500"
                            : "bg-gray-200 dark:bg-gray-700"
                        }`}
                      />
                    )}

                    {/* Node */}
                    <div
                      className={`relative z-10 w-6 h-6 rounded-full ${colors.bg} ${
                        isCurrent ? `ring-2 ring-offset-2 ring-offset-white dark:ring-offset-gray-900 ${colors.ring}` : ""
                      } flex items-center justify-center`}
                    >
                      {(status === "completed" || status === "confirmed") && (
                        <svg className="w-3.5 h-3.5 text-white" fill="currentColor" viewBox="0 0 20 20">
                          <path
                            fillRule="evenodd"
                            d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                            clipRule="evenodd"
                          />
                        </svg>
                      )}
                    </div>

                    {/* Label */}
                    <span
                      className={`mt-2 text-xs text-center ${colors.text} ${isCurrent ? "font-semibold" : ""}`}
                    >
                      {stage.label}
                    </span>
                    <span className="text-[10px] text-gray-400">{status}</span>
                  </div>
                );
              })}
            </div>

            {/* Token spend bar */}
            <div className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
              <div className="flex items-center justify-between text-xs text-gray-500 dark:text-gray-400 mb-1">
                <span>Token spend</span>
                <span>
                  {pipeline.token_spend.total.toLocaleString()} /{" "}
                  {pipeline.token_spend.budget.toLocaleString()}
                </span>
              </div>
              <div className="h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                <div
                  className="h-full bg-indigo-500 rounded-full transition-all"
                  style={{
                    width: `${Math.min(100, (pipeline.token_spend.total / pipeline.token_spend.budget) * 100)}%`,
                  }}
                />
              </div>
            </div>
          </div>

          {/* Gate actions for current stage */}
          {pipeline.current_stage && (
            <GateActions
              stageId={pipeline.current_stage}
              stageData={pipeline.stages[pipeline.current_stage]}
            />
          )}

          {/* Audit trail */}
          {auditEntries.length > 0 && (
            <section>
              <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider mb-3">
                Pipeline Audit Trail
              </h3>
              <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                  <thead className="bg-gray-50 dark:bg-gray-800">
                    <tr>
                      <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                        Time
                      </th>
                      <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                        Event
                      </th>
                      <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                        Stage
                      </th>
                      <th className="px-4 py-2 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                        Actor
                      </th>
                    </tr>
                  </thead>
                  <tbody className="bg-white dark:bg-gray-900 divide-y divide-gray-200 dark:divide-gray-700">
                    {auditEntries.slice(0, 20).map((entry) => (
                      <tr key={entry.id}>
                        <td className="px-4 py-2 text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap">
                          {new Date(entry.timestamp).toLocaleString()}
                        </td>
                        <td className="px-4 py-2 text-sm text-gray-900 dark:text-gray-100">
                          {entry.event}
                        </td>
                        <td className="px-4 py-2 text-sm text-gray-500 dark:text-gray-400">
                          {entry.stageId ?? "—"}
                        </td>
                        <td className="px-4 py-2 text-sm text-gray-500 dark:text-gray-400">
                          {entry.actor ?? "system"}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </section>
          )}
        </>
      )}
    </div>
  );
}

function GateActions({
  stageId,
  stageData,
}: {
  stageId: string;
  stageData?: StageData;
}) {
  const fetcher = useFetcher();
  const [showReject, setShowReject] = useState(false);

  const status = stageData?.status ?? "pending";
  const canAct = status === "completed" || status === "in_progress";

  if (!canAct) return null;

  return (
    <div className="border border-indigo-200 dark:border-indigo-800 rounded-lg p-4 bg-indigo-50 dark:bg-indigo-900/20">
      <h3 className="text-sm font-semibold text-indigo-900 dark:text-indigo-200 mb-2">
        Gate Approval: {stageId}
      </h3>
      <p className="text-sm text-indigo-700 dark:text-indigo-300 mb-4">
        This stage is awaiting review. Approve to advance the pipeline, or
        reject to halt.
      </p>

      <div className="flex gap-3">
        <fetcher.Form method="POST">
          <input type="hidden" name="intent" value="confirm" />
          <input type="hidden" name="stageId" value={stageId} />
          <button
            type="submit"
            disabled={fetcher.state !== "idle"}
            className="rounded-md bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700 disabled:opacity-50"
          >
            Approve Stage
          </button>
        </fetcher.Form>

        {!showReject ? (
          <button
            type="button"
            onClick={() => setShowReject(true)}
            className="rounded-md border border-red-300 dark:border-red-700 px-4 py-2 text-sm font-medium text-red-700 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
          >
            Reject
          </button>
        ) : (
          <fetcher.Form method="POST" className="flex gap-2 items-start">
            <input type="hidden" name="intent" value="reject" />
            <input type="hidden" name="stageId" value={stageId} />
            <input
              type="text"
              name="reason"
              placeholder="Rejection reason..."
              required
              className="rounded-md border border-gray-300 dark:border-gray-600 px-3 py-2 text-sm dark:bg-gray-800 dark:text-gray-100"
            />
            <button
              type="submit"
              disabled={fetcher.state !== "idle"}
              className="rounded-md bg-red-600 px-3 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:opacity-50"
            >
              Confirm Reject
            </button>
            <button
              type="button"
              onClick={() => setShowReject(false)}
              className="text-sm text-gray-500 hover:text-gray-700 dark:text-gray-400 px-2 py-2"
            >
              Cancel
            </button>
          </fetcher.Form>
        )}
      </div>
    </div>
  );
}

function CancelButton() {
  const fetcher = useFetcher();

  return (
    <fetcher.Form
      method="POST"
      onSubmit={(e) => {
        if (!confirm("Cancel this pipeline?")) {
          e.preventDefault();
        }
      }}
    >
      <input type="hidden" name="intent" value="cancel" />
      <button
        type="submit"
        disabled={fetcher.state !== "idle"}
        className="rounded-md border border-red-300 dark:border-red-700 px-3 py-2 text-sm font-medium text-red-700 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 disabled:opacity-50"
      >
        Cancel Pipeline
      </button>
    </fetcher.Form>
  );
}
