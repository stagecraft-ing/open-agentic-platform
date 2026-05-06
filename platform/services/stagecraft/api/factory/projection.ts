// Spec 139 Phase 1 — projection.ts
//
// View-builder that reads substrate rows and emits the legacy
// spec 108 `factory_adapters` / `factory_contracts` / `factory_processes`
// JSONB shapes verbatim. The dual-write window keeps spec 108's external
// API surface (`/api/factory/{adapters,contracts,processes}`) byte-stable
// while the substrate is the authoritative store underneath.
//
// Phase 4 (T091) replaces `browse.ts` with substrate-direct reads and
// removes this module.
//
// **Round-trip parity contract (T010):** for any input that
// `translateUpstreams(...)` produces from a fixture tree, the
// `projectSubstrateToLegacy(translateUpstreamsToSubstrate(...))` chain
// must produce the same `TranslationResult` (modulo skill-key insertion
// order — see `projection.test.ts` `normalize`).

import { basename } from "node:path";
import { parse as parseYaml } from "yaml";
import type {
  AdapterTranslation,
  ContractTranslation,
  ProcessTranslation,
  SubstrateRowDraft,
  SubstrateTranslation,
  TranslationResult,
} from "./translator";

// ---------------------------------------------------------------------------
// Internal helpers — mirror legacy `translator.ts` predicates so the
// projection re-buckets substrate rows by path the same way.
// ---------------------------------------------------------------------------

const STAGE_FILENAME = /^factory-orchestration-(s\d+|tm|cd|xf)\.md$/;

function stageId(rel: string): string | null {
  if (!/^Factory Agent\/Orchestrator\/.+\.md$/.test(rel)) return null;
  const m = STAGE_FILENAME.exec(basename(rel));
  return m ? m[1] : null;
}

function isFactoryControllers(rel: string): boolean {
  return /^Factory Agent\/Controllers\/.+\.md$/.test(rel);
}
function isFactoryClientInterface(rel: string): boolean {
  return /^Factory Agent\/Client_Interface\/.+\.md$/.test(rel);
}
function isFactoryReqSystem(rel: string): boolean {
  return /^Factory Agent\/Requirements\/System\/.+\.md$/.test(rel);
}
function isFactoryReqService(rel: string): boolean {
  return /^Factory Agent\/Requirements\/Service\/.+\.md$/.test(rel);
}
function isFactoryReqClient(rel: string): boolean {
  return /^Factory Agent\/Requirements\/Client\/.+\.md$/.test(rel);
}
function isFactoryDatabase(rel: string): boolean {
  return /^Factory Agent\/Database\/.+\.md$/.test(rel);
}
function isFactoryReferenceJson(rel: string): boolean {
  return /^Factory Agent\/Requirements\/(Service|System)\/.+\.json$/.test(rel);
}
function isFactoryOtherAgent(rel: string): boolean {
  // Orchestrator/<file>.md that isn't a stage AND isn't the root
  // orchestrator. Mirrors `translateFactorySource`'s "otherAgents" bucket.
  return (
    /^Factory Agent\/Orchestrator\/.+\.md$/.test(rel) && stageId(rel) === null
  );
}

function isContractSchema(rel: string): boolean {
  return /\.(schema)\.(json|ya?ml)$/i.test(rel);
}

function deriveContractName(path: string): string {
  const base = basename(path).replace(/\.schema\.(json|ya?ml)$/, "");
  return base || path;
}

const SHA40 = /^[0-9a-f]{40}$/i;
function resolveDefaultBranch(ref: string | undefined): string {
  if (!ref || SHA40.test(ref)) return "main";
  return ref;
}

// ---------------------------------------------------------------------------
// Public projection
// ---------------------------------------------------------------------------

/**
 * Project a substrate translation back into the spec 108 categorical
 * `TranslationResult`. Pure function — same input, same output, modulo
 * ordering normalisation in the test (`projection.test.ts`).
 */
export function projectSubstrateToLegacy(
  input: SubstrateTranslation,
): TranslationResult {
  const factoryRows = input.rows.filter(
    (r) => r.origin === input.factoryOriginId,
  );
  const templateRows = input.rows.filter(
    (r) => r.origin === input.templateOriginId,
  );
  // OAP-owned contract schemas under `crates/factory-contracts/schemas/`
  // ride alongside the upstream contracts. They aren't tracked by either
  // upstream repo so the substrate sync surfaces them under a third
  // origin (`oap-self`); the projection merges them into the legacy
  // `contracts` array. See `oapContracts.ts` for the ingest path.
  const oapSelfRows = input.rows.filter((r) => r.origin === "oap-self");

  // Adapter set: the synthetic `aim-vue-node` from the template upstream
  // PLUS one adapter per OAP-native `adapter-manifest` substrate row.
  // De-duplicated by name (template wins on collision — it carries
  // skill/orchestrator content shape; oap-self adapters carry their own
  // manifest.yaml shape).
  const adapters: AdapterTranslation[] = [buildAdapter(templateRows, input)];
  for (const adapter of buildOapNativeAdapters(oapSelfRows)) {
    if (!adapters.some((a) => a.name === adapter.name)) {
      adapters.push(adapter);
    }
  }

  return {
    adapters,
    processes: [buildProcess(factoryRows, input)],
    contracts: buildContracts(factoryRows, templateRows, oapSelfRows, input),
  };
}

