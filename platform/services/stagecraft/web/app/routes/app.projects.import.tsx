/**
 * Factory project import (spec 112 §6).
 *
 * Accepts a GitHub repo URL, detects factory state server-side, and
 * either registers the repo (ACP-native) or rejects with an
 * actionable error (not factory / scaffold-only / legacy incomplete).
 * On L1 import, returns the translator's preview for the user to
 * confirm before the platform opens a PR adding
 * `.factory/pipeline-state.json`.
 */

import { Form, useActionData, useNavigation } from "react-router";
import { useState } from "react";
import { requireUser } from "../lib/auth.server";
import { importFactoryProject } from "../lib/projects-api.server";

interface ActionSuccess {
  projectId: string | null;
  detectionLevel:
    | "not_factory"
    | "scaffold_only"
    | "legacy_produced"
    | "acp_produced";
  repoUrl: string;
  cloneUrl: string;
  oapDeepLink: string | null;
  translatorVersion: string | null;
  translatedPreview?: Record<string, unknown>;
  previewOnly: boolean;
}

interface ActionFailure {
  error: string;
}

type ActionResult = ActionSuccess | ActionFailure;

export async function loader({ request }: { request: Request }) {
  await requireUser(request);
  return {};
}

export async function action({ request }: { request: Request }): Promise<ActionResult> {
  const user = await requireUser(request);
  const formData = await request.formData();
  const repoUrl = (formData.get("repoUrl") as string | null) ?? "";
  const name = (formData.get("name") as string | null) ?? "";
  const slug = (formData.get("slug") as string | null) ?? "";
  const description = (formData.get("description") as string | null) ?? "";
  const previewOnly = formData.get("action") === "preview";

  if (!repoUrl) {
    return { error: "A GitHub repo URL is required." };
  }

  try {
    const result = await importFactoryProject(request, {
      repoUrl,
      name: name || undefined,
      slug: slug || undefined,
      description: description || undefined,
      previewOnly,
    });
    return result;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error("importFactoryProject failed", {
      userId: user.userId,
      repoUrl,
      error: msg,
    });
    let backendMsg = msg;
    try {
      const parsed = JSON.parse(msg) as { message?: string };
      if (parsed.message) backendMsg = parsed.message;
    } catch {
      /* not JSON */
    }
    return { error: backendMsg };
  }
}

function isSuccess(data: ActionResult | undefined): data is ActionSuccess {
  return Boolean(data && (data as ActionSuccess).detectionLevel);
}

export default function ImportProject() {
  const actionData = useActionData() as ActionResult | undefined;
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";
  const [repoUrl, setRepoUrl] = useState("");
  const [name, setName] = useState("");

  if (isSuccess(actionData) && !actionData.previewOnly) {
    return <ImportRegistered data={actionData} />;
  }

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Import Existing Project
        </h2>
        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
          Register a factory-produced GitHub repo with this workspace. The
          platform detects factory state before creating any rows.
        </p>
      </div>

      {actionData && !isSuccess(actionData) && (
        <div className="rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 px-4 py-3">
          <p className="text-sm text-red-700 dark:text-red-400">
            {(actionData as ActionFailure).error}
          </p>
        </div>
      )}

      {isSuccess(actionData) && actionData.previewOnly && (
        <ImportPreviewPanel data={actionData} />
      )}

      <Form method="post" className="space-y-5">
        <div>
          <label htmlFor="repoUrl" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            GitHub Repo URL
          </label>
          <input
            type="text"
            name="repoUrl"
            id="repoUrl"
            required
            value={repoUrl}
            onChange={(e) => setRepoUrl(e.target.value)}
            placeholder="https://github.com/acme/my-project"
            className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm font-mono text-gray-900 dark:text-gray-100 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
          />
          <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
            The repo must belong to a GitHub org your OAP App installation has access to.
          </p>
        </div>

        <div>
          <label htmlFor="name" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Project Name (optional)
          </label>
          <input
            type="text"
            name="name"
            id="name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Defaults to the repo name"
            className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-gray-100 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
          />
        </div>

        <div>
          <label htmlFor="slug" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Slug (optional)
          </label>
          <input
            type="text"
            name="slug"
            id="slug"
            placeholder="Defaults to the repo name lowercased"
            className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm font-mono text-gray-900 dark:text-gray-100 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
          />
        </div>

        <div>
          <label htmlFor="description" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Description
          </label>
          <textarea
            name="description"
            id="description"
            rows={2}
            className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-gray-100 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
          />
        </div>

        <div className="flex items-center gap-3 pt-2">
          <button
            type="submit"
            name="action"
            value="preview"
            disabled={isSubmitting}
            className="inline-flex items-center rounded-md border border-gray-300 dark:border-gray-600 px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50"
          >
            {isSubmitting ? "Inspecting…" : "Detect only"}
          </button>
          <button
            type="submit"
            name="action"
            value="import"
            disabled={isSubmitting}
            className="inline-flex items-center rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-50"
          >
            {isSubmitting ? "Importing…" : "Import project"}
          </button>
          <a
            href="/app"
            className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
          >
            Cancel
          </a>
        </div>
      </Form>
    </div>
  );
}

