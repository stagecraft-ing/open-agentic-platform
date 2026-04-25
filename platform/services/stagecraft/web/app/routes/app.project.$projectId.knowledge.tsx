import { useLoaderData, Link } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  listKnowledgeObjects,
  listConnectors,
} from "../lib/workspace-api.server";
import type {
  KnowledgeObjectRow,
  SourceConnectorRow,
} from "../lib/workspace-api.server";
import { useState, useRef } from "react";

export async function loader({
  request,
  params,
}: {
  request: Request;
  params: { projectId: string };
}) {
  await requireUser(request);

  const url = new URL(request.url);
  const stateFilter = url.searchParams.get("state") ?? undefined;

  const [koRes, connRes] = await Promise.all([
    listKnowledgeObjects(request, stateFilter),
    listConnectors(request).catch(() => ({ connectors: [] as SourceConnectorRow[] })),
  ]);

  return {
    projectId: params.projectId,
    objects: koRes.objects,
    connectors: connRes.connectors,
    stateFilter: stateFilter ?? "all",
  };
}

const STATES = ["all", "imported", "extracting", "extracted", "classified", "available"] as const;

const STATE_COLORS: Record<string, string> = {
  imported: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300",
  extracting: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300",
  extracted: "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300",
  classified: "bg-cyan-100 text-cyan-800 dark:bg-cyan-900/30 dark:text-cyan-300",
  available: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
};

