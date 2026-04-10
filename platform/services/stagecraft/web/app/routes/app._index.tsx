import { useLoaderData, Link } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getDefaultWorkspace,
  listKnowledgeObjects,
} from "../lib/workspace-api.server";
import { listProjects } from "../lib/projects-api.server";
import type {
  KnowledgeObjectRow,
  WorkspaceRow,
} from "../lib/workspace-api.server";

export async function loader({ request }: { request: Request }) {
  await requireUser(request);

  let workspace: WorkspaceRow | null = null;
  let objects: KnowledgeObjectRow[] = [];
  let projects: Array<{ id: string; name: string; slug: string }> = [];

  try {
    const wsRes = await getDefaultWorkspace(request);
    workspace = wsRes.workspace;
  } catch {
    // no workspace yet
  }

  try {
    const koRes = await listKnowledgeObjects(request);
    objects = koRes.objects;
  } catch {
    // knowledge service may not be ready
  }

  try {
    const pRes = await listProjects(request);
    projects = pRes.projects;
  } catch {
    // projects service may not be ready
  }

  return { workspace, objects, projects };
}

const STATE_COLORS: Record<string, string> = {
  imported: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300",
  extracting: "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300",
  extracted: "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300",
  classified: "bg-cyan-100 text-cyan-800 dark:bg-cyan-900/30 dark:text-cyan-300",
  available: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
};

export default function Dashboard() {
  const { workspace, objects, projects } = useLoaderData() as {
    workspace: WorkspaceRow | null;
    objects: KnowledgeObjectRow[];
    projects: Array<{ id: string; name: string; slug: string }>;
  };

  if (!workspace) {
    return (
      <div className="text-center py-12">
        <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
          No workspace found
        </h3>
        <p className="text-gray-500 dark:text-gray-400">
          A default workspace will be created when you sign in with a GitHub
          organization.
        </p>
      </div>
    );
  }

  // Compute stats
  const stateCounts = objects.reduce(
    (acc, o) => {
      acc[o.state] = (acc[o.state] || 0) + 1;
      return acc;
    },
    {} as Record<string, number>
  );
  const totalSize = objects.reduce((s, o) => s + (o.sizeBytes || 0), 0);

  return (
    <div className="space-y-8">
      {/* Stats cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard label="Projects" value={projects.length} />
        <StatCard label="Knowledge Objects" value={objects.length} />
        <StatCard
          label="Available Docs"
          value={stateCounts["available"] ?? 0}
        />
        <StatCard label="Total Size" value={formatBytes(totalSize)} />
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Recent knowledge objects */}
        <section>
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider">
              Recent Knowledge Objects
            </h3>
            <Link
              to="/app/knowledge"
              className="text-sm text-indigo-600 hover:text-indigo-500 dark:text-indigo-400"
            >
              View all
            </Link>
          </div>
          {objects.length === 0 ? (
            <EmptyState message="No knowledge objects yet.">
              <Link
                to="/app/knowledge"
                className="text-sm text-indigo-600 hover:text-indigo-500 dark:text-indigo-400"
              >
                Upload your first document
              </Link>
            </EmptyState>
          ) : (
            <ul className="divide-y divide-gray-200 dark:divide-gray-700 border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
              {objects.slice(0, 5).map((obj) => (
                <li
                  key={obj.id}
                  className="px-4 py-3 bg-white dark:bg-gray-900 flex items-center justify-between gap-3"
                >
                  <div className="min-w-0">
                    <Link
                      to={`/app/knowledge/${obj.id}`}
                      className="text-sm font-medium text-gray-900 dark:text-gray-100 hover:text-indigo-600 dark:hover:text-indigo-400 truncate block"
                    >
                      {obj.filename}
                    </Link>
                    <span className="text-xs text-gray-500 dark:text-gray-400">
                      {obj.mimeType} &middot; {formatBytes(obj.sizeBytes)}
                    </span>
                  </div>
                  <span
                    className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${STATE_COLORS[obj.state] ?? "bg-gray-100 text-gray-800"}`}
                  >
                    {obj.state}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>

        {/* Projects */}
        <section>
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider">
              Projects
            </h3>
            <Link
              to="/app/pipelines"
              className="text-sm text-indigo-600 hover:text-indigo-500 dark:text-indigo-400"
            >
              View pipelines
            </Link>
          </div>
          {projects.length === 0 ? (
            <EmptyState message="No projects yet." />
          ) : (
            <ul className="divide-y divide-gray-200 dark:divide-gray-700 border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
              {projects.slice(0, 5).map((p) => (
                <li
                  key={p.id}
                  className="px-4 py-3 bg-white dark:bg-gray-900"
                >
                  <Link
                    to={`/app/pipelines/${p.id}`}
                    className="text-sm font-medium text-gray-900 dark:text-gray-100 hover:text-indigo-600 dark:hover:text-indigo-400"
                  >
                    {p.name}
                  </Link>
                  <span className="ml-2 text-xs text-gray-500 dark:text-gray-400">
                    {p.slug}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>

      {/* Knowledge state distribution */}
      {objects.length > 0 && (
        <section>
          <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider mb-3">
            Knowledge State Distribution
          </h3>
          <div className="flex gap-2 flex-wrap">
            {Object.entries(stateCounts).map(([state, count]) => (
              <span
                key={state}
                className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-sm font-medium ${STATE_COLORS[state] ?? "bg-gray-100 text-gray-800"}`}
              >
                {state}
                <span className="font-bold">{count}</span>
              </span>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function StatCard({
  label,
  value,
}: {
  label: string;
  value: string | number;
}) {
  return (
    <div className="bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-3">
      <dt className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
        {label}
      </dt>
      <dd className="mt-1 text-2xl font-semibold text-gray-900 dark:text-gray-100">
        {value}
      </dd>
    </div>
  );
}

function EmptyState({
  message,
  children,
}: {
  message: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="border border-dashed border-gray-300 dark:border-gray-600 rounded-lg px-4 py-8 text-center">
      <p className="text-sm text-gray-500 dark:text-gray-400">{message}</p>
      {children && <div className="mt-2">{children}</div>}
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
