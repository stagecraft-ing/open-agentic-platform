/**
 * Factory — top-level nav entry (spec 108 placeholder).
 *
 * Reimplements the legacy `factory/` folder as a first-class platform
 * feature: adapters, contracts, processes, and `upstream-map.yaml`
 * configuration. Projects are processed by the Factory; Factory runs
 * execute through OPC while orchestrated from here.
 *
 * Phase 1 lands this placeholder + navigation. Phase 3 (spec 108)
 * wires the DB schema, Encore APIs, sync against the upstream sources,
 * and OPC interface contract.
 */

import { NavLink, Outlet, useLocation } from "react-router";
import { requireUser } from "../lib/auth.server";

export async function loader({ request }: { request: Request }) {
  await requireUser(request);
  return {};
}

const TABS = [
  { to: "/app/factory", label: "Overview", end: true },
  { to: "/app/factory/upstreams", label: "Upstreams", end: false },
  { to: "/app/factory/adapters", label: "Adapters", end: false },
  { to: "/app/factory/contracts", label: "Contracts", end: false },
  { to: "/app/factory/processes", label: "Processes", end: false },
];

export default function Factory() {
  const location = useLocation();
  const isIndex = location.pathname === "/app/factory";

  return (
    <div className="space-y-6">
      <header>
        <h1 className="text-xl font-semibold text-gray-900 dark:text-gray-100">
          Factory
        </h1>
        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
          Governed delivery engine. Adapters, contracts, and processes that
          projects are run through.
        </p>
      </header>

      <div className="flex gap-1 border-b border-gray-200 dark:border-gray-700">
        {TABS.map((tab) => (
          <NavLink
            key={tab.to}
            to={tab.to}
            end={tab.end}
            className={({ isActive }) =>
              `px-3 py-2 text-sm font-medium border-b-2 transition-colors ${
                isActive
                  ? "border-indigo-500 text-indigo-600 dark:text-indigo-400"
                  : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
              }`
            }
          >
            {tab.label}
          </NavLink>
        ))}
      </div>

      {isIndex ? <FactoryOverview /> : <Outlet />}
    </div>
  );
}

function FactoryOverview() {
  return (
    <div className="space-y-6">
      <section className="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 p-5">
        <h2 className="text-sm font-semibold text-gray-900 dark:text-gray-100 uppercase tracking-wider">
          Upstream sources
        </h2>
        <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
          Factory adapters, contracts, and processes are generated from two
          GitHub sources. Configuring these replaces the legacy
          <code className="mx-1 px-1 py-0.5 rounded bg-gray-100 dark:bg-gray-800 font-mono text-xs">factory/upstream-map.yaml</code>
          manifest.
        </p>

        <div className="mt-4 grid grid-cols-1 sm:grid-cols-2 gap-4">
          <UpstreamPreview
            title="Factory source"
            hint="Canonical process definitions and adapter scaffolds."
            placeholder="GovAlta-Pronghorn/goa-software-factory"
          />
          <UpstreamPreview
            title="Template source"
            hint="Per-project templates consumed by the factory."
            placeholder="GovAlta-Pronghorn/template"
          />
        </div>

        <div className="mt-4 flex items-center gap-2 text-xs text-amber-700 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded px-3 py-2">
          <svg className="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01M4.93 19h14.14c1.54 0 2.5-1.67 1.73-3L13.73 4a2 2 0 00-3.46 0L3.2 16c-.77 1.33.19 3 1.73 3z" />
          </svg>
          <span>
            Coming in spec 108 Phase 3 — placeholders shown below.
          </span>
        </div>
      </section>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Tile title="Adapters" description="Pluggable tech stacks — aim-vue-node, next-prisma, encore-react, rust-axum." count="—" />
        <Tile title="Contracts" description="Build Spec, Adapter Manifest, Pipeline State, Verification schemas." count="—" />
        <Tile title="Processes" description="7-stage pipeline definitions executed by OPC agents." count="—" />
      </div>
    </div>
  );
}

function UpstreamPreview({
  title,
  hint,
  placeholder,
}: {
  title: string;
  hint: string;
  placeholder: string;
}) {
  return (
    <div className="rounded-md border border-dashed border-gray-300 dark:border-gray-600 p-3">
      <label className="block text-xs font-medium text-gray-700 dark:text-gray-300">
        {title}
      </label>
      <input
        type="text"
        disabled
        placeholder={placeholder}
        className="mt-1 block w-full rounded-md border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-800 px-3 py-2 text-sm font-mono text-gray-500 dark:text-gray-400 disabled:cursor-not-allowed"
      />
      <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">{hint}</p>
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
  count: string;
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
