// Spec 112 §5.3 operation 5 — initial git commit + authed push to main.
//
// Takes a populated project tree on disk (the scaffold run's output,
// with `.factory/pipeline-state.json` already seeded) and pushes it as
// the repo's first commit using the installation token.
//
// The concrete git operations (`git init`, `git add`, `git commit`,
// `git push`) are delegated to a shell runner so stagecraft does not
// carry a git-rs dependency. The shell runner captures the resulting
// commit SHA for the scaffold_jobs row.

import { spawn } from "node:child_process";

export interface GithubPushOptions {
  projectRoot: string;
  /** Authed clone URL — https://x-access-token:<token>@github.com/<org>/<repo>.git */
  authedRemoteUrl: string;
  /** Default branch (typically "main"). */
  branch: string;
  commitMessage: string;
  authorName: string;
  authorEmail: string;
  coAuthor?: { name: string; email: string };
}

export interface GithubPushResult {
  commitSha: string;
  branch: string;
}

export function authedGitUrl(
  githubOrg: string,
  repoName: string,
  installationToken: string
): string {
  return `https://x-access-token:${installationToken}@github.com/${githubOrg}/${repoName}.git`;
}

export async function pushInitialCommit(
  opts: GithubPushOptions
): Promise<GithubPushResult> {
  const commitBody = opts.coAuthor
    ? `${opts.commitMessage}\n\nCo-authored-by: ${opts.coAuthor.name} <${opts.coAuthor.email}>`
    : opts.commitMessage;

  const env = {
    ...process.env,
    GIT_AUTHOR_NAME: opts.authorName,
    GIT_AUTHOR_EMAIL: opts.authorEmail,
    GIT_COMMITTER_NAME: opts.authorName,
    GIT_COMMITTER_EMAIL: opts.authorEmail,
  };

  await run("git", ["init", "-b", opts.branch], opts.projectRoot, env);
  await run("git", ["add", "-A"], opts.projectRoot, env);
  await run(
    "git",
    ["commit", "-m", commitBody],
    opts.projectRoot,
    env
  );
  await run(
    "git",
    ["remote", "add", "origin", opts.authedRemoteUrl],
    opts.projectRoot,
    env
  );
  await run(
    "git",
    ["push", "--force-with-lease", "origin", opts.branch],
    opts.projectRoot,
    env
  );
  const sha = await capture(
    "git",
    ["rev-parse", "HEAD"],
    opts.projectRoot,
    env
  );
  return { commitSha: sha.trim(), branch: opts.branch };
}

function run(
  bin: string,
  args: string[],
  cwd: string,
  env: NodeJS.ProcessEnv
): Promise<void> {
  return new Promise((resolve, reject) => {
    const proc = spawn(bin, args, { cwd, env, stdio: ["ignore", "pipe", "pipe"] });
    const err: Buffer[] = [];
    proc.stderr.on("data", (d: Buffer) => err.push(d));
    proc.on("close", (code) => {
      if (code === 0) resolve();
      else
        reject(
          new Error(
            `${bin} ${args.join(" ")} failed (exit ${code}): ${Buffer.concat(err).toString("utf8")}`
          )
        );
    });
    proc.on("error", reject);
  });
}

function capture(
  bin: string,
  args: string[],
  cwd: string,
  env: NodeJS.ProcessEnv
): Promise<string> {
  return new Promise((resolve, reject) => {
    const proc = spawn(bin, args, { cwd, env, stdio: ["ignore", "pipe", "pipe"] });
    const out: Buffer[] = [];
    proc.stdout.on("data", (d: Buffer) => out.push(d));
    proc.on("close", (code) => {
      if (code === 0) resolve(Buffer.concat(out).toString("utf8"));
      else reject(new Error(`${bin} ${args.join(" ")} failed (exit ${code})`));
    });
    proc.on("error", reject);
  });
}
