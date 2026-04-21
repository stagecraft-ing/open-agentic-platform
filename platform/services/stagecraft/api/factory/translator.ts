import { readdir, readFile, stat } from "node:fs/promises";
import { join, relative, basename } from "node:path";

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
const FACTORY_SOURCE_EXCLUDES: Array<(rel: string) => boolean> = [
  (p) => p === ".git" || p.startsWith(".git/"),
  (p) => p === ".github" || p.startsWith(".github/"),
  (p) => p === "README.md" || p === ".project",
  (p) => p.startsWith("eval_framework/"),
  (p) => p.startsWith("REDTEAM/"),
  (p) => p.startsWith("Security Agent/"),
  (p) => p.startsWith("Factory Agent/Security/"),
  (p) => p.startsWith("Factory Agent/Orchestrator/scripts/"),
  (p) => p === "Factory Agent/Orchestrator/factory-orchestration-cd.md",
  (p) => p.startsWith("Factory Agent/Requirements/Client/"),
  (p) => p === "Factory Agent/Controllers/api-web-standards.md",
  (p) => p === "Factory Agent/Controllers/api-standards-compliance.md",
  (p) => /^Factory Agent\/Requirements\/Service\/sitemap-template-.*\.json$/.test(p),
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
  const m = /^factory-orchestration-(s\d+|tm|xf)\.md$/.exec(name);
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
  const requirements: CapturedFile[] = [];
  const database: CapturedFile[] = [];
  const otherAgents: CapturedFile[] = [];
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

    if (/^Factory Agent\/Requirements\/.+\.md$/.test(rel)) {
      requirements.push({ path: rel, body: await readText(abs) });
      continue;
    }

    if (/^Factory Agent\/Database\/.+\.md$/.test(rel)) {
      database.push({ path: rel, body: await readText(abs) });
      continue;
    }
  }

  stages.sort((a, b) => a.id.localeCompare(b.id));
  const sortByPath = (a: CapturedFile, b: CapturedFile) =>
    a.path.localeCompare(b.path);
  controllers.sort(sortByPath);
  clientInterface.sort(sortByPath);
  requirements.sort(sortByPath);
  database.sort(sortByPath);
  otherAgents.sort(sortByPath);
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
        requirements,
        database,
        other: otherAgents,
      },
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
