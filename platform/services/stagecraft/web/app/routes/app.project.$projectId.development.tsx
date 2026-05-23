import { useLoaderData, useFetcher, useOutletContext } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getFactoryStatus,
  listFactoryAudit,
  confirmFactoryStage,
  rejectFactoryStage,
  cancelPipeline,
  initFactoryPipeline,
  listKnowledgeObjects,
} from "../lib/project-api.server";
import { useState } from "react";

const KNOWN_ADAPTERS = [
  "aim-vue-node",
  "next-prisma",
  "rust-axum",
  "encore-react",
] as const;

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

  // Spec 119 dropped document_bindings — every project's knowledge object is
  // already scoped by `project_id`, so the init form just lists everything in
  // the project's corpus and lets the operator choose. There is no separate
  // "bound" set to reconcile.
  let availableKnowledge: Array<{
    id: string;
    filename: string;
    state: string;
  }> = [];
  const boundKnowledgeIds: string[] = [];
  if (!pipeline) {
    try {
      const kRes = await listKnowledgeObjects(request, params.projectId);
      availableKnowledge = (kRes.objects ?? []).map((o) => ({
        id: o.id,
        filename: o.filename,
        state: o.state,
      }));
    } catch {
      // knowledge surface unavailable — init form will render with no preselection.
    }
  }

  return {
    pipeline,
    auditEntries,
    availableKnowledge,
    boundKnowledgeIds,
  };
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

  if (intent === "init") {
    const adapter = form.get("adapter") as string;
    const knowledgeIds = form.getAll("knowledge_object_ids") as string[];
    if (!adapter) return { error: "adapter is required" };
    const res = await initFactoryPipeline(request, params.projectId, {
      adapter,
      actorUserId: user.userId,
      knowledge_object_ids: knowledgeIds.filter(Boolean),
    });
    return { initialized: res.pipeline_id };
  }

  if (intent === "confirm") {
    const stageId = form.get("stageId") as string;
    const notes = form.get("notes") as string | null;
    await confirmFactoryStage(
      request,
      params.projectId,
      stageId,
      user.userId,
      notes ?? undefined
    );
    return { confirmed: stageId };
  }

  if (intent === "reject") {
    const stageId = form.get("stageId") as string;
    const feedback = form.get("feedback") as string;
    await rejectFactoryStage(
      request,
      params.projectId,
      stageId,
      user.userId,
      feedback
    );
    return { rejected: stageId };
  }

  if (intent === "cancel") {
    const reason = form.get("reason") as string | null;
    await cancelPipeline(
      request,
      params.projectId,
      user.userId,
      reason ?? undefined
    );
    return { cancelled: true };
  }

  return null;
}

export default function PipelineDetail() {
  const {
    pipeline,
    auditEntries,
    availableKnowledge,
    boundKnowledgeIds,
  } = useLoaderData() as {
    pipeline: FactoryStatus | null;
    auditEntries: Array<{
      id: string;
      timestamp: string;
      event: string;
      actor: string | null;
      stageId: string | null;
      details: unknown;
    }>;
    availableKnowledge: Array<{ id: string; filename: string; state: string }>;
    boundKnowledgeIds: string[];
  };
  const { project } = useOutletContext<{
    project: { id: string; name: string; slug: string };
  }>();

  return (
    <div className="space-y-6">
      {!pipeline ? (
        <InitPipelineForm
          availableKnowledge={availableKnowledge}
          boundKnowledgeIds={boundKnowledgeIds}
        />
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
              name="feedback"
              placeholder="Rejection feedback..."
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

function InitPipelineForm({
  availableKnowledge,
  boundKnowledgeIds,
}: {
  availableKnowledge: Array<{ id: string; filename: string; state: string }>;
  boundKnowledgeIds: string[];
}) {
  const fetcher = useFetcher();
  const [adapter, setAdapter] = useState<string>(KNOWN_ADAPTERS[0]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(
    new Set(boundKnowledgeIds)
  );

  function toggle(id: string) {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  const submitting = fetcher.state !== "idle";

  return (
    <section className="border border-gray-200 dark:border-gray-700 rounded-lg p-5 bg-white dark:bg-gray-900">
      <header className="mb-4">
        <h2 className="text-base font-semibold text-gray-900 dark:text-gray-100">
          Initialize Factory Pipeline
        </h2>
        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
          Pick an adapter and the knowledge objects this run should consume.
          The pipeline runs in OPC desktop; progress streams back here.
        </p>
      </header>

      <fetcher.Form method="POST" className="space-y-4">
        <input type="hidden" name="intent" value="init" />

        <div>
          <label
            htmlFor="adapter"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
          >
            Adapter
          </label>
          <select
            id="adapter"
            name="adapter"
            value={adapter}
            onChange={(e) => setAdapter(e.target.value)}
            className="block w-full max-w-sm rounded-md border border-gray-300 dark:border-gray-600 px-3 py-2 text-sm dark:bg-gray-800 dark:text-gray-100"
          >
            {KNOWN_ADAPTERS.map((a) => (
              <option key={a} value={a}>
                {a}
              </option>
            ))}
          </select>
        </div>

        <div>
          <span className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Knowledge Objects
          </span>
          {availableKnowledge.length === 0 ? (
            <p className="text-sm text-gray-500 dark:text-gray-400">
              No knowledge objects in this project yet. Upload some from the{" "}
              <span className="font-medium">Knowledge</span> tab first.
            </p>
          ) : (
            <ul className="max-h-48 overflow-y-auto divide-y divide-gray-100 dark:divide-gray-800 border border-gray-200 dark:border-gray-700 rounded-md">
              {availableKnowledge.map((obj) => {
                const checked = selectedIds.has(obj.id);
                return (
                  <li
                    key={obj.id}
                    className="flex items-center gap-2 px-3 py-2 text-sm"
                  >
                    <input
                      type="checkbox"
                      name="knowledge_object_ids"
                      value={obj.id}
                      checked={checked}
                      onChange={() => toggle(obj.id)}
                      className="h-4 w-4"
                    />
                    <span className="text-gray-900 dark:text-gray-100 font-mono text-xs truncate">
                      {obj.filename}
                    </span>
                    <span className="ml-auto text-xs text-gray-500 dark:text-gray-400">
                      {obj.state}
                    </span>
                  </li>
                );
              })}
            </ul>
          )}
          <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
            {selectedIds.size} selected. Pre-selected items are currently
            bound to the project.
          </p>
        </div>

        {fetcher.data &&
          typeof fetcher.data === "object" &&
          "error" in fetcher.data &&
          fetcher.data.error ? (
          <p className="text-sm text-red-600 dark:text-red-400">
            {String(fetcher.data.error)}
          </p>
        ) : null}

        <div className="flex gap-3">
          <button
            type="submit"
            disabled={submitting}
            className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50"
          >
            {submitting ? "Starting…" : "Initialize Pipeline"}
          </button>
        </div>
      </fetcher.Form>
    </section>
  );
}