function ImportPreviewPanel({ data }: { data: ActionSuccess }) {
  return (
    <div className="rounded-md border border-gray-200 dark:border-gray-700 p-4 space-y-2">
      <div className="text-sm">
        <strong>Detection:</strong> <code>{data.detectionLevel}</code>
        {data.translatorVersion && (
          <span className="ml-3 text-xs text-gray-500 dark:text-gray-400">
            translator {data.translatorVersion}
          </span>
        )}
      </div>
      {data.translatedPreview && (
        <pre className="bg-gray-50 dark:bg-gray-800 rounded p-2 text-xs overflow-x-auto max-h-64">
          {JSON.stringify(data.translatedPreview, null, 2)}
        </pre>
      )}
      <p className="text-xs text-gray-500 dark:text-gray-400">
        Press "Import project" to register and open the translation PR.
      </p>
    </div>
  );
}

function ImportRegistered({ data }: { data: ActionSuccess }) {
  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div className="rounded-md bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 px-4 py-3">
        <p className="text-sm text-green-800 dark:text-green-300">
          Project imported. Detection level: <code>{data.detectionLevel}</code>.
          {data.translatorVersion && (
            <>
              {" "}
              Translated via <code>{data.translatorVersion}</code>.
            </>
          )}
        </p>
      </div>
      <dl className="rounded-md border border-gray-200 dark:border-gray-700 divide-y divide-gray-200 dark:divide-gray-700">
        <div className="grid grid-cols-[10rem_1fr] px-4 py-3">
          <dt className="text-sm font-medium text-gray-500 dark:text-gray-400">Repo</dt>
          <dd className="text-sm text-indigo-600 dark:text-indigo-400">
            <a href={data.repoUrl} target="_blank" rel="noreferrer">
              {data.repoUrl}
            </a>
          </dd>
        </div>
        <div className="grid grid-cols-[10rem_1fr] px-4 py-3">
          <dt className="text-sm font-medium text-gray-500 dark:text-gray-400">Open in OPC</dt>
          <dd>
            {data.oapDeepLink ? (
              <a
                href={data.oapDeepLink}
                className="inline-flex items-center rounded-md bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-indigo-700"
              >
                Launch Factory Cockpit
              </a>
            ) : (
              <span className="text-sm text-gray-500 dark:text-gray-400">
                (preview only — no deep link)
              </span>
            )}
          </dd>
        </div>
      </dl>
      {data.projectId && (
        <a
          href={`/app/project/${data.projectId}`}
          className="text-sm text-indigo-600 dark:text-indigo-400"
        >
          Go to project dashboard →
        </a>
      )}
    </div>
  );
}
