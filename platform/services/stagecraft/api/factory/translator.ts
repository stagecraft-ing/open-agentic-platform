import { readdir, readFile, stat } from "node:fs/promises";
import { join, relative, basename } from "node:path";
import { randomUUID } from "node:crypto";

// ---------------------------------------------------------------------------
// Spec 108 Phase 3 — deterministic translation from upstream repos to the
// factory_adapters / factory_contracts / factory_processes tables.
//
// The translation lifts spec 088 §5 exclusion rules into code and produces a
// single snapshot per run:
//   - one factory_processes row ("7-stage-build") derived from the factory
//     source's Factory Agent/ tree
//   - one factory_adapters row ("aim-vue-node") derived from the template
//     repo's orchestration/ tree
//   - zero or more factory_contracts rows, one per *.schema.{json,yaml,yml}
//     discovered under either repo
//
// Everything is captured verbatim — body, source path, content hash — so the
// OPC contract in spec 108 §7 can replay adapter/process content without
// additional upstream fetches. We emit plain POJOs here; the caller wraps
// them in a DB transaction and handles org scoping.
// ---------------------------------------------------------------------------

export type AdapterTranslation = {
  name: string;
  version: string;
  sourceSha: string;
  manifest: Record<string, unknown>;
};

export type ProcessTranslation = {
  name: string;
  version: string;
  sourceSha: string;
  definition: Record<string, unknown>;
};

export type ContractTranslation = {
  name: string;
  version: string;
  sourceSha: string;
  schema: Record<string, unknown>;
};

export type TranslationResult = {
  adapters: AdapterTranslation[];
  contracts: ContractTranslation[];
  processes: ProcessTranslation[];
};

// ---------------------------------------------------------------------------
// Exclusion rules lifted from spec 088 §5
// ---------------------------------------------------------------------------

// Paths are evaluated against the repo-relative path (POSIX separators).
//
// Excludes lifted from spec 088 §5, updated for upstream-map.yaml v2.0.0
// (2026-04-24 — GovAlta-Pronghorn/goa-software-factory):
//   - factory-orchestration-cd.md is NO LONGER excluded. It is a real
//     optional stage file (Client Documentation) in the new upstream and
//     the translator captures it via stageIdFromFilename.
//   - sitemap-template-*.json are NO LONGER excluded. They are canonical
//     variant baselines in the new upstream and are captured as JSON
//     reference assets.
//   - Factory Agent/Requirements/Client/ is NO LONGER excluded. Client
//     Documentation sub-agents are captured as "client" requirements
//     agents alongside System/ and Service/.
//   - .claude/ is added to the exclude list (project tooling, not factory
//     surface).
const FACTORY_SOURCE_EXCLUDES: Array<(rel: string) => boolean> = [
  (p) => p === ".git" || p.startsWith(".git/"),
  (p) => p === ".github" || p.startsWith(".github/"),
  (p) => p === ".claude" || p.startsWith(".claude/"),
  (p) => p === "README.md" || p === ".project" || p === ".env.github",
  (p) => p.startsWith("eval_framework/"),
  (p) => p.startsWith("REDTEAM/"),
  (p) => p.startsWith("Security Agent/"),
  (p) => p.startsWith("Factory Agent/Security/"),
  (p) => p.startsWith("Factory Agent/Orchestrator/scripts/"),
  (p) => p === "Factory Agent/Controllers/api-web-standards.md",
  (p) => p === "Factory Agent/Controllers/api-standards-compliance.md",
];