export default function KnowledgeBrowser() {
  const { projectId, objects, connectors, stateFilter } = useLoaderData() as {
    projectId: string;
    objects: KnowledgeObjectRow[];
    connectors: SourceConnectorRow[];
    stateFilter: string;
  };
  const base = `/app/project/${projectId}/knowledge`;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Knowledge Objects
        </h2>
        <UploadButton />
      </div>

      {/* State filter tabs */}
      <div className="flex gap-1 border-b border-gray-200 dark:border-gray-700">
        {STATES.map((state) => {
          const isActive = stateFilter === state;
          const count =
            state === "all"
              ? objects.length
              : objects.filter((o) => o.state === state).length;
          return (
            <Link
              key={state}
              to={state === "all" ? base : `${base}?state=${state}`}
              className={`px-3 py-2 text-sm font-medium border-b-2 transition-colors ${
                isActive
                  ? "border-indigo-500 text-indigo-600 dark:text-indigo-400"
                  : "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400"
              }`}
            >
              {state}
              <span className="ml-1 text-xs text-gray-400">({count})</span>
            </Link>
          );
        })}
      </div>

      {/* Object list */}
      {objects.length === 0 ? (
        <div className="border border-dashed border-gray-300 dark:border-gray-600 rounded-lg px-4 py-12 text-center">
          <p className="text-sm text-gray-500 dark:text-gray-400 mb-2">
            No knowledge objects
            {stateFilter !== "all" ? ` in "${stateFilter}" state` : ""}.
          </p>
          <p className="text-sm text-gray-400 dark:text-gray-500">
            Upload documents to start building your workspace knowledge base.
          </p>
        </div>
      ) : (
        <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
          <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
            <thead className="bg-gray-50 dark:bg-gray-800">
              <tr>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  File
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Type
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Size
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  State
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Source
                </th>
                <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Imported
                </th>
              </tr>
            </thead>
            <tbody className="bg-white dark:bg-gray-900 divide-y divide-gray-200 dark:divide-gray-700">
              {objects.map((obj) => (
                <tr
                  key={obj.id}
                  className="hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
                >
                  <td className="px-4 py-3">
                    <Link
                      to={`${base}/${obj.id}`}
                      className="text-sm font-medium text-gray-900 dark:text-gray-100 hover:text-indigo-600 dark:hover:text-indigo-400"
                    >
                      {obj.filename}
                    </Link>
                  </td>
                  <td className="px-4 py-3 text-sm text-gray-500 dark:text-gray-400">
                    {obj.mimeType}
                  </td>
                  <td className="px-4 py-3 text-sm text-gray-500 dark:text-gray-400">
                    {formatBytes(obj.sizeBytes)}
                  </td>
                  <td className="px-4 py-3">
                    <span
                      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${STATE_COLORS[obj.state] ?? "bg-gray-100 text-gray-800"}`}
                    >
                      {obj.state}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-sm text-gray-500 dark:text-gray-400">
                    {obj.provenance?.sourceType ?? "unknown"}
                  </td>
                  <td className="px-4 py-3 text-sm text-gray-500 dark:text-gray-400">
                    {new Date(obj.createdAt).toLocaleDateString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Connectors summary */}
      {connectors.length > 0 && (
        <section>
          <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider mb-3">
            Source Connectors
          </h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            {connectors.map((c) => (
              <div
                key={c.id}
                className="border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-3 bg-white dark:bg-gray-900"
              >
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
                    {c.name}
                  </span>
                  <span className="text-xs text-gray-500 dark:text-gray-400 bg-gray-100 dark:bg-gray-800 px-2 py-0.5 rounded">
                    {c.type}
                  </span>
                </div>
                <div className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                  Status: {c.status}
                  {c.lastSyncedAt &&
                    ` · Last sync: ${new Date(c.lastSyncedAt).toLocaleDateString()}`}
                </div>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

/**
 * Upload button with client-side file handling.
 * Flow: select file → compute SHA-256 → request presigned URL → PUT to S3 → confirm.
 */
function UploadButton() {
  const fileRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [uploadError, setUploadError] = useState<string | null>(null);

  async function handleFileSelect(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;

    setUploading(true);
    setUploadError(null);

    try {
      const buffer = await file.arrayBuffer();
      const hashBuffer = await crypto.subtle.digest("SHA-256", buffer);
      const hashArray = Array.from(new Uint8Array(hashBuffer));
      const contentHash = hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");

      // Call the Encore API directly. Going through the Remix action handler
      // returns HTML in React Router v7 single-fetch (the action's JSON is
      // only available on `.data` URLs), and `await res.json()` on HTML
      // throws "The string did not match the expected pattern" on Safari.
      // Same-origin fetch carries the __session cookie automatically.
      const reqUploadRes = await fetch("/api/knowledge/upload", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          filename: file.name,
          mimeType: file.type || "application/octet-stream",
          contentHash,
        }),
      });

      if (!reqUploadRes.ok) {
        const body = await reqUploadRes.text();
        throw new Error(
          `Failed to request upload (${reqUploadRes.status}): ${body.slice(0, 200)}`
        );
      }

      const { uploadUrl, objectId } = (await reqUploadRes.json()) as {
        uploadUrl: string;
        objectId: string;
      };

      const s3Res = await fetch(uploadUrl, {
        method: "PUT",
        body: buffer,
      });

      if (!s3Res.ok) {
        throw new Error(`S3 upload failed: ${s3Res.status} ${s3Res.statusText}`);
      }

      const confirmRes = await fetch(
        `/api/knowledge/objects/${objectId}/confirm`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: "{}",
        }
      );

      if (!confirmRes.ok) {
        const body = await confirmRes.text();
        throw new Error(
          `Upload landed but confirm failed (${confirmRes.status}): ${body.slice(0, 200)}`
        );
      }

      window.location.reload();
    } catch (err) {
      setUploadError(err instanceof Error ? err.message : "Upload failed");
    } finally {
      setUploading(false);
      if (fileRef.current) fileRef.current.value = "";
    }
  }

  return (
    <div className="relative">
      <input
        ref={fileRef}
        type="file"
        className="hidden"
        onChange={handleFileSelect}
      />
      <button
        type="button"
        disabled={uploading}
        onClick={() => fileRef.current?.click()}
        className="inline-flex items-center gap-2 rounded-md bg-indigo-600 px-3 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {uploading ? (
          <>
            <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent" />
            Uploading...
          </>
        ) : (
          "Upload Document"
        )}
      </button>
      {uploadError && (
        <p className="absolute top-full right-0 mt-1 text-xs text-red-600 dark:text-red-400">
          {uploadError}
        </p>
      )}
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
