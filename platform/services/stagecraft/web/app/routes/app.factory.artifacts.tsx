/**
 * Spec 139 Phase 1 — factory artifact substrate browser.
 *
 * Kind-filtered list + detail drawer with a textarea-based override editor
 * and a conflict resolution pane (keep-mine / take-upstream). The
 * CodeMirror merge view for `edit_and_accept` resolution lands in Phase 2
 * (per spec §11 risk 1).
 */

import { useLoaderData, Form, useNavigation } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  applyFactoryArtifactOverride,
  clearFactoryArtifactOverride,
  getFactoryArtifactById,
  listFactoryArtifactConflicts,
  listFactoryArtifacts,
  resolveFactoryArtifactConflict,
  resolveFactoryArtifactEditAndAccept,
  type ArtifactConflictSummary,
  type ArtifactDetail,
  type ArtifactKind,
  type ArtifactSummary,
} from "../lib/factory-api.server";
import { ArtifactMergeEditor } from "../components/artifact-merge-editor";

const KIND_FILTERS: Array<ArtifactKind | "all"> = [
  "all",
  "agent",
  "skill",
  "process-stage",
  "adapter-manifest",
  "contract-schema",
  "pattern",
  "page-type-reference",
  "sample-html",
  "reference-data",
  "invariant",
  "pipeline-orchestrator",
];

type LoaderData = {
  kindFilter: ArtifactKind | "all";
  artifacts: ArtifactSummary[];
  total: number;
  page: number;
  pageSize: number;
  selectedId: string | null;
  selected: ArtifactDetail | null;
  conflicts: ArtifactConflictSummary[];
  loadError: string | null;
};

export async function loader({
  request,
}: {
  request: Request;
}): Promise<LoaderData> {
  await requireUser(request);
  const url = new URL(request.url);
  const rawKind = (url.searchParams.get("kind") ?? "all") as
    | ArtifactKind
    | "all";
  const kindFilter: ArtifactKind | "all" = KIND_FILTERS.includes(rawKind)
    ? rawKind
    : "all";
  const page = Math.max(1, Number(url.searchParams.get("page") ?? "1"));
  const pageSize = Math.min(
    200,
    Math.max(10, Number(url.searchParams.get("pageSize") ?? "50")),
  );
  const selectedId = url.searchParams.get("id");

  let listResp: Awaited<ReturnType<typeof listFactoryArtifacts>>;
  let conflictsResp: Awaited<ReturnType<typeof listFactoryArtifactConflicts>>;
  try {
    [listResp, conflictsResp] = await Promise.all([
      listFactoryArtifacts(request, {
        kind: kindFilter === "all" ? undefined : kindFilter,
        page,
        pageSize,
      }),
      listFactoryArtifactConflicts(request),
    ]);
  } catch (err) {
    return {
      kindFilter,
      artifacts: [],
      total: 0,
      page,
      pageSize,
      selectedId,
      selected: null,
      conflicts: [],
      loadError: err instanceof Error ? err.message : String(err),
    };
  }

  let selected: ArtifactDetail | null = null;
  let loadError: string | null = null;
  if (selectedId) {
    try {
      selected = await getFactoryArtifactById(request, selectedId);
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    }
  }

  return {
    kindFilter,
    artifacts: listResp.artifacts,
    total: listResp.total,
    page: listResp.page,
    pageSize: listResp.pageSize,
    selectedId,
    selected,
    conflicts: conflictsResp.conflicts,
    loadError,
  };
}

type ActionResult = { ok: boolean; error?: string };

