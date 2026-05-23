// Spec 163 — Requirements view (read-shaped spec-spine surface).
//
// Renders the project's authored spec corpus as the primary
// stakeholder-visible artifact of OAP's "spec-spine as universal
// representation" thesis (intent doc §2.4). All reads route through
// the Encore API surface in `api/specRegistry/`, which shells
// registry-consumer per spec 103 governed-read discipline (FR-002).

import { Link, useLoaderData, useSearchParams } from "react-router";
import { requireUser } from "../lib/auth.server";
import { listProjectSpecs } from "../lib/spec-registry-api.server";
import type {
  SpecInventory,
  SpecListRow,
} from "../../../api/specRegistry/types";
import {
  buildDerivedGroups,
  type DerivedGroup,
  type GroupingDimension,
  GROUPING_DIMENSIONS,
} from "../lib/spec-registry-grouping";

export async function loader({
  request,
  params,
}: {
  request: Request;
  params: { projectId: string };
}) {
  await requireUser(request);
  const inventory = await listProjectSpecs(request, params.projectId);
  return { projectId: params.projectId, inventory };
}

const STATUS_STYLES: Record<string, string> = {
  draft: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300",
  approved: "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300",
  superseded: "bg-gray-200 text-gray-700 dark:bg-gray-800 dark:text-gray-300",
};

const IMPLEMENTATION_STYLES: Record<string, string> = {
  complete: "bg-green-50 text-green-700 dark:bg-green-900/20 dark:text-green-300",
  "in-progress": "bg-blue-50 text-blue-700 dark:bg-blue-900/20 dark:text-blue-300",
  pending: "bg-amber-50 text-amber-700 dark:bg-amber-900/20 dark:text-amber-300",
  deferred: "bg-slate-50 text-slate-700 dark:bg-slate-800/40 dark:text-slate-300",
  "n/a": "bg-gray-50 text-gray-700 dark:bg-gray-800/40 dark:text-gray-300",
};

const VIEW_MODES = ["list", "grouped"] as const;
type ViewMode = (typeof VIEW_MODES)[number];

function isViewMode(s: string | null): s is ViewMode {
  return s === "list" || s === "grouped";
}

function isGroupingDimension(s: string | null): s is GroupingDimension {
  return (
    s === "by-category" ||
    s === "by-establishment-chain" ||
    s === "by-supersession-chain"
  );
}

export default function RequirementsView() {
  const { projectId, inventory } = useLoaderData() as {
    projectId: string;
    inventory: SpecInventory;
  };
  const [searchParams] = useSearchParams();
  const viewParam = searchParams.get("view");
  const dimParam = searchParams.get("dim");
  const view: ViewMode = isViewMode(viewParam) ? viewParam : "list";
  const dimension: GroupingDimension = isGroupingDimension(dimParam)
    ? dimParam
    : "by-category";

  if (!inventory.registryAvailable) {
    return <EmptyState projectId={projectId} />;
  }

  if (inventory.specs.length === 0) {
    // Spec-spine published but no rows yet (decomposition ran, found
    // nothing). Treat as empty too — same CTA covers this case.
    return <EmptyState projectId={projectId} />;
  }

  return (
    <div className="space-y-4">
      <ViewToolbar
        projectId={projectId}
        view={view}
        dimension={dimension}
        count={inventory.specs.length}
      />
      {view === "list" ? (
        <SpecTable specs={inventory.specs} projectId={projectId} />
      ) : (
        <GroupedView
          specs={inventory.specs}
          dimension={dimension}
          projectId={projectId}
        />
      )}
    </div>
  );
}

function ViewToolbar({
  projectId,
  view,
  dimension,
  count,
}: {
  projectId: string;
  view: ViewMode;
  dimension: GroupingDimension;
  count: number;
}) {
  const base = `/app/project/${projectId}/requirements`;
  return (
    <div className="flex flex-wrap items-center justify-between gap-3 border-b border-gray-200 dark:border-gray-700 pb-3">
      <div>
        <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">
          Requirements
        </h2>
        <p className="text-xs text-gray-500 dark:text-gray-400">
          {count} spec{count === 1 ? "" : "s"} in this project's spec spine.
        </p>
      </div>
      <div className="flex flex-wrap gap-1">
        <Link
          to={base}
          className={pillClass(view === "list")}
          aria-current={view === "list" ? "page" : undefined}
        >
          Flat list
        </Link>
        {GROUPING_DIMENSIONS.map((d) => (
          <Link
            key={d.value}
            to={`${base}?view=grouped&dim=${d.value}`}
            className={pillClass(view === "grouped" && dimension === d.value)}
            aria-current={
              view === "grouped" && dimension === d.value ? "page" : undefined
            }
          >
            {d.label}
          </Link>
        ))}
      </div>
    </div>
  );
}

