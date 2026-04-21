import { useLoaderData, useFetcher, Link } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getKnowledgeObject,
  getDownloadUrl,
  transitionKnowledgeState,
  deleteKnowledgeObject,
} from "../lib/workspace-api.server";
import type { KnowledgeObjectRow } from "../lib/workspace-api.server";
import { redirect } from "react-router";

export async function loader({
  request,
  params,
}: {
  request: Request;
  params: { projectId: string; id: string };
}) {
  await requireUser(request);
  const { object } = await getKnowledgeObject(request, params.id);
  return { object, projectId: params.projectId };
}

export async function action({
  request,
  params,
}: {
  request: Request;
  params: { projectId: string; id: string };
}) {
  await requireUser(request);
  const form = await request.formData();
  const intent = form.get("intent");

  if (intent === "transition") {
    const targetState = form.get("targetState") as string;
    const res = await transitionKnowledgeState(request, params.id, {
      targetState,
    });
    return { object: res.object };
  }

  if (intent === "download") {
    const res = await getDownloadUrl(request, params.id);
    return { downloadUrl: res.downloadUrl };
  }

  if (intent === "delete") {
    await deleteKnowledgeObject(request, params.id);
    return redirect(`/app/project/${params.projectId}/knowledge`);
  }

  return null;
}

const VALID_TRANSITIONS: Record<string, string> = {
  imported: "extracting",
  extracting: "extracted",
  extracted: "classified",
  classified: "available",
};

const STATE_COLORS: Record<string, string> = {
  imported: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300",
  extracting: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300",
  extracted: "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300",
  classified: "bg-cyan-100 text-cyan-800 dark:bg-cyan-900/30 dark:text-cyan-300",
  available: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
};

const STATE_ORDER = ["imported", "extracting", "extracted", "classified", "available"];

export default function KnowledgeObjectDetail() {
  const { object, projectId } = useLoaderData() as {
    object: KnowledgeObjectRow;
    projectId: string;
  };
  const fetcher = useFetcher();
  const downloadFetcher = useFetcher();

  const nextState = VALID_TRANSITIONS[object.state];
  const canDelete = object.state !== "available";
  const provenance = object.provenance as {
    sourceType?: string;
    sourceUri?: string;
    importedAt?: string;
  };

  const downloadUrl = (downloadFetcher.data as any)?.downloadUrl;

  return (
    <div className="max-w-3xl space-y-6">
      {/* Breadcrumb */}
      <nav className="text-sm text-gray-500 dark:text-gray-400">
        <Link
          to={`/app/project/${projectId}/knowledge`}
          className="hover:text-gray-700 dark:hover:text-gray-300"
        >
          Knowledge
        </Link>
        <span className="mx-1">/</span>
        <span className="text-gray-900 dark:text-gray-100">
          {object.filename}
        </span>
      </nav>

      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">
            {object.filename}
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            {object.mimeType} &middot; {formatBytes(object.sizeBytes)}
          </p>
        </div>
        <span
          className={`inline-flex items-center px-3 py-1 rounded-full text-sm font-medium ${STATE_COLORS[object.state] ?? "bg-gray-100 text-gray-800"}`}
        >
          {object.state}
        </span>
      </div>

      {/* State progress bar */}
      <div className="flex items-center gap-1">
        {STATE_ORDER.map((state, i) => {
          const currentIdx = STATE_ORDER.indexOf(object.state);
          const isPast = i <= currentIdx;
          return (
            <div key={state} className="flex-1 flex flex-col items-center gap-1">
              <div
                className={`h-2 w-full rounded-full ${
                  isPast
                    ? "bg-indigo-500"
                    : "bg-gray-200 dark:bg-gray-700"
                }`}
              />
              <span
                className={`text-xs ${
                  isPast
                    ? "text-gray-900 dark:text-gray-100 font-medium"
                    : "text-gray-400 dark:text-gray-500"
                }`}
              >
                {state}
              </span>
            </div>
          );
        })}
      </div>

      {/* Actions */}
      <div className="flex gap-3">
        {nextState && (
          <fetcher.Form method="POST">
            <input type="hidden" name="intent" value="transition" />
            <input type="hidden" name="targetState" value={nextState} />
            <button
              type="submit"
              disabled={fetcher.state !== "idle"}
              className="rounded-md bg-indigo-600 px-3 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50"
            >
              Advance to {nextState}
            </button>
          </fetcher.Form>
        )}

        <downloadFetcher.Form method="POST">
          <input type="hidden" name="intent" value="download" />
          <button
            type="submit"
            disabled={downloadFetcher.state !== "idle"}
            className="rounded-md border border-gray-300 dark:border-gray-600 px-3 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-800 disabled:opacity-50"
          >
            Get Download Link
          </button>
        </downloadFetcher.Form>

        {canDelete && (
          <fetcher.Form
            method="POST"
            onSubmit={(e) => {
              if (!confirm("Delete this knowledge object?")) {
                e.preventDefault();
              }
            }}
          >
            <input type="hidden" name="intent" value="delete" />
            <button
              type="submit"
              className="rounded-md border border-red-300 dark:border-red-700 px-3 py-2 text-sm font-medium text-red-700 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
            >
              Delete
            </button>
          </fetcher.Form>
        )}
      </div>

      {downloadUrl && (
        <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-3">
          <p className="text-sm text-green-800 dark:text-green-300">
            Download link ready (expires in 1 hour):
          </p>
          <a
            href={downloadUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-indigo-600 dark:text-indigo-400 hover:underline break-all"
          >
            Download {object.filename}
          </a>
        </div>
      )}

      {/* Metadata */}
      <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
        <dl className="divide-y divide-gray-200 dark:divide-gray-700">
          <MetaRow label="ID" value={object.id} />
          <MetaRow label="Storage Key" value={object.storageKey} />
          <MetaRow label="Content Hash (SHA-256)" value={object.contentHash} />
          <MetaRow label="Source Type" value={provenance?.sourceType ?? "—"} />
          <MetaRow label="Source URI" value={provenance?.sourceUri ?? "—"} />
          <MetaRow
            label="Imported At"
            value={
              provenance?.importedAt
                ? new Date(provenance.importedAt).toLocaleString()
                : "—"
            }
          />
          <MetaRow
            label="Created"
            value={new Date(object.createdAt).toLocaleString()}
          />
          <MetaRow
            label="Updated"
            value={new Date(object.updatedAt).toLocaleString()}
          />
        </dl>
      </div>

      {/* Extraction output */}
      {object.extractionOutput && (
        <section>
          <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider mb-2">
            Extraction Output
          </h3>
          <pre className="bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 text-xs text-gray-700 dark:text-gray-300 overflow-auto max-h-64">
            {JSON.stringify(object.extractionOutput, null, 2)}
          </pre>
        </section>
      )}

      {/* Classification */}
      {object.classification && (
        <section>
          <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider mb-2">
            Classification
          </h3>
          <div className="flex gap-2 flex-wrap">
            {(Array.isArray(object.classification)
              ? object.classification
              : [object.classification]
            ).map((tag: string, i: number) => (
              <span
                key={i}
                className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200"
              >
                {String(tag)}
              </span>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function MetaRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="px-4 py-3 sm:grid sm:grid-cols-3 sm:gap-4">
      <dt className="text-sm font-medium text-gray-500 dark:text-gray-400">
        {label}
      </dt>
      <dd className="mt-1 text-sm text-gray-900 dark:text-gray-100 sm:mt-0 sm:col-span-2 break-all">
        {value}
      </dd>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}