export async function action({
  request,
}: {
  request: Request;
}): Promise<ActionResult> {
  await requireUser(request);
  const form = await request.formData();
  const intent = String(form.get("intent") ?? "");
  const id = String(form.get("id") ?? "");
  if (!id) return { ok: false, error: "missing artifact id" };

  try {
    switch (intent) {
      case "save_override": {
        const userBody = String(form.get("userBody") ?? "");
        await applyFactoryArtifactOverride(request, id, userBody);
        return { ok: true };
      }
      case "clear_override": {
        await clearFactoryArtifactOverride(request, id);
        return { ok: true };
      }
      case "resolve_keep_mine": {
        await resolveFactoryArtifactConflict(request, id, "keep_mine");
        return { ok: true };
      }
      case "resolve_take_upstream": {
        await resolveFactoryArtifactConflict(request, id, "take_upstream");
        return { ok: true };
      }
      case "resolve_edit_and_accept": {
        // Spec 139 Phase 2 (T058) — merged body comes via `body`.
        const body = String(form.get("body") ?? "");
        if (body.length === 0) {
          return {
            ok: false,
            error: "edit_and_accept requires a non-empty merged body",
          };
        }
        await resolveFactoryArtifactEditAndAccept(request, id, body);
        return { ok: true };
      }
      default:
        return { ok: false, error: `unknown intent: ${intent}` };
    }
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export default function FactoryArtifacts() {
  const data = useLoaderData<typeof loader>();
  const nav = useNavigation();
  const submitting = nav.state === "submitting";

  return (
    <div className="flex h-full flex-col gap-4 p-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Factory artifacts</h1>
          <p className="text-sm text-gray-500">
            Substrate-backed view of every upstream file. Overrides apply
            per-org and survive sync; divergence surfaces below.
          </p>
        </div>
        <div className="text-right text-sm text-gray-500">
          {data.total.toLocaleString()} total · page {data.page}
        </div>
      </header>

      {data.conflicts.length > 0 ? (
        <ConflictsPanel conflicts={data.conflicts} submitting={submitting} />
      ) : null}

      <div className="grid grid-cols-12 gap-4">
        <aside className="col-span-3">
          <KindFilter active={data.kindFilter} />
        </aside>
        <section className="col-span-9 grid grid-cols-2 gap-4">
          <ArtifactList
            artifacts={data.artifacts}
            selectedId={data.selectedId}
            kindFilter={data.kindFilter}
          />
          <ArtifactDrawer
            selected={data.selected}
            loadError={data.loadError}
            submitting={submitting}
          />
        </section>
      </div>
    </div>
  );
}

function KindFilter({ active }: { active: ArtifactKind | "all" }) {
  return (
    <nav className="rounded border border-gray-200 bg-white p-3">
      <h2 className="mb-2 text-xs font-semibold uppercase text-gray-500">
        Filter by kind
      </h2>
      <ul className="space-y-1 text-sm">
        {KIND_FILTERS.map((k) => {
          const params = new URLSearchParams();
          if (k !== "all") params.set("kind", k);
          return (
            <li key={k}>
              <a
                href={`?${params.toString()}`}
                className={`block rounded px-2 py-1 ${
                  active === k
                    ? "bg-gray-900 text-white"
                    : "hover:bg-gray-100"
                }`}
              >
                {k}
              </a>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}

function ArtifactList({
  artifacts,
  selectedId,
  kindFilter,
}: {
  artifacts: ArtifactSummary[];
  selectedId: string | null;
  kindFilter: ArtifactKind | "all";
}) {
  if (artifacts.length === 0) {
    return (
      <div className="rounded border border-gray-200 bg-white p-6 text-sm text-gray-500">
        No artifacts in this kind. Run an upstream sync.
      </div>
    );
  }
  return (
    <ul className="max-h-[70vh] overflow-y-auto rounded border border-gray-200 bg-white">
      {artifacts.map((a) => {
        const params = new URLSearchParams();
        if (kindFilter !== "all") params.set("kind", kindFilter);
        params.set("id", a.id);
        const isSelected = a.id === selectedId;
        return (
          <li
            key={a.id}
            className={`border-b border-gray-100 px-3 py-2 text-sm ${
              isSelected ? "bg-blue-50" : "hover:bg-gray-50"
            }`}
          >
            <a href={`?${params.toString()}`} className="block">
              <div className="font-mono text-xs text-gray-500">{a.origin}</div>
              <div className="font-medium">{a.path}</div>
              <div className="mt-1 flex gap-2 text-xs text-gray-500">
                <span>kind={a.kind}</span>
                <span>v{a.version}</span>
                {a.hasOverride ? (
                  <span className="font-semibold text-amber-600">
                    override
                  </span>
                ) : null}
                {a.conflictState === "diverged" ? (
                  <span className="font-semibold text-red-600">diverged</span>
                ) : null}
                {a.status === "retired" ? (
                  <span className="text-gray-400">retired</span>
                ) : null}
              </div>
            </a>
          </li>
        );
      })}
    </ul>
  );
}

function ArtifactDrawer({
  selected,
  loadError,
  submitting,
}: {
  selected: ArtifactDetail | null;
  loadError: string | null;
  submitting: boolean;
}) {
  if (loadError) {
    return (
      <div className="rounded border border-red-200 bg-red-50 p-4 text-sm text-red-700">
        Failed to load artifact: {loadError}
      </div>
    );
  }
  if (!selected) {
    return (
      <div className="rounded border border-gray-200 bg-white p-4 text-sm text-gray-500">
        Select an artifact to view its body and override status.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3 rounded border border-gray-200 bg-white p-4 text-sm">
      <header>
        <h2 className="font-semibold">{selected.path}</h2>
        <div className="mt-1 flex flex-wrap gap-2 text-xs text-gray-500">
          <span>origin={selected.origin}</span>
          <span>kind={selected.kind}</span>
          <span>v{selected.version}</span>
          <span>hash={selected.contentHash.slice(0, 12)}…</span>
          {selected.conflictState === "diverged" ? (
            <span className="font-semibold text-red-600">diverged</span>
          ) : null}
        </div>
      </header>

      <Form method="post" className="flex flex-col gap-2">
        <input type="hidden" name="id" value={selected.id} />
        <label className="text-xs uppercase text-gray-500">User body</label>
        <textarea
          name="userBody"
          defaultValue={selected.userBody ?? selected.upstreamBody ?? ""}
          className="h-72 w-full rounded border border-gray-300 p-2 font-mono text-xs"
        />
        <div className="flex gap-2">
          <button
            type="submit"
            name="intent"
            value="save_override"
            disabled={submitting}
            className="rounded bg-gray-900 px-3 py-1 text-white disabled:opacity-50"
          >
            Save override
          </button>
          {selected.userBody !== null ? (
            <button
              type="submit"
              name="intent"
              value="clear_override"
              disabled={submitting}
              className="rounded border border-gray-300 px-3 py-1 text-gray-700 disabled:opacity-50"
            >
              Clear override
            </button>
          ) : null}
        </div>
      </Form>

      <details className="text-xs">
        <summary className="cursor-pointer text-gray-500">
          Upstream body (read-only)
        </summary>
        <pre className="mt-2 max-h-72 overflow-y-auto rounded bg-gray-50 p-2 font-mono">
          {selected.upstreamBody ?? "(null)"}
        </pre>
      </details>
    </div>
  );
}

function ConflictsPanel({
  conflicts,
  submitting,
}: {
  conflicts: ArtifactConflictSummary[];
  submitting: boolean;
}) {
  return (
    <section className="rounded border border-red-200 bg-red-50 p-3">
      <header className="mb-2 flex items-center justify-between">
        <h2 className="font-semibold text-red-800">
          {conflicts.length} divergent artifact{conflicts.length === 1 ? "" : "s"}
        </h2>
        <p className="text-xs text-red-700">
          Upstream changed after your override. Pick a resolution per row.
        </p>
      </header>
      <ul className="space-y-3">
        {conflicts.map((c) => (
          <li
            key={c.id}
            className="rounded bg-white px-3 py-2 text-sm"
          >
            <div className="flex items-center justify-between">
              <div>
                <div className="font-mono text-xs text-gray-500">
                  {c.origin}
                </div>
                <div className="font-medium">{c.path}</div>
              </div>
              <Form method="post" className="flex gap-2">
                <input type="hidden" name="id" value={c.id} />
                <button
                  type="submit"
                  name="intent"
                  value="resolve_keep_mine"
                  disabled={submitting}
                  className="rounded border border-gray-300 px-2 py-1 text-xs disabled:opacity-50"
                >
                  Keep mine
                </button>
                <button
                  type="submit"
                  name="intent"
                  value="resolve_take_upstream"
                  disabled={submitting}
                  className="rounded border border-gray-300 px-2 py-1 text-xs disabled:opacity-50"
                >
                  Take upstream
                </button>
              </Form>
            </div>
            <div className="mt-2">
              <ArtifactMergeEditor conflict={c} submitting={submitting} />
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
