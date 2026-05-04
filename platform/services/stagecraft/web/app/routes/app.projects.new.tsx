/**
 * Self-service project creation (spec 080 FR-006 → spec 112 §5.1).
 *
 * Lists factory adapters from the org's `factory_adapters` table and creates
 * the project through the ACP-native `/api/projects/factory-create`
 * endpoint, which writes commit #1 with a `.factory/pipeline-state.json`
 * L0 seed and returns an `opc://` deep link for the success page to
 * hand off to OPC.
 *
 * Gating (spec 112 Phase 5): the loader fetches `/api/projects/scaffold-readiness`
 * alongside the adapter list. If warmup is in progress, no adapter exists,
 * or no upstream PAT is configured, the form renders a banner and disables
 * submit instead of letting the user hit a 500.
 */

import {
  Form,
  useActionData,
  useLoaderData,
  useNavigation,
} from "react-router";
import { useState } from "react";
import { requireUser } from "../lib/auth.server";
import {
  createFactoryProject,
  listFactoryAdapters,
  getScaffoldReadiness,
  type ScaffoldReadiness,
} from "../lib/projects-api.server";

type Variant = "single-public" | "single-internal" | "dual";

const VARIANTS: Array<{ value: Variant; label: string; description: string }> = [
  {
    value: "single-public",
    label: "Single (public)",
    description: "One stack served to citizens / external users.",
  },
  {
    value: "single-internal",
    label: "Single (internal)",
    description: "One stack served to staff / internal users.",
  },
  {
    value: "dual",
    label: "Dual",
    description: "Separate public + internal stacks with a BFF boundary.",
  },
];

// Mirror of platform/services/stagecraft/api/projects/scaffold/moduleCatalog.ts.
// Kept inline to avoid a shared client/server dep on the api/ tree (Encore.ts
// codegen would surface it as well, but this UI is small enough that
// duplication beats coupling).
interface ModuleDescriptor {
  id: string;
  displayName: string;
  category: string;
  description: string;
  requires: string[];
  conflicts: string[];
}

const MODULE_CATALOG: ModuleDescriptor[] = [
  {
    id: "auth-saml",
    displayName: "SAML 2.0",
    category: "Authentication",
    description: "Alberta.ca Account auth (public/citizen-facing apps)",
    requires: [],
    conflicts: ["auth-entra-id"],
  },
  {
    id: "auth-entra-id",
    displayName: "Entra ID",
    category: "Authentication",
    description: "Microsoft Entra ID / Azure AD (staff-facing apps)",
    requires: [],
    conflicts: ["auth-saml"],
  },
  {
    id: "data-redis",
    displayName: "Redis",
    category: "Data Access",
    description: "Redis client with access-key and Entra ID auth modes",
    requires: [],
    conflicts: [],
  },
  {
    id: "data-postgres",
    displayName: "PostgreSQL",
    category: "Data Access",
    description: "PostgreSQL pool with Azure compliance (SSL, retry, metrics)",
    requires: [],
    conflicts: [],
  },
  {
    id: "session-store-redis",
    displayName: "Redis Sessions",
    category: "Session Store",
    description: "Redis session store for express-session",
    requires: ["data-redis"],
    conflicts: ["session-store-postgres"],
  },
  {
    id: "session-store-postgres",
    displayName: "PostgreSQL Sessions",
    category: "Session Store",
    description: "PostgreSQL session store for express-session",
    requires: ["data-postgres"],
    conflicts: ["session-store-redis"],
  },
  {
    id: "service-auth",
    displayName: "Service Auth",
    category: "Infrastructure",
    description:
      "Azure AD service-to-service JWT validation (Client Credentials flow)",
    requires: [],
    conflicts: [],
  },
  {
    id: "api-gateway",
    displayName: "API Gateway",
    category: "Infrastructure",
    description: "BFF gateway/proxy layer for routing requests",
    requires: [],
    conflicts: [],
  },
  {
    id: "api-docs",
    displayName: "API Docs",
    category: "Infrastructure",
    description: "OpenAPI/Swagger documentation UI served at /api-docs",
    requires: [],
    conflicts: [],
  },
  {
    id: "user-management",
    displayName: "User/Role Management",
    category: "Application",
    description:
      "Admin UI for user and role management with IdP-to-DB sync on login",
    requires: ["data-postgres"],
    conflicts: [],
  },
];

