/**
 * Factory Adapters browser (spec 108 Phase 4).
 *
 * List of the org's adapters with a detail drawer showing the manifest JSON,
 * source sha, and synced_at timestamp. Read-only — edits happen upstream and
 * land via the sync worker.
 */

import { useLoaderData } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getFactoryAdapter,
  listFactoryAdapters,
  type FactoryAdapterDetail,
  type FactoryResourceSummary,
} from "../lib/factory-api.server";
import {
  FactoryBrowser,
  type FactoryBrowserDetail,
} from "../components/factory-browser";

type LoaderData = {
  adapters: FactoryResourceSummary[];
  selectedName: string | null;
  selected: FactoryBrowserDetail | null;
  loadError: string | null;
};

export async function loader({
  request,
}: {
  request: Request;
}): Promise<LoaderData> {
  await requireUser(request);
  const url = new URL(request.url);
  const selectedName = url.searchParams.get("name");

  const { adapters } = await listFactoryAdapters(request);

  let selected: FactoryBrowserDetail | null = null;
  let loadError: string | null = null;
  if (selectedName) {
    try {
      const detail: FactoryAdapterDetail = await getFactoryAdapter(
        request,
        selectedName
      );
      selected = {
        name: detail.name,
        version: detail.version,
        sourceSha: detail.sourceSha,
        syncedAt: detail.syncedAt,
        body: detail.manifest,
      };
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    }
  }

  return { adapters, selectedName, selected, loadError };
}

export default function FactoryAdapters() {
  const { adapters, selected, selectedName, loadError } =
    useLoaderData<typeof loader>();

  return (
    <FactoryBrowser
      items={adapters}
      selected={selected}
      selectedName={selectedName}
      resourceKind="adapter"
      bodyLabel="Manifest"
      loadError={loadError}
      emptyCopy={{
        title: "No adapters yet",
        description:
          "Adapters appear here after the first successful sync of the factory upstreams.",
      }}
    />
  );
}