const TEMPLATE_EXCLUDES: Array<(rel: string) => boolean> = [
  (p) => p === ".git" || p.startsWith(".git/"),
  (p) => p === ".github" || p.startsWith(".github/"),
  (p) => p === ".claude" || p.startsWith(".claude/"),
  (p) => p === "node_modules" || p.startsWith("node_modules/"),
  (p) => p === "apps" || p.startsWith("apps/"),
  (p) => p === "packages" || p.startsWith("packages/"),
  (p) => p === "modules" || p.startsWith("modules/"),
  (p) => p === "scripts" || p.startsWith("scripts/"),
  (p) => p === "docker" || p.startsWith("docker/"),
  (p) => p === "docs" || p.startsWith("docs/"),
  (p) => p === "README.md" || p === "CODEMAP.md" || p === "PLACEHOLDERS.md",
  (p) => p === "docker-compose.yml" || p === "eslint.config.mjs",
  (p) => p === "tsconfig.base.json" || p === "package.json",
  (p) => p === "template.json",
  (p) => /(^|\/)package-lock\.json$/.test(p),
];

// ---------------------------------------------------------------------------
// Filesystem walker — yields POSIX-relative paths for files only, respecting
// an exclusion predicate evaluated against each relative path.
// ---------------------------------------------------------------------------

async function* walk(
  root: string,
  excluded: (rel: string) => boolean
): AsyncGenerator<{ rel: string; abs: string }> {
  async function* recurse(dir: string): AsyncGenerator<{ rel: string; abs: string }> {
    const entries = await readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const abs = join(dir, entry.name);
      const rel = relative(root, abs).split(/\\|\//).join("/");
      if (excluded(rel)) continue;
      if (entry.isDirectory()) {
        yield* recurse(abs);
      } else if (entry.isFile()) {
        yield { rel, abs };
      }
    }
  }
  yield* recurse(root);
}

async function readText(abs: string): Promise<string> {
  return readFile(abs, "utf8");
}

// ---------------------------------------------------------------------------
// Factory source → process + contracts
// ---------------------------------------------------------------------------

type CapturedFile = { path: string; body: string };

function stageIdFromFilename(name: string): string | null {
  // s1..s5 — main 5-stage pipeline
  // tm    — Template Mode detection (delegates Stages 4–5 to template)
  // cd    — Client Documentation (optional, added in upstream-map v2.0.0)
  // xf    — Pipeline completion / factory-manifest (added in upstream-map v2.0.0)
  const m = /^factory-orchestration-(s\d+|tm|cd|xf)\.md$/.exec(name);
  return m ? m[1] : null;
}