const PRESETS: Record<Variant, string[]> = {
  "single-public": [
    "data-redis",
    "auth-saml",
    "session-store-redis",
    "api-gateway",
  ],
  "single-internal": [
    "data-postgres",
    "auth-entra-id",
    "session-store-postgres",
    "service-auth",
  ],
  // Dual modules are managed by setup-dual-app.ts; the picker is hidden
  // for variant=dual, so the empty preset is informational only.
  dual: [],
};

const MODULES_BY_CATEGORY = MODULE_CATALOG.reduce<Record<string, ModuleDescriptor[]>>(
  (acc, m) => {
    (acc[m.category] ??= []).push(m);
    return acc;
  },
  {}
);

interface AdapterSummary {
  id: string;
  name: string;
  version: string;
}

interface LoaderData {
  adapters: AdapterSummary[];
  readiness: ScaffoldReadiness;
}

interface ActionSuccess {
  projectId: string;
  repoUrl: string;
  cloneUrl: string;
  opcDeepLink: string;
  factoryAdapterId: string;
  devEnvironmentId: string;
  profile: string;
}

interface ActionFailure {
  error: string;
}

type ActionResult = ActionSuccess | ActionFailure;

export async function loader({
  request,
}: {
  request: Request;
}): Promise<LoaderData> {
  await requireUser(request);

  const [adapterListResult, readinessResult] = await Promise.allSettled([
    listFactoryAdapters(request),
    getScaffoldReadiness(request),
  ]);

  const adapters =
    adapterListResult.status === "fulfilled"
      ? adapterListResult.value.adapters
          .filter((a): a is AdapterSummary & { id: string } => Boolean(a.id))
          .map((a) => ({ id: a.id, name: a.name, version: a.version }))
      : [];

  const readiness: ScaffoldReadiness =
    readinessResult.status === "fulfilled"
      ? readinessResult.value
      : {
          ready: false,
          step: "error",
          progress: 0,
          hasFactoryAdapter: adapters.length > 0,
          hasUpstreamPat: false,
          hasTemplateRemote: false,
          canCreate: false,
          blocker: "warmup-error",
          error:
            readinessResult.status === "rejected"
              ? readinessResult.reason instanceof Error
                ? readinessResult.reason.message
                : String(readinessResult.reason)
              : "scaffold-readiness lookup failed",
        };

  return { adapters, readiness };
}

export async function action({
  request,
}: {
  request: Request;
}): Promise<ActionResult> {
  const user = await requireUser(request);
  const formData = await request.formData();

  const name = (formData.get("name") as string | null) ?? "";
  const slug = (formData.get("slug") as string | null) ?? "";
  const description = (formData.get("description") as string | null) ?? "";
  const adapterId = (formData.get("adapterId") as string | null) ?? "";
  const variant = (formData.get("variant") as string | null) ?? "dual";
  const repoName = (formData.get("repoName") as string | null) ?? "";
  const isPrivate = formData.get("visibility") !== "public";
  const modules = formData.getAll("modules").map(String);

  if (!name || !slug || !adapterId || !repoName) {
    return { error: "Name, slug, adapter, and repository name are required." };
  }
  if (slug.length < 3 || !/^[a-z0-9][a-z0-9-]*[a-z0-9]$/.test(slug)) {
    return {
      error:
        "Slug must be at least 3 characters, lowercase alphanumeric with hyphens (e.g., my-project).",
    };
  }
  if (!["single-public", "single-internal", "dual"].includes(variant)) {
    return { error: "Invalid variant selected." };
  }

  try {
    const result = await createFactoryProject(request, {
      name,
      slug,
      description: description || undefined,
      adapterId,
      variant: variant as Variant,
      modules: variant === "dual" ? [] : modules,
      repoName,
      isPrivate,
    });
    return result;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error("createFactoryProject failed", {
      userId: user.userId,
      orgSlug: user.orgSlug,
      slug,
      repoName,
      adapterId,
      error: msg,
    });
    let backendMsg = msg;
    try {
      const parsed = JSON.parse(msg) as { message?: string };
      if (parsed.message) backendMsg = parsed.message;
    } catch {
      /* leave msg alone */
    }
    return { error: backendMsg };
  }
}

