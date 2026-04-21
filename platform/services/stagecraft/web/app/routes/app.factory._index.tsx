/**
 * Factory Overview (spec 108 Phase 2).
 *
 * Live view of the org's upstream config + counts of the derived resources.
 * Sync action lands in Phase 3; this page only reads the stored config and
 * last-sync metadata.
 */

import { Link, useLoaderData } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getFactoryUpstreams,
  type FactoryUpstream,
  type FactoryUpstreamCounts,
} from "../lib/factory-api.server";

type LoaderData = {
  upstream: FactoryUpstream | null;
  counts: FactoryUpstreamCounts;
  canConfigure: boolean;
};

export async function loader({ request }: { request: Request }): Promise<LoaderData> {
  const user = await requireUser(request);
  const { upstream, counts } = await getFactoryUpstreams(request);
  const canConfigure =
    user.platformRole === "owner" || user.platformRole === "admin";
  return { upstream, counts, canConfigure };
}

export default function FactoryOverview() {
  const { upstream, counts, canConfigure } = useLoaderData<typeof loader>();

  return (
    <div className="space-y-6">
      <section className="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 p-5">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider">
              Upstream sources
            </h2>
            <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
              Factory adapters, contracts, and processes are generated from two
              GitHub sources. Replaces the legacy
              <code className="mx-1 px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-800 font-mono text-xs">
                factory/upstream-map.yaml
              </code>
              manifest.
            </p>
          </div>
          <Link
            to="/app/factory/upstreams"
            className="inline-flex shrink-0 items-center rounded-md bg-indigo-600 px-3 py-1.5 text-xs font-medium text-white shadow-sm hover:bg-indigo-700"
          >
            {upstream ? "Edit" : canConfigure ? "Configure" : "View"}
          </Link>
        </div>

        <div className="mt-4 grid grid-cols-1 sm:grid-cols-2 gap-4">
          <UpstreamCard
            title="Factory source"
            hint="Canonical process definitions and adapter scaffolds."
            repo={upstream?.factorySource ?? null}
            ref={upstream?.factoryRef ?? null}
            sha={upstream?.lastSyncSha?.factory ?? null}
            placeholder="GovAlta-Pronghorn/goa-software-factory"
          />
          <UpstreamCard
            title="Template source"
            hint="Per-project templates consumed by the factory."
            repo={upstream?.templateSource ?? null}
            ref={upstream?.templateRef ?? null}
            sha={upstream?.lastSyncSha?.template ?? null}
            placeholder="GovAlta-Pronghorn/template"
          />
        </div>

        <SyncStatus upstream={upstream} />
      </section>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Tile
          title="Adapters"
          description="Pluggable tech stacks — aim-vue-node, next-prisma, encore-react, rust-axum."
          count={counts.adapters}
        />
        <Tile
          title="Contracts"
          description="Build Spec, Adapter Manifest, Pipeline State, Verification schemas."
          count={counts.contracts}
        />
        <Tile
          title="Processes"
          description="7-stage pipeline definitions executed by OPC agents."
          count={counts.processes}
        />
      </div>
    </div>
  );
}

function UpstreamCard({
  title,
  hint,
  repo,
  ref,
  sha,
  placeholder,
}: {
  title: string;
  hint: string;
  repo: string | null;
  ref: string | null;
  sha: string | null;
  placeholder: string;
}) {
  return (
    <div className="rounded-md border border-gray-200 dark:border-gray-700 p-3">
      <div className="text-xs font-medium text-gray-700 dark:text-gray-300">
        {title}
      </div>
      <div className="mt-1 font-mono text-sm text-gray-900 dark:text-gray-100">
        {repo ?? (
          <span className="text-gray-400 dark:text-gray-500">
            {placeholder}
          </span>
        )}
      </div>
      <div className="mt-1 flex gap-3 text-xs text-gray-500 dark:text-gray-400">
        <span>
          ref: <code className="font-mono">{ref ?? "—"}</code>
        </span>
        <span>
          sha: <code className="font-mono">{sha ? sha.slice(0, 7) : "—"}</code>
        </span>
      </div>
      <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">{hint}</p>
    </div>
  );
}

function SyncStatus({ upstream }: { upstream: FactoryUpstream | null }) {
  if (!upstream) {
    return (
      <div className="mt-4 text-xs text-gray-500 dark:text-gray-400">
        No upstream configured yet. Sync will run once upstreams are set.
      </div>
    );
  }

  const status = upstream.lastSyncStatus ?? "pending";
  const color =
    status === "ok"
      ? "text-emerald-700 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-900/20 border-emerald-200 dark:border-emerald-800"
      : status === "failed"
        ? "text-red-700 dark:text-red-400 bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800"
        : "text-amber-700 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20 border-amber-200 dark:border-amber-800";

  return (
    <div className={`mt-4 flex items-start gap-3 text-xs rounded border px-3 py-2 ${color}`}>
      <div className="flex-1">
        <div className="font-medium">
          Last sync:{" "}
          {upstream.lastSyncedAt
            ? new Date(upstream.lastSyncedAt).toLocaleString()
            : "never"}{" "}
          — {status}
        </div>
        {upstream.lastSyncError ? (
          <div className="mt-1 font-mono text-[11px] break-all">
            {upstream.lastSyncError}
          </div>
        ) : null}
      </div>
      <span className="shrink-0 italic opacity-80">
        Sync worker arrives in Phase 3.
      </span>
    </div>
  );
}

function Tile({
  title,
  description,
  count,
}: {
  title: string;
  description: string;
  count: number;
}) {
  return (
    <div className="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 p-4">
      <div className="flex items-baseline justify-between">
        <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
          {title}
        </h3>
        <span className="text-xs font-mono text-gray-400 dark:text-gray-500">
          {count}
        </span>
      </div>
      <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
        {description}
      </p>
    </div>
  );
}
