/**
 * Factory Processes browser (spec 108 Phase 4).
 *
 * List of the org's process definitions with a detail drawer showing the
 * full definition JSON, source sha, and synced_at timestamp.
 */

import { useLoaderData } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getFactoryProcess,
  listFactoryProcesses,
  type FactoryProcessDetail,
  type FactoryResourceSummary,
} from "../lib/factory-api.server";
import {
  FactoryBrowser,
  type FactoryBrowserDetail,
} from "../components/factory-browser";

type LoaderData = {
  processes: FactoryResourceSummary[];
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

  const { processes } = await listFactoryProcesses(request);

  let selected: FactoryBrowserDetail | null = null;
  let loadError: string | null = null;
  if (selectedName) {
    try {
      const detail: FactoryProcessDetail = await getFactoryProcess(
        request,
        selectedName
      );
      selected = {
        name: detail.name,
        version: detail.version,
        sourceSha: detail.sourceSha,
        syncedAt: detail.syncedAt,
        body: detail.definition,
      };
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    }
  }

  return { processes, selectedName, selected, loadError };
}

export default function FactoryProcesses() {
  const { processes, selected, selectedName, loadError } =
    useLoaderData<typeof loader>();

  return (
    <FactoryBrowser
      items={processes}
      selected={selected}
      selectedName={selectedName}
      resourceKind="process"
      bodyLabel="Definition"
      loadError={loadError}
      emptyCopy={{
        title: "No processes yet",
        description:
          "Processes appear here after the first successful sync of the factory upstreams.",
      }}
    />
  );
}
