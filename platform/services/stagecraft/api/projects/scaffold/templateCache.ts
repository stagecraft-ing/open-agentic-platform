// Spec 112 §5.3 ops 1+2 — template cache + four profile prebuilds.
//
// Mirrors template-distributor/src/server.ts:329-446. On startup
// stagecraft (a) clones the upstream `template` repo into
// `${WORKSPACE_DIR}/_template-cache` and runs `npm install`, then
// (b) runs `tsx scripts/setup-{app,dual-app}.ts` four times to materialise
// `_prebuilt-{minimal,public,internal,dual}`. Both steps are idempotent on
// disk: SHA tracking via `.template-commit` and `.prebuilt-commit` lets a
// restarted pod reuse the existing cache without re-doing work.
//
// A 30-min background refresher polls the upstream branch head for new
// commits; when it advances, the cache + prebuilts are rebuilt under the
// temp-dir-then-rename pattern so in-flight create requests are not
// disturbed.
//
// The Create endpoint reads `getInitStatus()` to decide whether to accept
// the request; the readiness UI (Phase 5) renders the same status.

import { spawn } from "node:child_process";
import { access, mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import log from "encore.dev/log";
import { PROFILES, type Profile } from "./moduleCatalog";

// ── Types ──────────────────────────────────────────────────────────────

export type InitStep =
  | "idle"
  | "cloning"
  | "cache-installing"
  | "building-minimal"
  | "building-public"
  | "building-internal"
  | "building-dual"
  | "ready"
  | "error";

export interface InitStatus {
  step: InitStep;
  progress: number;
  ready: boolean;
  error?: string;
}

export interface WarmupContext {
  /** Absolute path of the workspace dir; cache + prebuilts live underneath. */
  workspaceDir: string;
  /** GitHub `<owner>/<repo>` for the upstream template. */
  templateRemote: string;
  /** Branch the cache pins to; the refresher polls this. */
  defaultBranch: string;
  /**
   * Async resolver that returns the plaintext PAT used to clone the template
   * (and to read the branch head via REST). The brief mandates a single PAT
   * per org — the same `factory_upstream_pats` row we use for /factory-sync.
   */
  patResolver: () => Promise<string | null>;
}

// ── Module-scoped state ────────────────────────────────────────────────

let initStatus: InitStatus = { step: "idle", progress: 0, ready: false };
let templateCacheReady = false;
let templateCacheRefreshing = false;
let backgroundRefresherStarted = false;

export function getInitStatus(): InitStatus {
  return { ...initStatus };
}

export function isTemplateCacheReady(): boolean {
  return templateCacheReady;
}

export function isTemplateCacheRefreshing(): boolean {
  return templateCacheRefreshing;
}

/**
 * Surface a warmup-blocked condition (e.g. no adapter manifest carries
 * `template_remote`) through the existing readiness path. The status
 * step stays "error" so the UI's `warmup-error` blocker fires with a
 * clear, actionable reason.
 */
export function setInitErrorFromContext(reason: string): void {
  initStatus = { step: "error", progress: 0, ready: false, error: reason };
  templateCacheReady = false;
}

/**
 * Test-only: reset the in-memory status flags. Production code never calls
 * this — pods restart cleanly because warmup re-derives from disk state.
 */
export function _resetForTests(): void {
  initStatus = { step: "idle", progress: 0, ready: false };
  templateCacheReady = false;
  templateCacheRefreshing = false;
  backgroundRefresherStarted = false;
}

// ── Path helpers ───────────────────────────────────────────────────────

export function defaultWorkspaceDir(): string {
  return resolve(process.env.STAGECRAFT_WORKSPACE_DIR ?? "./workspace");
}

function cacheDir(workspace: string): string {
  return join(workspace, "_template-cache");
}

function templateCommitFile(workspace: string): string {
  return join(workspace, ".template-commit");
}

function prebuiltCommitFile(workspace: string): string {
  return join(workspace, ".prebuilt-commit");
}

export function prebuiltDir(workspace: string, profile: Profile): string {
  return join(workspace, `_prebuilt-${profile}`);
}

/**
 * Build a subprocess env that routes npm + node tooling at writable
 * paths. The pod has `readOnlyRootFilesystem: true` and runs as uid
 * 10001 with no $HOME, so npm's defaults (`$HOME/.npm`) resolve to
 * `/.npm` which the kernel rejects with EROFS / ENOENT. Setting
 * `npm_config_cache` and `HOME` under the workspace PVC makes npm,
 * tsx, and any subprocess they spawn write to a backed-up location.
 */
function tooledEnv(
  workspace: string,
  extra: NodeJS.ProcessEnv = {}
): NodeJS.ProcessEnv {
  const npmCache = join(workspace, ".npm-cache");
  const homeOverride = join(workspace, ".home");
  return {
    ...process.env,
    HOME: homeOverride,
    npm_config_cache: npmCache,
    // Some tools (corepack, pnpm shim) read XDG_CACHE_HOME independently.
    XDG_CACHE_HOME: join(workspace, ".xdg-cache"),
    ...extra,
  };
}

async function ensureToolingDirs(workspace: string): Promise<void> {
  const dirs = [
    join(workspace, ".npm-cache"),
    join(workspace, ".home"),
    join(workspace, ".xdg-cache"),
  ];
  for (const d of dirs) {
    await mkdir(d, { recursive: true });
  }
}

// ── Public surface ─────────────────────────────────────────────────────

/**
 * Clone the template repo (or refresh in place if upstream advanced) and
 * `npm install`. Idempotent on disk SHA. Throws on any non-recoverable
 * failure; callers translate that into a job-level error.
 */
export async function ensureTemplateCache(ctx: WarmupContext): Promise<void> {
  if (templateCacheRefreshing) {
    while (templateCacheRefreshing) {
      await sleep(500);
    }
    return;
  }

  const cache = cacheDir(ctx.workspaceDir);
  const cachedSha = await readShaFile(templateCommitFile(ctx.workspaceDir));
  const latestSha = await fetchLatestTemplateCommit(ctx);
  const diskCacheValid =
    (await pathExists(cache)) && !!latestSha && cachedSha === latestSha;

  if (diskCacheValid) {
    templateCacheReady = true;
    log.info("template cache: already up to date", {
      remote: ctx.templateRemote,
      sha: cachedSha,
    });
    return;
  }

  templateCacheRefreshing = true;
  templateCacheReady = false;
  initStatus = { step: "cloning", progress: 5, ready: false };

  const tempDir = cache + "_new";
  try {
    await mkdir(ctx.workspaceDir, { recursive: true });
    await ensureToolingDirs(ctx.workspaceDir);
    await rm(tempDir, { recursive: true, force: true }).catch(() => {});

    const token = await ctx.patResolver();
    const cloneUrl = buildCloneUrl(ctx.templateRemote, token);
    const env = tooledEnv(ctx.workspaceDir);

    log.info("template cache: cloning", {
      remote: ctx.templateRemote,
      branch: ctx.defaultBranch,
    });
    await spawnLogged(
      "git",
      ["clone", "--branch", ctx.defaultBranch, "--depth", "1", cloneUrl, tempDir],
      ctx.workspaceDir,
      env,
      token ?? undefined
    );

    initStatus = { step: "cache-installing", progress: 15, ready: false };
    log.info("template cache: npm install");
    await spawnLogged("npm", ["install"], tempDir, env, undefined);

    if (await pathExists(cache)) {
      await rm(cache, { recursive: true, force: true });
    }
    await rename(tempDir, cache);

    if (latestSha) {
      await writeFile(templateCommitFile(ctx.workspaceDir), latestSha, "utf8");
    }
    templateCacheReady = true;
    log.info("template cache: ready", {
      remote: ctx.templateRemote,
      sha: latestSha,
    });
  } catch (err) {
    initStatus = {
      step: "error",
      progress: 0,
      ready: false,
      error: errMsg(err),
    };
    await rm(tempDir, { recursive: true, force: true }).catch(() => {});
    throw err;
  } finally {
    templateCacheRefreshing = false;
  }
}

/**
 * Materialise the four `_prebuilt-{profile}` trees by running `tsx
 * scripts/setup-{app,dual-app}.ts` against the template cache. Skipped
 * cleanly when `.prebuilt-commit` already matches `.template-commit`.
 */
export async function ensurePrebuilts(ctx: WarmupContext): Promise<void> {
  if (initStatus.ready) return;

  const cache = cacheDir(ctx.workspaceDir);
  const templateCommit = await readShaFile(templateCommitFile(ctx.workspaceDir));
  const prebuiltCommit = await readShaFile(prebuiltCommitFile(ctx.workspaceDir));

  const allExist = await Promise.all(
    PROFILES.map((p) => pathExists(prebuiltDir(ctx.workspaceDir, p)))
  );
  if (
    allExist.every(Boolean) &&
    !!templateCommit &&
    prebuiltCommit === templateCommit
  ) {
    initStatus = { step: "ready", progress: 100, ready: true };
    log.info("prebuilts: already up to date", { sha: templateCommit });
    return;
  }

  const tsx = join(cache, "node_modules", "tsx", "dist", "cli.mjs");
  await ensureToolingDirs(ctx.workspaceDir);
  const prebuiltEnv = tooledEnv(ctx.workspaceDir, {
    NODE_PATH: join(cache, "node_modules"),
    NO_INSTALL: "true",
  });

  type ProfileSpec = { name: Profile; script: string; args: string[] };
  const PROFILE_SPECS: ProfileSpec[] = [
    { name: "minimal", script: "setup-app.ts", args: ["--profile", "minimal"] },
    { name: "public", script: "setup-app.ts", args: ["--profile", "public"] },
    {
      name: "internal",
      script: "setup-app.ts",
      args: ["--profile", "internal"],
    },
    { name: "dual", script: "setup-dual-app.ts", args: [] },
  ];

  for (const [i, spec] of PROFILE_SPECS.entries()) {
    const dest = prebuiltDir(ctx.workspaceDir, spec.name);
    initStatus = {
      step: `building-${spec.name}` as InitStep,
      progress: 20 + i * 20,
      ready: false,
    };
    log.info("prebuilt: building", { profile: spec.name, dest });
    if (await pathExists(dest)) {
      await rm(dest, { recursive: true, force: true });
    }

    try {
      await spawnLogged(
        process.execPath,
        [tsx, `scripts/${spec.script}`, ...spec.args, "--dest", dest, "--yes"],
        cache,
        prebuiltEnv,
        undefined
      );
    } catch (err) {
      initStatus = {
        step: "error",
        progress: 20 + i * 20,
        ready: false,
        error: `${spec.name} build failed — ${errMsg(err)}`,
      };
      throw err;
    }
  }

  if (templateCommit) {
    await writeFile(prebuiltCommitFile(ctx.workspaceDir), templateCommit, "utf8");
  }
  initStatus = { step: "ready", progress: 100, ready: true };
  log.info("prebuilts: all ready", { sha: templateCommit });
}

/**
 * One-shot warmup helper: cache → prebuilts. Called by Phase 4's startup
 * hook. Does not throw; failures land in `initStatus.error` so Create can
 * surface them via the readiness endpoint.
 */
export async function runWarmup(ctx: WarmupContext): Promise<void> {
  try {
    await ensureTemplateCache(ctx);
    await ensurePrebuilts(ctx);
  } catch (err) {
    log.warn("scaffold warmup failed", { error: errMsg(err) });
  }
}

/**
 * Start the 30-min refresher (idempotent — second call is a no-op).
 * Wakes up, checks upstream HEAD, rebuilds if it advanced.
 */
export function startBackgroundRefresher(ctx: WarmupContext): void {
  if (backgroundRefresherStarted) return;
  backgroundRefresherStarted = true;
  const interval = setInterval(() => {
    runWarmup(ctx).catch((err) => {
      log.warn("background refresher cycle failed", { error: errMsg(err) });
    });
  }, 30 * 60_000);
  // unref so the timer doesn't keep the process alive in tests / shutdown.
  if (typeof interval.unref === "function") interval.unref();
}

// ── Internal helpers ───────────────────────────────────────────────────

async function fetchLatestTemplateCommit(
  ctx: WarmupContext
): Promise<string | null> {
  const [owner, repo] = ctx.templateRemote.split("/");
  if (!owner || !repo) return null;
  const token = await ctx.patResolver().catch(() => null);

  const headers: Record<string, string> = {
    Accept: "application/vnd.github+json",
    "X-GitHub-Api-Version": "2022-11-28",
  };
  if (token) headers.Authorization = `Bearer ${token}`;

  try {
    const resp = await fetch(
      `https://api.github.com/repos/${owner}/${repo}/branches/${encodeURIComponent(ctx.defaultBranch)}`,
      { headers }
    );
    if (!resp.ok) {
      log.warn("template head lookup failed", {
        remote: ctx.templateRemote,
        status: resp.status,
      });
      return null;
    }
    const data = (await resp.json()) as { commit?: { sha?: string } };
    return data.commit?.sha ?? null;
  } catch (err) {
    log.warn("template head lookup threw", {
      remote: ctx.templateRemote,
      error: errMsg(err),
    });
    return null;
  }
}

function buildCloneUrl(remote: string, token: string | null): string {
  if (token) {
    return `https://x-access-token:${token}@github.com/${remote}.git`;
  }
  return `https://github.com/${remote}.git`;
}

function spawnLogged(
  bin: string,
  args: string[],
  cwd: string,
  env: NodeJS.ProcessEnv | undefined,
  redactToken: string | undefined
): Promise<void> {
  return new Promise((resolveRun, rejectRun) => {
    const proc = spawn(bin, args, {
      cwd,
      env: env ?? process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    const tail: string[] = [];
    const pushTail = (line: string) => {
      const safe = redactToken ? line.replaceAll(redactToken, "***") : line;
      tail.push(safe);
      while (tail.length > 40) tail.shift();
    };
    let buf = "";
    const onData = (d: Buffer) => {
      buf += d.toString();
      let nl: number;
      while ((nl = buf.indexOf("\n")) !== -1) {
        const line = buf.slice(0, nl).trim();
        if (line) pushTail(line);
        buf = buf.slice(nl + 1);
      }
    };
    proc.stdout.on("data", onData);
    proc.stderr.on("data", onData);
    proc.on("close", (code) => {
      if (buf.trim()) pushTail(buf.trim());
      if (code === 0) {
        resolveRun();
      } else {
        const detail = tail.slice(-10).join(" | ");
        rejectRun(
          new Error(
            `${bin} ${args.join(" ")} exited ${code}${detail ? `: ${detail}` : ""}`
          )
        );
      }
    });
    proc.on("error", rejectRun);
  });
}

async function pathExists(p: string): Promise<boolean> {
  try {
    await access(p);
    return true;
  } catch {
    return false;
  }
}

async function readShaFile(p: string): Promise<string | null> {
  try {
    return (await readFile(p, "utf8")).trim();
  } catch {
    return null;
  }
}

function errMsg(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}
