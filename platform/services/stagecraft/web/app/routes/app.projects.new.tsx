/**
 * Self-service project creation form (spec 080 Phase 2 — FR-006).
 *
 * Authenticated org members can create a project with a GitHub repo,
 * adapter template, branch protection, and CI workflow.
 */

import { Form, useActionData, useNavigation, redirect } from "react-router";
import { useState } from "react";
import { requireUser } from "../lib/auth.server";
import { createProjectWithRepo } from "../lib/projects-api.server";

const ADAPTERS = [
  { value: "aim-vue-node", label: "AIM Vue + Node", description: "Vue 3 frontend with Node.js API backend" },
  { value: "encore-react", label: "Encore + React", description: "Encore.ts backend with React frontend" },
  { value: "next-prisma", label: "Next.js + Prisma", description: "Next.js full-stack with Prisma ORM" },
  { value: "rust-axum", label: "Rust Axum", description: "Rust backend with Axum framework" },
] as const;

export async function loader({ request }: { request: Request }) {
  await requireUser(request);
  return {};
}

export async function action({ request }: { request: Request }) {
  const user = await requireUser(request);
  const formData = await request.formData();

  const name = formData.get("name") as string;
  const slug = formData.get("slug") as string;
  const description = formData.get("description") as string;
  const adapter = formData.get("adapter") as string;
  const repoName = formData.get("repoName") as string;
  const isPrivate = formData.get("visibility") !== "public";

  if (!name || !slug || !adapter || !repoName) {
    return { error: "Name, slug, adapter, and repository name are required." };
  }

  if (slug.length < 3 || !/^[a-z0-9][a-z0-9-]*[a-z0-9]$/.test(slug)) {
    return { error: "Slug must be at least 3 characters, lowercase alphanumeric with hyphens (e.g., my-project)." };
  }

  try {
    const result = await createProjectWithRepo(request, {
      name,
      slug,
      description: description || undefined,
      adapter,
      repoName,
      isPrivate,
    });
    return redirect(`/app/project/${result.project.id}`);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.error("createProjectWithRepo failed", {
      userId: user.userId,
      orgSlug: user.orgSlug,
      slug,
      repoName,
      adapter,
      error: msg,
    });

    // Extract Encore APIError message from the JSON body apiFetch throws.
    let backendMsg = msg;
    try {
      const parsed = JSON.parse(msg) as { message?: string; code?: string };
      if (parsed.message) backendMsg = parsed.message;
    } catch {
      // msg wasn't JSON — leave it alone
    }

    if (backendMsg.includes("already exists")) {
      return { error: "A project or repository with that name already exists." };
    }
    if (backendMsg.includes("No active GitHub App")) {
      return { error: "No GitHub App installation found for your org. Install the OAP GitHub App from the admin settings." };
    }
    if (backendMsg.includes("missing required permissions")) {
      return { error: backendMsg };
    }
    if (backendMsg.includes("Insufficient permissions") || backendMsg.includes("permission")) {
      return { error: "You don't have permission to create projects in this org." };
    }
    if (backendMsg.includes("No active workspace")) {
      return { error: "No active workspace for your org. Ask your org admin to create a default workspace." };
    }
    return { error: `Failed to create project: ${backendMsg}` };
  }
}

function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}

export default function NewProject() {
  const actionData = useActionData() as { error?: string } | undefined;
  const navigation = useNavigation();
  const isSubmitting = navigation.state === "submitting";
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [repoName, setRepoName] = useState("");
  const [slugEdited, setSlugEdited] = useState(false);
  const [repoEdited, setRepoEdited] = useState(false);

  const handleNameChange = (value: string) => {
    setName(value);
    const derived = slugify(value);
    if (!slugEdited) setSlug(derived);
    if (!repoEdited) setRepoName(slugEdited ? slug : derived);
  };

  return (
    <div className="max-w-2xl mx-auto space-y-6">
      <div>
        <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Create New Project
        </h2>
        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
          Set up a new project with a GitHub repository, CI workflow, and deployment environments.
        </p>
      </div>

      {actionData?.error && (
        <div className="rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 px-4 py-3">
          <p className="text-sm text-red-700 dark:text-red-400">{actionData.error}</p>
        </div>
      )}

      <Form method="post" className="space-y-5">
        {/* Project Name */}
        <div>
          <label htmlFor="name" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
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

        {/* Slug */}
        <div>
          <label htmlFor="slug" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
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
          <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
            Lowercase letters, numbers, and hyphens. Used in URLs and namespaces.
          </p>
        </div>

        {/* Description */}
        <div>
          <label htmlFor="description" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
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

        {/* Adapter Selection */}
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Adapter Template
          </label>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {ADAPTERS.map((adapter) => (
              <label
                key={adapter.value}
                className="relative flex items-start border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-3 cursor-pointer hover:border-indigo-500 dark:hover:border-indigo-500 has-[:checked]:border-indigo-500 has-[:checked]:bg-indigo-50 dark:has-[:checked]:bg-indigo-900/20"
              >
                <input
                  type="radio"
                  name="adapter"
                  value={adapter.value}
                  required
                  className="mt-0.5 h-4 w-4 text-indigo-600 border-gray-300"
                />
                <div className="ml-3">
                  <span className="block text-sm font-medium text-gray-900 dark:text-gray-100">
                    {adapter.label}
                  </span>
                  <span className="block text-xs text-gray-500 dark:text-gray-400">
                    {adapter.description}
                  </span>
                </div>
              </label>
            ))}
          </div>
        </div>

        {/* Repository Name */}
        <div>
          <label htmlFor="repoName" className="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Repository Name
          </label>
          <input
            type="text"
            name="repoName"
            id="repoName"
            required
            value={repoName}
            onChange={(e) => { setRepoName(e.target.value); setRepoEdited(true); }}
            className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-3 py-2 text-sm text-gray-900 dark:text-gray-100 font-mono focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
            placeholder="my-project"
          />
          <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
            The GitHub repository will be created in your org with this name.
          </p>
        </div>

        {/* Visibility */}
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
              <span className="ml-2 text-sm text-gray-700 dark:text-gray-300">Private</span>
            </label>
            <label className="flex items-center">
              <input
                type="radio"
                name="visibility"
                value="public"
                className="h-4 w-4 text-indigo-600 border-gray-300"
              />
              <span className="ml-2 text-sm text-gray-700 dark:text-gray-300">Public</span>
            </label>
          </div>
        </div>

        {/* Submit */}
        <div className="flex items-center gap-4 pt-2">
          <button
            type="submit"
            disabled={isSubmitting}
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
    </div>
  );
}
