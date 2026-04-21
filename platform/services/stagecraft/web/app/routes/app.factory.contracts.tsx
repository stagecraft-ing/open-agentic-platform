/**
 * Factory Contracts browser (spec 108 Phase 4).
 *
 * List of the org's contract schemas with a detail drawer showing the JSON
 * schema body, source sha, and synced_at timestamp.
 */

import { useLoaderData } from "react-router";
import { requireUser } from "../lib/auth.server";
import {
  getFactoryContract,
  listFactoryContracts,
  type FactoryContractDetail,
  type FactoryResourceSummary,
} from "../lib/factory-api.server";
import {
  FactoryBrowser,
  type FactoryBrowserDetail,
} from "../components/factory-browser";

type LoaderData = {
  contracts: FactoryResourceSummary[];
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

  const { contracts } = await listFactoryContracts(request);

  let selected: FactoryBrowserDetail | null = null;
  let loadError: string | null = null;
  if (selectedName) {
    try {
      const detail: FactoryContractDetail = await getFactoryContract(
        request,
        selectedName
      );
      selected = {
        name: detail.name,
        version: detail.version,
        sourceSha: detail.sourceSha,
        syncedAt: detail.syncedAt,
        body: detail.schema,
      };
    } catch (err) {
      loadError = err instanceof Error ? err.message : String(err);
    }
  }

  return { contracts, selectedName, selected, loadError };
}

export default function FactoryContracts() {
  const { contracts, selected, selectedName, loadError } =
    useLoaderData<typeof loader>();

  return (
    <FactoryBrowser
      items={contracts}
      selected={selected}
      selectedName={selectedName}
      resourceKind="contract"
      bodyLabel="Schema"
      loadError={loadError}
      emptyCopy={{
        title: "No contracts yet",
        description:
          "Contracts appear here after the first successful sync of the factory upstreams.",
      }}
    />
  );
}
