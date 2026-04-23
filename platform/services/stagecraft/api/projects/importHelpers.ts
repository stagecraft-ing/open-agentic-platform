// Spec 112 Phase 6 — pure helpers extracted from `import.ts` so they
// can be unit-tested without loading the Encore native runtime.

export const TRANSLATOR_VERSION = "spec-112-v1";

export function parseRepoUrlImpl(input: string): { owner: string; repo: string } {
  const trimmed = input.trim();
  const shortForm = /^([^\s/]+)\/([^\s/]+?)(?:\.git)?$/.exec(trimmed);
  if (shortForm) {
    return { owner: shortForm[1], repo: shortForm[2] };
  }
  const url = new URL(trimmed); // throws on invalid URL
  if (!/^(www\.)?github\.com$/i.test(url.host)) {
    throw new Error(
      `Expected github.com host, got "${url.host}". Only GitHub repos are importable.`
    );
  }
  const parts = url.pathname.replace(/^\//, "").replace(/\.git$/i, "").split("/");
  if (parts.length < 2 || !parts[0] || !parts[1]) {
    throw new Error(`Could not parse owner/repo from URL: ${trimmed}`);
  }
  return { owner: parts[0], repo: parts[1] };
}