function slugify(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

function isSuccess(data: ActionResult | undefined): data is ActionSuccess {
  return Boolean(data && (data as ActionSuccess).projectId);
}

export default function NewProject() {
  const { adapters, readiness } = useLoaderData() as LoaderData;
  const actionData = useActionData() as ActionResult | undefined;
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [repoName, setRepoName] = useState("");
  const [slugEdited, setSlugEdited] = useState(false);
  const [repoEdited, setRepoEdited] = useState(false);
  const [variant, setVariant] = useState<Variant>("dual");
  const [selectedModules, setSelectedModules] = useState<Set<string>>(
    () => new Set(PRESETS["dual"])
  );

  const handleNameChange = (value: string) => {
    setName(value);
    const derived = slugify(value);
    if (!slugEdited) setSlug(derived);
    if (!repoEdited) setRepoName(slugEdited ? slug : derived);
  };

  const handleVariantChange = (next: Variant) => {
    setVariant(next);
    setSelectedModules(new Set(PRESETS[next]));
  };

  const toggleModule = (id: string, checked: boolean) => {
    setSelectedModules((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(id);
        // Auto-add required deps.
        const mod = MODULE_CATALOG.find((m) => m.id === id);
        for (const dep of mod?.requires ?? []) next.add(dep);
        // Drop conflicts.
        for (const c of mod?.conflicts ?? []) next.delete(c);
      } else {
        next.delete(id);
        // Drop modules that depend on this one.
        for (const m of MODULE_CATALOG) {
          if (m.requires.includes(id)) next.delete(m.id);
        }
      }
      return next;
    });
  };

  if (isSuccess(actionData)) {
    return <CreateSuccess data={actionData} />;
  }

  const banner = renderReadinessBanner(readiness);
  const submitDisabled = isSubmitting || !readiness.canCreate;

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Create New Project
        </h2>
        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
          Scaffold a new project with a factory adapter, GitHub repo, and
          pre-seeded ACP pipeline state.
        </p>
      </div>

      {banner}

      {actionData && !isSuccess(actionData) && (
        <div className="rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 px-4 py-3">
          <p className="text-sm text-red-700 dark:text-red-400">
            {(actionData as ActionFailure).error}
          </p>
        </div>
      )}

      {readiness.canCreate && adapters.length > 0 && (
        <Form method="post" className="space-y-5">
          <div>
            <label
              htmlFor="name"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300"
            >
              Project Name
            </label>
            <input
              type="text"
              name="name"
              id="name"
              required
              value={name}
              onChange={(e) => handleNameChange(e.target.value)}
              className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-gray-100 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
              placeholder="My Project"
            />
          </div>

          <div>
            <label
              htmlFor="slug"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300"
            >
              Slug
            </label>
            <input
              type="text"
              name="slug"
              id="slug"
              required
              value={slug}
              onChange={(e) => {
                setSlug(e.target.value);
                setSlugEdited(true);
                if (!repoEdited) setRepoName(e.target.value);
              }}
              className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-gray-100 font-mono focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
              placeholder="my-project"
              pattern="[a-z0-9][a-z0-9-]*[a-z0-9]"
            />
          </div>

          <div>
            <label
              htmlFor="description"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300"
            >
              Description
            </label>
            <textarea
              name="description"
              id="description"
              rows={2}
              className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-gray-100 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
              placeholder="Brief project description"
            />
          </div>

          <div>
            <label
              htmlFor="adapterId"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300"
            >
              Factory Adapter
            </label>
            <select
              name="adapterId"
              id="adapterId"
              required
              defaultValue={adapters[0].id}
              className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-gray-100 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
            >
              {adapters.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name} @ {a.version}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Variant
            </label>
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
              {VARIANTS.map((v) => (
                <label
                  key={v.value}
                  className="relative flex items-start border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-3 cursor-pointer hover:border-indigo-500 dark:hover:border-indigo-500 has-[:checked]:border-indigo-500 has-[:checked]:bg-indigo-50 dark:has-[:checked]:bg-indigo-900/20"
                >
                  <input
                    type="radio"
                    name="variant"
                    value={v.value}
                    checked={variant === v.value}
                    onChange={() => handleVariantChange(v.value)}
                    required
                    className="mt-0.5 h-4 w-4 text-indigo-600 border-gray-300"
                  />
                  <div className="ml-3">
                    <span className="block text-sm font-medium text-gray-900 dark:text-gray-100">
                      {v.label}
                    </span>
                    <span className="block text-xs text-gray-500 dark:text-gray-400">
                      {v.description}
                    </span>
                  </div>
                </label>
              ))}
            </div>
          </div>

          {variant !== "dual" && (
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Modules
              </label>
              <p className="text-xs text-gray-500 dark:text-gray-400 mb-3">
                Pre-checked entries match the variant's preset; uncheck to drop
                them or add others. Conflicting modules are managed automatically.
              </p>
              <div className="space-y-4">
                {Object.entries(MODULES_BY_CATEGORY).map(([category, mods]) => (
                  <fieldset
                    key={category}
                    className="rounded-md border border-gray-200 dark:border-gray-700 px-4 py-3"
                  >
                    <legend className="text-xs font-semibold uppercase tracking-wide text-gray-500 dark:text-gray-400 px-1">
                      {category}
                    </legend>
                    <div className="space-y-2 mt-1">
                      {mods.map((m) => {
                        const checked = selectedModules.has(m.id);
                        return (
                          <label
                            key={m.id}
                            className="flex items-start gap-3 cursor-pointer"
                          >
                            <input
                              type="checkbox"
                              name="modules"
                              value={m.id}
                              checked={checked}
                              onChange={(e) =>
                                toggleModule(m.id, e.target.checked)
                              }
                              className="mt-0.5 h-4 w-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-500"
                            />
                            <span>
                              <span className="block text-sm font-medium text-gray-900 dark:text-gray-100">
                                {m.displayName}
                              </span>
                              <span className="block text-xs text-gray-500 dark:text-gray-400">
                                {m.description}
                              </span>
                            </span>
                          </label>
                        );
                      })}
                    </div>
                  </fieldset>
                ))}
              </div>
            </div>
          )}

          <div>
            <label
              htmlFor="repoName"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300"
            >
              Repository Name
            </label>
            <input
              type="text"
              name="repoName"
              id="repoName"
              required
              value={repoName}
              onChange={(e) => {
                setRepoName(e.target.value);
                setRepoEdited(true);
              }}
              className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-gray-100 font-mono focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
              placeholder="my-project"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Visibility
            </label>
            <div className="flex gap-4">
              <label className="flex items-center">
                <input
                  type="radio"
                  name="visibility"
                  value="private"
                  defaultChecked
                  className="h-4 w-4 text-indigo-600 border-gray-300"
                />
                <span className="ml-2 text-sm text-gray-700 dark:text-gray-300">
                  Private
                </span>
              </label>
              <label className="flex items-center">
                <input
                  type="radio"
                  name="visibility"
                  value="public"
                  className="h-4 w-4 text-indigo-600 border-gray-300"
                />
                <span className="ml-2 text-sm text-gray-700 dark:text-gray-300">
                  Public
                </span>
              </label>
            </div>
          </div>

          <div className="flex items-center gap-4 pt-2">
            <button
              type="submit"
              disabled={submitDisabled}
              className="inline-flex items-center rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? "Creating..." : "Create Project"}
            </button>
            <a
              href="/app"
              className="text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
            >
              Cancel
            </a>
          </div>
        </Form>
      )}
    </div>
  );
}

