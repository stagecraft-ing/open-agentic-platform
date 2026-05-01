import { Link, useOutletContext } from "react-router";

type ProjectCtx = {
  project: {
    id: string;
    name: string;
    slug: string;
    description?: string;
  };
};

export default function ProjectOverview() {
  const { project } = useOutletContext<ProjectCtx>();
  const base = `/app/project/${project.id}`;

  const tiles = [
    {
      to: `${base}/knowledge`,
      label: "Knowledge",
      hint: "Documents bound to this project and their state.",
    },
    {
      to: `${base}/agents`,
      label: "Imported agents",
      hint: "Org agents imported into this project via version-pinned bindings.",
    },
    {
      to: `${base}/pipelines`,
      label: "Pipelines",
      hint: "Factory pipeline runs and stage approvals.",
    },
    {
      to: `${base}/deploys`,
      label: "Deploys",
      hint: "Environment promotion across preview → dev → staging → prod.",
    },
    {
      to: `${base}/settings`,
      label: "Settings",
      hint: "Repos, environments, members, connectors.",
    },
  ];

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
      {tiles.map((t) => (
        <Link
          key={t.to}
          to={t.to}
          className="block border border-gray-200 dark:border-gray-700 rounded-lg px-4 py-4 bg-white dark:bg-gray-900 hover:border-indigo-500 dark:hover:border-indigo-500 transition-colors group"
        >
          <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 group-hover:text-indigo-600 dark:group-hover:text-indigo-400">
            {t.label}
          </h3>
          <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">{t.hint}</p>
        </Link>
      ))}
    </div>
  );
}
