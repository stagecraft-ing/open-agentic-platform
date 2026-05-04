// Spec 112 §5.3 op 3 — per-request scaffold execution.
//
// Mirrors template-distributor/src/server.ts:613-760 minus the in-memory
// poll loop (we drive scaffold_jobs.step from the orchestrator instead).
//
// Per-request flow:
//   1. Copy `${WORKSPACE_DIR}/_prebuilt-<profile>` → destDir (excluding
//      node_modules — they re-resolve from the project's lockfile).
//   2. For each `extras` module (sorted by INSTALL_ORDER), run
//      `tsx _template-cache/scripts/add-module.ts <mod> --yes` with
//      `ROOT=destDir, NO_INSTALL=true`.
//   3. If any extras were applied, run `npm install --package-lock-only`
//      once to produce a coherent lockfile.
//   4. Write `.factory/pipeline-state.json` from the L0 seed so commit #1
//      carries it atomically.

import { spawn } from "node:child_process";
import { cp as cpAsync, mkdir, writeFile } from "node:fs/promises";
import { join, sep } from "node:path";
import log from "encore.dev/log";
import { extrasFor, type Profile } from "./moduleCatalog";
import { prebuiltDir } from "./templateCache";

/**
 * Subprocess env shared with templateCache. Mirrors `tooledEnv` —
 * routes npm + node tooling at writable workspace paths so the pod's
 * `readOnlyRootFilesystem: true` posture doesn't kill `npm install`.
 */
function tooledEnv(
  workspace: string,
  extra: NodeJS.ProcessEnv = {}
): NodeJS.ProcessEnv {
  return {
    ...process.env,
    HOME: join(workspace, ".home"),
    npm_config_cache: join(workspace, ".npm-cache"),
    XDG_CACHE_HOME: join(workspace, ".xdg-cache"),
    ...extra,
  };
}

export interface PerRequestScaffoldOptions {
  workspaceDir: string;
  profile: Profile;
  /** Modules selected by the user (raw — extrasFor filters them down). */
  selectedModules: string[];
  destDir: string;
  /** L0 pipeline-state seed object to drop at `.factory/pipeline-state.json`. */
  pipelineStateSeed: Record<string, unknown>;
  /** Optional progress sink, fed scaffold_jobs.step transitions. */
  log?: (line: string) => void;
}

export interface PerRequestScaffoldResult {
  destDir: string;
  profile: Profile;
  extras: string[];
}

export async function scaffoldFromPrebuilt(
  opts: PerRequestScaffoldOptions
): Promise<PerRequestScaffoldResult> {
  const cacheDir = join(opts.workspaceDir, "_template-cache");
  const sourceDir = prebuiltDir(opts.workspaceDir, opts.profile);
  const dest = opts.destDir;
  const sink = opts.log ?? (() => {});

  await mkdir(dest, { recursive: true });

  // ── 1. Copy prebuilt tree, excluding node_modules ────────────────────
  sink(`copy: ${sourceDir} → ${dest}`);
  await cpAsync(sourceDir, dest, {
    recursive: true,
    filter: (src: string) => !src.includes(`${sep}node_modules${sep}`),
  });

  // ── 2. Run add-module.ts for each user-selected extra ────────────────
  const extras = opts.profile === "dual" ? [] : extrasFor(opts.profile, opts.selectedModules);
  if (extras.length > 0) {
    const tsx = join(cacheDir, "node_modules", "tsx", "dist", "cli.mjs");
    const addModuleScript = join(cacheDir, "scripts", "add-module.ts");
    const addModuleEnv = tooledEnv(opts.workspaceDir, {
      NODE_PATH: join(cacheDir, "node_modules"),
      NO_INSTALL: "true",
      // add-module.ts reads ROOT to know where to write; it must be the
      // per-request dest, not the cache dir.
      ROOT: dest,
    });
    for (const mod of extras) {
      sink(`add-module: ${mod}`);
      await spawnAndCapture(
        process.execPath,
        [tsx, addModuleScript, mod, "--yes"],
        cacheDir,
        addModuleEnv
      );
    }
    // ── 3. Refresh the lockfile once for all extras ─────────────────────
    sink("npm install --package-lock-only");
    await spawnAndCapture(
      "npm",
      ["install", "--package-lock-only"],
      dest,
      tooledEnv(opts.workspaceDir)
    );
  }

  // ── 4. Drop the L0 pipeline-state seed ───────────────────────────────
  const factoryDir = join(dest, ".factory");
  await mkdir(factoryDir, { recursive: true });
  await writeFile(
    join(factoryDir, "pipeline-state.json"),
    JSON.stringify(opts.pipelineStateSeed, null, 2) + "\n",
    "utf8"
  );
  sink("seed: .factory/pipeline-state.json");

  log.info("per-request scaffold: complete", {
    profile: opts.profile,
    dest,
    extras,
  });

  return { destDir: dest, profile: opts.profile, extras };
}

function spawnAndCapture(
  bin: string,
  args: string[],
  cwd: string,
  env: NodeJS.ProcessEnv | undefined
): Promise<void> {
  return new Promise((resolveRun, rejectRun) => {
    const proc = spawn(bin, args, {
      cwd,
      env: env ?? process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    const chunks: Buffer[] = [];
    proc.stdout.on("data", (d: Buffer) => chunks.push(d));
    proc.stderr.on("data", (d: Buffer) => chunks.push(d));
    proc.on("close", (code) => {
      if (code === 0) {
        resolveRun();
      } else {
        const tail = Buffer.concat(chunks).toString("utf8").slice(-2000);
        rejectRun(
          new Error(`${bin} ${args.join(" ")} exited ${code}: ${tail}`)
        );
      }
    });
    proc.on("error", rejectRun);
  });
}