export async function translateFactorySource(
  repoPath: string,
  sourceSha: string
): Promise<{
  process: ProcessTranslation;
  contracts: ContractTranslation[];
}> {
  const stages: Array<{ id: string; path: string; body: string }> = [];
  const controllers: CapturedFile[] = [];
  const clientInterface: CapturedFile[] = [];
  const requirementsSystem: CapturedFile[] = [];
  const requirementsService: CapturedFile[] = [];
  const requirementsClient: CapturedFile[] = [];
  const database: CapturedFile[] = [];
  const otherAgents: CapturedFile[] = [];
  const references: CapturedFile[] = [];
  const contractFiles: CapturedFile[] = [];
  let rootOrchestrator: CapturedFile | null = null;

  for await (const { rel, abs } of walk(repoPath, (p) =>
    FACTORY_SOURCE_EXCLUDES.some((fn) => fn(p))
  )) {
    if (/\.(schema)\.(json|ya?ml)$/.test(rel)) {
      contractFiles.push({ path: rel, body: await readText(abs) });
      continue;
    }

    if (rel === "Factory Agent/factory-orchestration.md") {
      rootOrchestrator = { path: rel, body: await readText(abs) };
      continue;
    }

    if (/^Factory Agent\/Orchestrator\/.+\.md$/.test(rel)) {
      const id = stageIdFromFilename(basename(rel));
      if (id) {
        stages.push({ id, path: rel, body: await readText(abs) });
      } else {
        otherAgents.push({ path: rel, body: await readText(abs) });
      }
      continue;
    }

    if (/^Factory Agent\/Controllers\/.+\.md$/.test(rel)) {
      controllers.push({ path: rel, body: await readText(abs) });
      continue;
    }

    if (/^Factory Agent\/Client_Interface\/.+\.md$/.test(rel)) {
      clientInterface.push({ path: rel, body: await readText(abs) });
      continue;
    }

    if (/^Factory Agent\/Requirements\/System\/.+\.md$/.test(rel)) {
      requirementsSystem.push({ path: rel, body: await readText(abs) });
      continue;
    }

    if (/^Factory Agent\/Requirements\/Service\/.+\.md$/.test(rel)) {
      requirementsService.push({ path: rel, body: await readText(abs) });
      continue;
    }

    if (/^Factory Agent\/Requirements\/Client\/.+\.md$/.test(rel)) {
      requirementsClient.push({ path: rel, body: await readText(abs) });
      continue;
    }

    if (/^Factory Agent\/Database\/.+\.md$/.test(rel)) {
      database.push({ path: rel, body: await readText(abs) });
      continue;
    }

    // Load-bearing JSON reference assets (not schemas): sitemap variant
    // templates and admin-interface base requirements. These are referenced
    // by stage skills and need to travel through the sync so adapters and
    // OPC cockpit actions can resolve them without re-cloning the factory.
    if (/^Factory Agent\/Requirements\/(Service|System)\/.+\.json$/.test(rel)) {
      references.push({ path: rel, body: await readText(abs) });
      continue;
    }
  }

  stages.sort((a, b) => a.id.localeCompare(b.id));
  const sortByPath = (a: CapturedFile, b: CapturedFile) =>
    a.path.localeCompare(b.path);
  controllers.sort(sortByPath);
  clientInterface.sort(sortByPath);
  requirementsSystem.sort(sortByPath);
  requirementsService.sort(sortByPath);
  requirementsClient.sort(sortByPath);
  database.sort(sortByPath);
  otherAgents.sort(sortByPath);
  references.sort(sortByPath);
  contractFiles.sort(sortByPath);

  const process: ProcessTranslation = {
    name: "7-stage-build",
    version: sourceSha.slice(0, 12),
    sourceSha,
    definition: {
      orchestrator: rootOrchestrator,
      stages,
      agents: {
        controllers,
        client_interface: clientInterface,
        // Split Requirements/ into sub-buckets so consumers can tell
        // business (System), service, and client-documentation agents
        // apart without string-matching paths.
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

  const contracts: ContractTranslation[] = contractFiles.map((f) => ({
    name: deriveContractName(f.path),
    version: sourceSha.slice(0, 12),
    sourceSha,
    schema: {
      path: f.path,
      body: f.body,
    },
  }));

  return { process, contracts };
}

function deriveContractName(path: string): string {
  // Strip the .schema.{json,yaml,yml} suffix and return the basename.
  const base = basename(path).replace(/\.schema\.(json|ya?ml)$/, "");
  return base || path;
}

// ---------------------------------------------------------------------------
// Template repo → adapter
// ---------------------------------------------------------------------------

export async function translateTemplate(
  repoPath: string,
  sourceSha: string
): Promise<{
  adapter: AdapterTranslation;
  contracts: ContractTranslation[];
}> {
  const skills: Record<string, { path: string; body: string }> = {};
  const contractFiles: CapturedFile[] = [];
  let orchestrator: CapturedFile | null = null;

  for await (const { rel, abs } of walk(repoPath, (p) =>
    TEMPLATE_EXCLUDES.some((fn) => fn(p))
  )) {
    if (/\.(schema)\.(json|ya?ml)$/.test(rel)) {
      contractFiles.push({ path: rel, body: await readText(abs) });
      continue;
    }

    if (rel === "orchestration/template-orchestrator.md") {
      orchestrator = { path: rel, body: await readText(abs) };
      continue;
    }

    const skillMatch = /^orchestration\/skills\/([^/]+)\.md$/.exec(rel);
    if (skillMatch) {
      const id = skillMatch[1];
      skills[id] = { path: rel, body: await readText(abs) };
      continue;
    }
  }

  const adapter: AdapterTranslation = {
    name: "aim-vue-node",
    version: sourceSha.slice(0, 12),
    sourceSha,
    manifest: {
      entry: "orchestration/template-orchestrator.md",
      orchestrator,
      skills,
    },
  };

  contractFiles.sort((a, b) => a.path.localeCompare(b.path));
  const contracts: ContractTranslation[] = contractFiles.map((f) => ({
    name: deriveContractName(f.path),
    version: sourceSha.slice(0, 12),
    sourceSha,
    schema: {
      path: f.path,
      body: f.body,
    },
  }));

  return { adapter, contracts };
}

// ---------------------------------------------------------------------------
// Combined translator
// ---------------------------------------------------------------------------

export async function translateUpstreams(opts: {
  factorySourcePath: string;
  factorySourceSha: string;
  templatePath: string;
  templateSha: string;
}): Promise<TranslationResult> {
  // Verify both paths exist before doing real work. Fail fast with a clear
  // message so the caller can surface it as a sync error.
  for (const [label, path] of [
    ["factory source", opts.factorySourcePath],
    ["template", opts.templatePath],
  ] as const) {
    const s = await stat(path).catch(() => null);
    if (!s || !s.isDirectory()) {
      throw new Error(`${label} path is not a directory: ${path}`);
    }
  }

  const factory = await translateFactorySource(
    opts.factorySourcePath,
    opts.factorySourceSha
  );
  const template = await translateTemplate(opts.templatePath, opts.templateSha);

  // De-duplicate contracts by name, preferring factory source if both repos
  // carry the same schema. Version/sha disambiguation can come later.
  const byName = new Map<string, ContractTranslation>();
  for (const c of [...factory.contracts, ...template.contracts]) {
    if (!byName.has(c.name)) byName.set(c.name, c);
  }

  return {
    adapters: [template.adapter],
    processes: [factory.process],
    contracts: Array.from(byName.values()),
  };
}

// ---------------------------------------------------------------------------
// Legacy `goa-software-factory` manifest → ACP pipeline-state
//
// Spec 112 §3.4. Bridges the 5-stage legacy manifest produced by
// `goa-software-factory` into a pipeline-state.schema.yaml-conformant
// document that ACP consumers can read. Build Spec production is deferred:
// the legacy split `requirements/{ui,api}/build-spec.json` stay on disk
// and the first ACP run after translation emits a unified Build Spec
// alongside them.
//
// This function is pure: it does not read the filesystem and does not
// mutate its inputs. Callers own the upstream read (the detection crate's
// structured report carries `legacyManifest`) and the downstream write
// (the Import PR that adds `.factory/pipeline-state.json` to the repo).
// ---------------------------------------------------------------------------

export type GoaSoftwareFactoryManifest = {
  pipelineStatus?: string;
  completedAt?: string;
  factoryInputs?: Record<string, unknown>;
  stages?: Record<string, GoaStageEntry>;
  fileOwnership?: Record<string, unknown>;
  [key: string]: unknown;
};

export type GoaStageEntry = {
  status?: string;
  startedAt?: string;
  completedAt?: string;
  outputDirectory?: string;
  artifacts?: string[];
  summary?: Record<string, unknown>;
  [key: string]: unknown;
};

export type GoaWorkingState = {
  schemaVersion?: string;
  lastCompletedStage?: string;
  completedAt?: string;
  templateVariant?: string;
  factoryInputs?: Record<string, unknown>;
  [key: string]: unknown;
};

export type FactoryAdapterRow = {
  name: string;
  version: string;
  sourceSha?: string;
  manifest?: Record<string, unknown>;
};

// Schema-conformant output shape. Kept structural rather than importing a
// generated type because the Rust-owned schema is the source of truth and
// this codebase does not yet generate TS types from the YAML.
export type PipelineStateDocument = {
  schema_version: string;
  pipeline: {
    id: string;
    factory_version: string;
    started_at: string;
    updated_at: string;
    completed_at?: string;
    status: "running" | "paused" | "completed" | "failed" | "cancelled";
    adapter: { name: string; version: string };
    build_spec: { path: string; hash: string };
  };
  stages: Record<string, PipelineStageEntry>;
  audit?: Array<{
    timestamp: string;
    event:
      | "stage_confirmed"
      | "stage_rejected"
      | "feature_flagged"
      | "pipeline_paused"
      | "pipeline_resumed"
      | "adapter_overridden"
      | "manual_fix_applied";
    stage?: string;
    details?: string;
  }>;
};

export type PipelineStageEntry = {
  status: "pending" | "in_progress" | "completed" | "failed" | "skipped";
  started_at?: string;
  completed_at?: string;
  artifacts?: Array<{ path: string; type: string; hash: string }>;
};

// Legacy stage-key → ACP stage-id mapping (spec 112 §3.4).
export const LEGACY_STAGE_MAP: ReadonlyArray<[string, string]> = [
  ["stage1_businessRequirements", "business-requirements"],
  ["stage2_serviceRequirements", "service-requirements"],
  ["stage3_databaseDesign", "data-model"],
  ["stage4_apiControllers", "api-specification"],
  ["stage5_clientInterface", "ui-specification"],
];

// Status tokens the legacy manifest uses for terminal completion.
const LEGACY_TERMINAL = new Set(["PASSED", "PASS", "COMPLETE", "COMPLETED"]);

function mapLegacyStatus(
  raw: string | undefined
): PipelineStageEntry["status"] {
  if (!raw) return "pending";
  const up = raw.toUpperCase();
  if (LEGACY_TERMINAL.has(up)) return "completed";
  if (up === "FAILED" || up === "FAIL") return "failed";
  if (up === "SKIPPED") return "skipped";
  if (up === "IN_PROGRESS" || up === "RUNNING") return "in_progress";
  return "pending";
}

function inferArtifactType(path: string): string {
  const base = basename(path).toLowerCase();
  if (base.endsWith(".schema.json") || base.endsWith(".schema.yaml")) {
    return "schema";
  }
  if (base.endsWith("build-spec.json")) return "build-spec";
  if (base.endsWith(".md") || base.endsWith(".docx")) return "document";
  if (base.endsWith(".json")) return "data";
  return "artifact";
}

function selectAdapter(
  legacy: GoaSoftwareFactoryManifest,
  orgAdapters: FactoryAdapterRow[]
): { name: string; version: string } {
  // Heuristic (deterministic): prefer an adapter matching the legacy
  // factoryInputs.templateMode / templateVariant hint if one is available,
  // else the first adapter in the supplied list, else a conservative fallback.
  const inputs = legacy.factoryInputs ?? {};
  const clientStack = typeof inputs.clientTechStack === "string"
    ? inputs.clientTechStack.toLowerCase()
    : "";
  const preferred = orgAdapters.find((a) =>
    clientStack.includes("vue") && a.name.toLowerCase().includes("vue")
  );
  const picked = preferred ?? orgAdapters[0];
  if (picked) {
    return { name: picked.name, version: picked.version };
  }
  return { name: "aim-vue-node", version: "0.0.0" };
}

function pickPipelineTimestamps(
  legacy: GoaSoftwareFactoryManifest,
  workingState: GoaWorkingState,
  stageEntries: Array<[string, PipelineStageEntry]>
): { started_at: string; updated_at: string; completed_at?: string } {
  const started = stageEntries
    .map(([, entry]) => entry.started_at)
    .filter((s): s is string => Boolean(s))
    .sort()[0];
  const completedCandidates = [
    legacy.completedAt,
    workingState.completedAt,
    ...stageEntries
      .map(([, entry]) => entry.completed_at)
      .filter((s): s is string => Boolean(s)),
  ].filter((s): s is string => Boolean(s));
  const completed = completedCandidates.sort().pop();
  const fallback = new Date(0).toISOString();
  const started_at = started ?? completed ?? fallback;
  const updated_at = completed ?? started_at;
  return {
    started_at,
    updated_at,
    completed_at: completed,
  };
}

/**
 * Translate a legacy `goa-software-factory` manifest + working-state
 * pair into an ACP `pipeline-state.schema.yaml`-conformant document.
 *
 * Pure. Idempotent for the same inputs modulo the generated pipeline
 * UUID, which is fresh per invocation — callers that need stability
 * (e.g. re-translation during Import preview) should persist the first
 * result rather than regenerating.
 */
export function translateLegacyManifest(
  legacy: GoaSoftwareFactoryManifest,
  workingState: GoaWorkingState,
  orgAdapters: FactoryAdapterRow[]
): PipelineStateDocument {
  const stages: Record<string, PipelineStageEntry> = {};
  const stageEntries: Array<[string, PipelineStageEntry]> = [];

  // pre-flight — synthesised as completed (legacy runs presuppose it).
  const preflight: PipelineStageEntry = { status: "completed" };
  stages["pre-flight"] = preflight;
  stageEntries.push(["pre-flight", preflight]);

  // Map the five numbered legacy stages.
  const legacyStages = legacy.stages ?? {};
  let anyMapped = false;
  let anyIncomplete = false;
  for (const [legacyKey, acpId] of LEGACY_STAGE_MAP) {
    const raw = legacyStages[legacyKey];
    if (!raw) {
      const entry: PipelineStageEntry = { status: "pending" };
      stages[acpId] = entry;
      stageEntries.push([acpId, entry]);
      anyIncomplete = true;
      continue;
    }
    anyMapped = true;
    const status = mapLegacyStatus(raw.status);
    if (status !== "completed") anyIncomplete = true;
    const entry: PipelineStageEntry = { status };
    if (raw.startedAt) entry.started_at = raw.startedAt;
    if (raw.completedAt) entry.completed_at = raw.completedAt;
    if (Array.isArray(raw.artifacts) && raw.artifacts.length > 0) {
      entry.artifacts = raw.artifacts.map((p) => ({
        path: p,
        type: inferArtifactType(p),
        hash: "",
      }));
    }
    stages[acpId] = entry;
    stageEntries.push([acpId, entry]);
  }

  // adapter-handoff — synthesised from fileOwnership when present, else
  // reported as pending. This is consistent with spec 112 §3.4.
  const fileOwnership = legacy.fileOwnership;
  const handoff: PipelineStageEntry =
    fileOwnership && typeof fileOwnership === "object"
      ? { status: "completed" }
      : { status: "pending" };
  stages["adapter-handoff"] = handoff;
  stageEntries.push(["adapter-handoff", handoff]);

  // Overall pipeline status.
  let pipelineStatus: PipelineStateDocument["pipeline"]["status"];
  const legacyPipelineStatus = String(legacy.pipelineStatus ?? "").toUpperCase();
  if (!anyMapped || anyIncomplete) {
    pipelineStatus = "paused";
  } else if (legacyPipelineStatus === "COMPLETE") {
    pipelineStatus = "completed";
  } else {
    pipelineStatus = "paused";
  }

  const ts = pickPipelineTimestamps(legacy, workingState, stageEntries);
  const adapter = selectAdapter(legacy, orgAdapters);

  const document: PipelineStateDocument = {
    schema_version: "1.0.0",
    pipeline: {
      id: randomUUID(),
      factory_version: "legacy-translated",
      started_at: ts.started_at,
      updated_at: ts.updated_at,
      status: pipelineStatus,
      adapter,
      // Build Spec production is deferred. The legacy split
      // build-specs remain in place as informational artefacts; the
      // first ACP run emits a unified one. We encode this explicitly
      // rather than guessing a synthetic hash.
      build_spec: { path: "requirements/", hash: "" },
    },
    stages,
    audit: [
      {
        timestamp: ts.updated_at,
        event: "manual_fix_applied",
        details: "translated-from-goa-software-factory-manifest",
      },
    ],
  };

  if (ts.completed_at && pipelineStatus === "completed") {
    document.pipeline.completed_at = ts.completed_at;
  }

  return document;
}