function pillClass(active: boolean): string {
  return [
    "inline-flex items-center px-3 py-1.5 rounded-md text-xs font-medium transition-colors border",
    active
      ? "bg-indigo-600 text-white border-indigo-600"
      : "bg-white dark:bg-gray-900 text-gray-700 dark:text-gray-300 border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-800",
  ].join(" ");
}

function GroupedView({
  specs,
  dimension,
  projectId,
}: {
  specs: SpecListRow[];
  dimension: GroupingDimension;
  projectId: string;
}) {
  const groups = buildDerivedGroups(specs, dimension);
  if (groups.length === 0) {
    return (
      <div className="border border-dashed border-gray-300 dark:border-gray-600 rounded-lg px-4 py-8 text-center text-sm text-gray-500 dark:text-gray-400">
        No groups derivable on this dimension yet. Switch projection or fall
        back to the flat list.
      </div>
    );
  }
  return (
    <div className="space-y-6">
      {groups.map((g) => (
        <GroupSection key={g.id} group={g} projectId={projectId} />
      ))}
    </div>
  );
}

function GroupSection({
  group,
  projectId,
}: {
  group: DerivedGroup;
  projectId: string;
}) {
  return (
    <section>
      <header className="flex items-baseline gap-2 mb-2">
        <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider">
          {group.label}
        </h3>
        <span className="text-xs text-gray-400 dark:text-gray-500">
          ({group.specs.length})
        </span>
      </header>
      <SpecTable specs={group.specs} projectId={projectId} />
    </section>
  );
}

function SpecTable({
  specs,
  projectId,
}: {
  specs: SpecListRow[];
  projectId: string;
}) {
  const base = `/app/project/${projectId}/requirements`;
  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
      <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
        <thead className="bg-gray-50 dark:bg-gray-800">
          <tr>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
              Id
            </th>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
              Title
            </th>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
              Kind
            </th>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
              Status
            </th>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
              Implementation
            </th>
            <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
              Provenance
            </th>
          </tr>
        </thead>
        <tbody className="bg-white dark:bg-gray-900 divide-y divide-gray-200 dark:divide-gray-700">
          {specs.map((s) => (
            <tr
              key={s.id}
              className="hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors"
            >
              <td className="px-4 py-3 text-sm font-mono text-gray-500 dark:text-gray-400">
                {s.id.split("-")[0]}
              </td>
              <td className="px-4 py-3 text-sm">
                <Link
                  to={`${base}/${encodeURIComponent(s.id)}`}
                  className="font-medium text-gray-900 dark:text-gray-100 hover:text-indigo-600 dark:hover:text-indigo-400"
                >
                  {s.title}
                </Link>
                {s.categories.map((c) => (
                  <span
                    key={c}
                    className="ml-2 inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-indigo-50 text-indigo-700 dark:bg-indigo-900/30 dark:text-indigo-300 uppercase tracking-wide"
                  >
                    {c}
                  </span>
                ))}
              </td>
              <td className="px-4 py-3 text-xs text-gray-500 dark:text-gray-400">
                {s.kind ?? "—"}
              </td>
              <td className="px-4 py-3">
                <span
                  className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                    STATUS_STYLES[s.status] ??
                    "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300"
                  }`}
                >
                  {s.status}
                </span>
              </td>
              <td className="px-4 py-3">
                <span
                  className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                    IMPLEMENTATION_STYLES[s.implementation] ??
                    "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300"
                  }`}
                >
                  {s.implementation}
                </span>
              </td>
              <td className="px-4 py-3">
                {s.hasDecompositionOrigin && (
                  <span
                    className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-purple-50 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300"
                    title="Derived from Knowledge or xray fingerprint (spec 161)"
                  >
                    decomposition
                  </span>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function EmptyState({ projectId: _projectId }: { projectId: string }) {
  return (
    <div className="border border-dashed border-gray-300 dark:border-gray-600 rounded-lg px-6 py-12 text-center">
      <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-2">
        No spec spine yet
      </h2>
      <p className="text-sm text-gray-500 dark:text-gray-400 max-w-prose mx-auto">
        This project's requirements have not been decomposed into the spec
        spine. Run the OPC decomposition pipeline against the project's
        Knowledge corpus to materialise per-requirement spec drafts here.
      </p>
      <p className="mt-4 text-xs text-gray-400 dark:text-gray-500">
        See <span className="font-mono">specs/165-opc-decomposition-pipeline</span>
        {" "}for the decomposition workflow.
      </p>
    </div>
  );
}