function renderReadinessBanner(readiness: ScaffoldReadiness): React.ReactNode {
  switch (readiness.blocker) {
    case "no-factory-adapter":
      return (
        <div className="rounded-md bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 px-4 py-3">
          <p className="text-sm text-yellow-900 dark:text-yellow-200">
            <strong>No factory adapters available.</strong> Visit{" "}
            <a href="/app/factory" className="underline font-medium">
              /app/factory
            </a>{" "}
            and run a sync to populate this org's adapter catalog before
            creating projects.
          </p>
        </div>
      );
    case "stale-adapter-manifest":
      return (
        <div className="rounded-md bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 px-4 py-3">
          <p className="text-sm text-yellow-900 dark:text-yellow-200">
            <strong>Adapter manifest needs refreshing.</strong> Your
            existing factory adapter rows predate the spec 138 translator
            change and lack the <code>template_remote</code> field the
            scaffold layer needs. Visit{" "}
            <a href="/app/factory" className="underline font-medium">
              /app/factory
            </a>{" "}
            and trigger a sync — once it completes the warmup will start
            automatically and the form will unlock.
          </p>
        </div>
      );
    case "no-upstream-pat":
      return (
        <div className="rounded-md bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 px-4 py-3">
          <p className="text-sm text-yellow-900 dark:text-yellow-200">
            <strong>No factory upstream PAT configured.</strong> Add a PAT at{" "}
            <a
              href="/app/admin/factory/pat"
              className="underline font-medium"
            >
              /app/admin/factory/pat
            </a>{" "}
            so stagecraft can clone the template repo on your behalf.
          </p>
        </div>
      );
    case "warmup-error":
      return (
        <div className="rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 px-4 py-3">
          <p className="text-sm text-red-700 dark:text-red-400">
            <strong>Scaffold infrastructure error:</strong>{" "}
            {readiness.error ?? "Warmup failed; check stagecraft logs."}
          </p>
        </div>
      );
    case "warming-up":
      return (
        <div className="rounded-md bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 px-4 py-3">
          <p className="text-sm text-blue-700 dark:text-blue-300">
            <strong>Warming up scaffold cache</strong> — step:{" "}
            <code>{readiness.step}</code> ({readiness.progress}%). The form will
            unlock once the four prebuilds are ready (~2-3 min after deploy).
          </p>
        </div>
      );
    default:
      return null;
  }
}

