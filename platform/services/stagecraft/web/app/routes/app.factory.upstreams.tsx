/**
 * Factory Upstreams form (spec 108 Phase 2).
 *
 * Admin-only writable config. All org members can view the current
 * configuration; only owners/admins can submit changes. On success the
 * Encore action returns the persisted row and the UI reloads the loader.
 */

import { Form, redirect, useActionData, useLoaderData, useNavigation } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getFactoryUpstreams,
  upsertFactoryUpstreams,
  type FactoryUpstream,
} from "../lib/factory-api.server";

type LoaderData = {
  upstream: FactoryUpstream | null;
  canConfigure: boolean;
};

export async function loader({ request }: { request: Request }): Promise<LoaderData> {
  const user = await requireUser(request);
  const { upstream } = await getFactoryUpstreams(request);
  const canConfigure =
    user.platformRole === "owner" || user.platformRole === "admin";
  return { upstream, canConfigure };
}

type ActionData = { error?: string; ok?: true };

export async function action({ request }: { request: Request }): Promise<ActionData | Response> {
  const user = await requireUser(request);
  if (user.platformRole !== "owner" && user.platformRole !== "admin") {
    return { error: "Only org admins can configure factory upstreams." };
  }

  const form = await request.formData();
  const factorySource = (form.get("factorySource") as string | null)?.trim() ?? "";
  const templateSource = (form.get("templateSource") as string | null)?.trim() ?? "";
  const factoryRef = (form.get("factoryRef") as string | null)?.trim() ?? "main";
  const templateRef = (form.get("templateRef") as string | null)?.trim() ?? "main";

  if (!factorySource || !templateSource) {
    return { error: "Both factory and template sources are required." };
  }

  try {
    await upsertFactoryUpstreams(request, {
      factorySource,
      factoryRef,
      templateSource,
      templateRef,
    });
    return redirect("/app/factory");
  } catch (err) {
    return {
      error: err instanceof Error ? err.message : "Failed to save upstreams.",
    };
  }
}

export default function UpstreamsForm() {
  const { upstream, canConfigure } = useLoaderData<typeof loader>();
  const actionData = useActionData<ActionData>();
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";

  return (
    <div className="max-w-2xl space-y-6">
      <div>
        <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider">
          Upstream configuration
        </h2>
        <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
          Point Factory at the two GitHub repositories that feed your
          adapters, contracts, and processes. Changes take effect on the
          next sync.
        </p>
      </div>

      {!canConfigure && (
        <div className="rounded-md bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 px-4 py-3 text-sm text-amber-800 dark:text-amber-300">
          You can view the current configuration but only org admins can change it.
        </div>
      )}

      {actionData?.error && (
        <div className="rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 px-4 py-3 text-sm text-red-700 dark:text-red-400">
          {actionData.error}
        </div>
      )}

      <Form method="post" className="space-y-6">
        <RepoField
          name="factorySource"
          refName="factoryRef"
          label="Factory source"
          hint="Canonical process definitions and adapter scaffolds."
          placeholder="GovAlta-Pronghorn/goa-software-factory"
          defaultRepo={upstream?.factorySource ?? ""}
          defaultRef={upstream?.factoryRef ?? "main"}
          disabled={!canConfigure}
        />

        <RepoField
          name="templateSource"
          refName="templateRef"
          label="Template source"
          hint="Per-project templates consumed by the factory."
          placeholder="GovAlta-Pronghorn/template"
          defaultRepo={upstream?.templateSource ?? ""}
          defaultRef={upstream?.templateRef ?? "main"}
          disabled={!canConfigure}
        />

        <div className="flex items-center gap-3">
          <button
            type="submit"
            disabled={!canConfigure || isSubmitting}
            className="inline-flex justify-center rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSubmitting ? "Saving…" : upstream ? "Save changes" : "Configure"}
          </button>
          <a
            href="/app/factory"
            className="text-sm text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
          >
            Cancel
          </a>
        </div>
      </Form>
    </div>
  );
}

function RepoField({
  name,
  refName,
  label,
  hint,
  placeholder,
  defaultRepo,
  defaultRef,
  disabled,
}: {
  name: string;
  refName: string;
  label: string;
  hint: string;
  placeholder: string;
  defaultRepo: string;
  defaultRef: string;
  disabled: boolean;
}) {
  return (
    <fieldset className="space-y-3 rounded-lg border border-gray-200 dark:border-gray-700 p-4">
      <legend className="px-1 text-sm font-medium text-gray-700 dark:text-gray-300">
        {label}
      </legend>
      <div>
        <label className="block text-xs font-medium text-gray-600 dark:text-gray-400">
          Repository (owner/repo)
        </label>
        <input
          type="text"
          name={name}
          required
          disabled={disabled}
          defaultValue={defaultRepo}
          placeholder={placeholder}
          className="mt-1 block w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-mono shadow-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 disabled:cursor-not-allowed disabled:bg-gray-50 disabled:text-gray-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:disabled:bg-gray-800/60"
        />
      </div>
      <div>
        <label className="block text-xs font-medium text-gray-600 dark:text-gray-400">
          Git ref
        </label>
        <input
          type="text"
          name={refName}
          disabled={disabled}
          defaultValue={defaultRef}
          placeholder="main"
          className="mt-1 block w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-mono shadow-sm focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500 disabled:cursor-not-allowed disabled:bg-gray-50 disabled:text-gray-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:disabled:bg-gray-800/60"
        />
      </div>
      <p className="text-xs text-gray-500 dark:text-gray-400">{hint}</p>
    </fieldset>
  );
}