/**
 * Project OAP-native adapter substrate rows into the legacy
 * `AdapterTranslation` shape — one adapter per `adapter-manifest` row,
 * keyed by the directory name embedded in the substrate path.
 */
function buildOapNativeAdapters(
  oapSelfRows: SubstrateRowDraft[],
): AdapterTranslation[] {
  const out: AdapterTranslation[] = [];
  for (const manifestRow of oapSelfRows) {
    if (manifestRow.kind !== "adapter-manifest") continue;
    const m = /^adapters\/([^/]+)\/manifest\.yaml$/.exec(manifestRow.path);
    if (!m) continue;
    const adapterName = m[1];

    let manifestObj: Record<string, unknown> = {};
    try {
      const parsed = parseYaml(manifestRow.upstreamBody);
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        manifestObj = parsed as Record<string, unknown>;
      }
    } catch {
      // Malformed YAML — preserve the raw body so the wire shape is
      // non-empty and the operator can debug. Substrate row stays intact.
      manifestObj = { raw: manifestRow.upstreamBody };
    }

    // Companion content (patterns, agents, invariants) for the adapter,
    // exposed alongside the manifest so consumers don't need a second
    // round-trip. Mirrors how `buildAdapter` exposes template skills.
    const prefix = `adapters/${adapterName}/`;
    const patterns: Record<string, { path: string; body: string }> = {};
    const agents: Record<string, { path: string; body: string }> = {};
    let invariants: { path: string; body: string } | null = null;
    for (const row of oapSelfRows) {
      if (!row.path.startsWith(prefix) || row === manifestRow) continue;
      if (row.kind === "pattern") {
        const key = row.path.slice(prefix.length);
        patterns[key] = { path: row.path, body: row.upstreamBody };
      } else if (row.kind === "agent" || row.kind === "skill") {
        const key = row.path.slice(prefix.length);
        agents[key] = { path: row.path, body: row.upstreamBody };
      } else if (row.kind === "invariant") {
        invariants = { path: row.path, body: row.upstreamBody };
      }
    }

    const enrichedManifest: Record<string, unknown> = {
      ...manifestObj,
      __companion: { patterns, agents, invariants },
    };

    out.push({
      name: adapterName,
      version: manifestRow.contentHash.slice(0, 12),
      sourceSha: manifestRow.upstreamSha || `oap-self/${adapterName}`,
      manifest: enrichedManifest,
    });
  }
  out.sort((a, b) => a.name.localeCompare(b.name));
  return out;
}

function buildAdapter(
  templateRows: SubstrateRowDraft[],
  input: SubstrateTranslation,
): AdapterTranslation {
  const orchestratorRow =
    templateRows.find(
      (r) => r.path === "orchestration/template-orchestrator.md",
    ) ?? null;
  const skills: Record<string, { path: string; body: string }> = {};
  for (const row of templateRows) {
    const m = /^orchestration\/skills\/(.+)\.md$/.exec(row.path);
    if (m) {
      skills[m[1]] = { path: row.path, body: row.upstreamBody };
    }
  }
  const manifest: Record<string, unknown> = {
    entry: "orchestration/template-orchestrator.md",
    orchestrator: orchestratorRow
      ? { path: orchestratorRow.path, body: orchestratorRow.upstreamBody }
      : null,
    skills,
  };
  if (input.templateRemote) {
    manifest.template_remote = input.templateRemote;
    manifest.template_default_branch = resolveDefaultBranch(
      input.templateDefaultBranch,
    );
  }
  return {
    name: "aim-vue-node",
    version: input.templateSourceSha.slice(0, 12),
    sourceSha: input.templateSourceSha,
    manifest,
  };
}