function CreateSuccess({ data }: { data: ActionSuccess }) {
  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div className="rounded-md bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 px-4 py-3">
        <p className="text-sm text-green-800 dark:text-green-300">
          Project created. Your GitHub repo now holds commit #1 with a seeded{" "}
          <code>.factory/pipeline-state.json</code> and the{" "}
          <code>{data.profile}</code> profile tree.
        </p>
      </div>
      <dl className="rounded-md border border-gray-200 dark:border-gray-700 divide-y divide-gray-200 dark:divide-gray-700">
        <div className="grid grid-cols-[12rem_1fr] px-4 py-3">
          <dt className="text-sm font-medium text-gray-500 dark:text-gray-400">
            GitHub repo
          </dt>
          <dd className="text-sm text-indigo-600 dark:text-indigo-400">
            <a href={data.repoUrl} target="_blank" rel="noreferrer">
              {data.repoUrl}
            </a>
          </dd>
        </div>
        <div className="grid grid-cols-[12rem_1fr] px-4 py-3">
          <dt className="text-sm font-medium text-gray-500 dark:text-gray-400">
            Clone URL
          </dt>
          <dd className="text-sm font-mono text-gray-900 dark:text-gray-100 break-all">
            {data.cloneUrl}
          </dd>
        </div>
        <div className="grid grid-cols-[12rem_1fr] px-4 py-3">
          <dt className="text-sm font-medium text-gray-500 dark:text-gray-400">
            Open in OPC
          </dt>
          <dd>
            <a
              href={data.opcDeepLink}
              className="inline-flex items-center rounded-md bg-indigo-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-indigo-700"
            >
              Launch Factory Cockpit
            </a>
            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400 font-mono break-all">
              {data.opcDeepLink}
            </p>
          </dd>
        </div>
      </dl>
      <div>
        <a
          href={`/app/project/${data.projectId}`}
          className="text-sm text-indigo-600 dark:text-indigo-400"
        >
          Go to project dashboard →
        </a>
      </div>
    </div>
  );
}
