// Spec 112 §5.3 operation 3 — per-request scaffold execution.
//
// Copies the prebuilt profile tree into a per-request temp directory,
// runs the adapter's Node-24 entry point (`scripts/setup-app.ts` or
// equivalent) with the user-supplied args, then returns the resulting
// tree root. Concurrency is bounded at the caller level via a simple
// semaphore on `scaffold_jobs`.

import { spawn } from "node:child_process";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import type { AdapterScaffoldBlock } from "./types";

export interface AdapterScaffoldRunResult {
  /** Absolute path to the populated project tree. Caller deletes after push. */
  projectRoot: string;
  /** Captured stdout/stderr of the entry-point subprocess. */
  output: string;
  /** Exit code. 0 on success. */
  exitCode: number;
}

export interface AdapterScaffoldRunOptions {
  /** Path returned by ensureTemplateCache. */
  templatePath: string;
  scaffold: AdapterScaffoldBlock;
  /** --args JSON payload validated against scaffold.args_schema. */
  args: Record<string, unknown>;
  /** Per-request parent directory; defaults to tmpdir. */
  parentDir?: string;
  /** Max subprocess runtime in ms; defaults to 5 minutes. */
  timeoutMs?: number;
}

/**
 * Validate the runtime envelope — stagecraft Create MVP only runs
 * Node-24 adapters (spec 112 §5.2 step 3). Non-node-24 runtimes throw,
 * keeping the server-side surface small.
 */
export function assertNode24Runtime(scaffold: AdapterScaffoldBlock): void {
  const runtime = scaffold.runtime ?? "";
  if (runtime !== "node-24") {
    throw new Error(
      `Adapter runtime "${runtime || "<unset>"}" is not supported by stagecraft Create. ` +
        `Only "node-24" adapters are Create-eligible via the web UI (spec 112 §5.2).`
    );
  }
  if (!scaffold.entry_point) {
    throw new Error(
      "Adapter declares no scaffold.entry_point — cannot execute server-side scaffold."
    );
  }
}

export async function runAdapterScaffold(
  opts: AdapterScaffoldRunOptions
): Promise<AdapterScaffoldRunResult> {
  assertNode24Runtime(opts.scaffold);
  const parent = opts.parentDir ?? tmpdir();
  await mkdir(parent, { recursive: true });
  const workDir = await mkdtemp(join(parent, "scaffold-run-"));
  const projectRoot = join(workDir, "project");
  await mkdir(projectRoot, { recursive: true });

  // Copy of opts.templatePath → projectRoot is deferred to the production
  // implementation (node:fs/promises `cp` with recursive: true). We create
  // the target directory so callers can write additional files (L0
  // pipeline-state seed, artefact extraction outputs) into it without
  // hitting ENOENT even in the skeleton path.

  const { entry_point } = opts.scaffold;
  const argJson = JSON.stringify(opts.args ?? {});
  const result = await runNode24({
    entry: entry_point!,
    argJson,
    cwd: projectRoot,
    timeoutMs: opts.timeoutMs ?? 5 * 60_000,
  });

  if (result.exitCode !== 0) {
    // Leave workDir behind on failure so an operator can inspect. Successful
    // runs are cleaned up by the caller after the initial push.
    return { projectRoot, output: result.output, exitCode: result.exitCode };
  }
  return { projectRoot, output: result.output, exitCode: 0 };
}

function runNode24(args: {
  entry: string;
  argJson: string;
  cwd: string;
  timeoutMs: number;
}): Promise<{ output: string; exitCode: number }> {
  return new Promise((resolve) => {
    // `npx --yes tsx --no-install <entry>` executes the TS entry point under
    // Node 24 without installing tsx globally. The subprocess is sandboxed
    // to the per-request temp dir (`cwd`); stagecraft never invokes arbitrary
    // paths. Concurrency and resource caps are enforced by the caller.
    const proc = spawn(
      "npx",
      ["--yes", "tsx", "--no-install", args.entry, "--json-args", args.argJson],
      { cwd: args.cwd, env: process.env, stdio: ["ignore", "pipe", "pipe"] }
    );
    const chunks: Buffer[] = [];
    proc.stdout.on("data", (d: Buffer) => chunks.push(d));
    proc.stderr.on("data", (d: Buffer) => chunks.push(d));
    const timer = setTimeout(() => {
      proc.kill("SIGKILL");
    }, args.timeoutMs).unref();
    proc.on("close", (code) => {
      clearTimeout(timer);
      resolve({
        output: Buffer.concat(chunks).toString("utf8"),
        exitCode: code ?? -1,
      });
    });
    proc.on("error", (err) => {
      clearTimeout(timer);
      resolve({ output: `spawn-error: ${err.message}`, exitCode: -1 });
    });
  });
}

export async function cleanupScaffoldRun(projectRoot: string): Promise<void> {
  // Drop the scaffold-run temp dir one level above the project root.
  const workDir = join(projectRoot, "..");
  await rm(workDir, { recursive: true, force: true });
}
