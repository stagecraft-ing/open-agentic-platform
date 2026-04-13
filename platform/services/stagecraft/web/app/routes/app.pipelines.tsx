import { useLoaderData, Link } from "react-router";
import { requireUser } from "../lib/auth.server";
import { listProjects } from "../lib/projects-api.server";

export async function loader({ request }: { request: Request }) {
  await requireUser(request);

  let projects: Array<{
    id: string;
    name: string;
    slug: string;
    description?: string;
    createdAt: string;
  }> = [];

  try {
    const res = await listProjects(request);
    projects = res.projects;
  } catch {
    // projects service may not be ready
  }

  return { projects };
}

export default function PipelinesList() {
  const { projects } = useLoaderData() as {
    projects: Array<{
      id: string;
      name: string;
      slug: string;
      description?: string;
      createdAt: string;
    }>;
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
            Factory Pipelines
          </h2>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Select a project to view its factory pipeline status and manage stage
            approvals.
          </p>
        </div>
        <Link
          to="/app/projects/new"
          className="inline-flex items-center rounded-md bg-indigo-600 px-3 py-2 text-sm font-medium text-white hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2"
        >
          New Project
        </Link>
      </div>

      {projects.length === 0 ? (
        <div className="border border-dashed border-gray-300 dark:border-gray-600 rounded-lg px-4 py-12 text-center">
          <p className="text-sm text-gray-500 dark:text-gray-400 mb-3">
            No projects yet.
          </p>
          <Link
            to="/app/projects/new"
            className="text-sm text-indigo-600 dark:text-indigo-400 hover:underline"
          >
            Create your first project
          </Link>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {projects.map((p) => (
            <Link
              key={p.id}
              to={`/app/pipelines/${p.id}`}
              className="border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-4 bg-white dark:bg-gray-900 hover:border-indigo-500 dark:hover:border-indigo-500 transition-colors group"
            >
              <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 group-hover:text-indigo-600 dark:group-hover:text-indigo-400">
                {p.name}
              </h3>
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                {p.slug}
              </p>
              {p.description && (
                <p className="text-sm text-gray-600 dark:text-gray-400 mt-2 line-clamp-2">
                  {p.description}
                </p>
              )}
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