function buildProcess(
  factoryRows: SubstrateRowDraft[],
  input: SubstrateTranslation,
): ProcessTranslation {
  // Re-bucket by path predicate, exactly mirroring `translateFactorySource`.
  const stages: Array<{ id: string; path: string; body: string }> = [];
  const controllers: Array<{ path: string; body: string }> = [];
  const clientInterface: Array<{ path: string; body: string }> = [];
  const requirementsSystem: Array<{ path: string; body: string }> = [];
  const requirementsService: Array<{ path: string; body: string }> = [];
  const requirementsClient: Array<{ path: string; body: string }> = [];
  const database: Array<{ path: string; body: string }> = [];
  const otherAgents: Array<{ path: string; body: string }> = [];
  const references: Array<{ path: string; body: string }> = [];
  let rootOrchestrator: { path: string; body: string } | null = null;

  for (const row of factoryRows) {
    if (isContractSchema(row.path)) continue; // contracts are a separate output bucket
    if (row.path === "Factory Agent/factory-orchestration.md") {
      rootOrchestrator = { path: row.path, body: row.upstreamBody };
      continue;
    }
    const sId = stageId(row.path);
    if (sId) {
      stages.push({ id: sId, path: row.path, body: row.upstreamBody });
      continue;
    }
    if (isFactoryControllers(row.path)) {
      controllers.push({ path: row.path, body: row.upstreamBody });
      continue;
    }
    if (isFactoryClientInterface(row.path)) {
      clientInterface.push({ path: row.path, body: row.upstreamBody });
      continue;
    }
    if (isFactoryReqSystem(row.path)) {
      requirementsSystem.push({ path: row.path, body: row.upstreamBody });
      continue;
    }
    if (isFactoryReqService(row.path)) {
      requirementsService.push({ path: row.path, body: row.upstreamBody });
      continue;
    }
    if (isFactoryReqClient(row.path)) {
      requirementsClient.push({ path: row.path, body: row.upstreamBody });
      continue;
    }
    if (isFactoryDatabase(row.path)) {
      database.push({ path: row.path, body: row.upstreamBody });
      continue;
    }
    if (isFactoryOtherAgent(row.path)) {
      otherAgents.push({ path: row.path, body: row.upstreamBody });
      continue;
    }
    if (isFactoryReferenceJson(row.path)) {
      references.push({ path: row.path, body: row.upstreamBody });
      continue;
    }
    // Anything else falls outside the legacy categorical model and is
    // ignored by the projection (substrate still has the row; the
    // projection's job is to mirror the legacy shape, not extend it).
  }

  // Same sort discipline as `translateFactorySource`.
  const sortByPath = (
    a: { path: string },
    b: { path: string },
  ) => a.path.localeCompare(b.path);
  stages.sort((a, b) => a.id.localeCompare(b.id));
  controllers.sort(sortByPath);
  clientInterface.sort(sortByPath);
  requirementsSystem.sort(sortByPath);
  requirementsService.sort(sortByPath);
  requirementsClient.sort(sortByPath);
  database.sort(sortByPath);
  otherAgents.sort(sortByPath);
  references.sort(sortByPath);

  return {
    name: "7-stage-build",
    version: input.factorySourceSha.slice(0, 12),
    sourceSha: input.factorySourceSha,
    definition: {
      orchestrator: rootOrchestrator,
      stages,
      agents: {
        controllers,
        client_interface: clientInterface,
        requirements: {
          system: requirementsSystem,
          service: requirementsService,
          client: requirementsClient,
        },
        database,
        other: otherAgents,
      },
      references,
    },
  };
}

function buildContracts(
  factoryRows: SubstrateRowDraft[],
  templateRows: SubstrateRowDraft[],
  oapSelfRows: SubstrateRowDraft[],
  input: SubstrateTranslation,
): ContractTranslation[] {
  const factorySchemaRows = factoryRows
    .filter((r) => isContractSchema(r.path))
    .sort((a, b) => a.path.localeCompare(b.path))
    .map<ContractTranslation>((row) => ({
      name: deriveContractName(row.path),
      version: input.factorySourceSha.slice(0, 12),
      sourceSha: input.factorySourceSha,
      schema: { path: row.path, body: row.upstreamBody },
    }));
  const templateSchemaRows = templateRows
    .filter((r) => isContractSchema(r.path))
    .sort((a, b) => a.path.localeCompare(b.path))
    .map<ContractTranslation>((row) => ({
      name: deriveContractName(row.path),
      version: input.templateSourceSha.slice(0, 12),
      sourceSha: input.templateSourceSha,
      schema: { path: row.path, body: row.upstreamBody },
    }));
  const oapSelfSchemaRows = oapSelfRows
    .filter((r) => r.kind === "contract-schema")
    .sort((a, b) => a.path.localeCompare(b.path))
    .map<ContractTranslation>((row) => ({
      name: deriveContractName(row.path),
      // OAP-self schemas are content-versioned via `contentHash`. Use
      // the short hash as the wire-shape `version` slug so consumers
      // can detect content changes; `sourceSha` carries the substrate's
      // stable upstream-sha stamp for cross-row consistency.
      version: row.contentHash.slice(0, 12),
      sourceSha: row.upstreamSha || "oap-self/contract-schemas",
      schema: { path: row.path, body: row.upstreamBody },
    }));

  // Dedup by name; first-seen wins. Factory wins over template (matches
  // legacy `translateUpstreams` ordering); upstream wins over OAP-self
  // when both define the same schema name (theoretical case — today
  // upstream repos carry zero schemas, but the precedence is explicit).
  const byName = new Map<string, ContractTranslation>();
  for (const c of [
    ...factorySchemaRows,
    ...templateSchemaRows,
    ...oapSelfSchemaRows,
  ]) {
    if (!byName.has(c.name)) byName.set(c.name, c);
  }
  return Array.from(byName.values());
}
