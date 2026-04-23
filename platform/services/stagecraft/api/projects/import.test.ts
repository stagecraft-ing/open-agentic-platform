// Spec 112 Phase 6 — unit tests for import-path pure helpers.
//
// End-to-end cloning + DB insertion require live GitHub + Postgres and
// are covered by integration testing. This file pins the pure helpers
// extracted into `importHelpers.ts` so they can be exercised without
// loading the Encore native runtime.

import { describe, expect, test } from "vitest";
import { parseRepoUrlImpl, TRANSLATOR_VERSION } from "./importHelpers";

describe("parseRepoUrlImpl", () => {
  test("accepts short form owner/repo", () => {
    expect(parseRepoUrlImpl("acme/foo")).toEqual({ owner: "acme", repo: "foo" });
  });

  test("accepts short form with .git suffix", () => {
    expect(parseRepoUrlImpl("acme/foo.git")).toEqual({ owner: "acme", repo: "foo" });
  });

  test("accepts full github URL", () => {
    expect(parseRepoUrlImpl("https://github.com/acme/foo")).toEqual({
      owner: "acme",
      repo: "foo",
    });
  });

  test("accepts github URL with .git suffix", () => {
    expect(parseRepoUrlImpl("https://github.com/acme/foo.git")).toEqual({
      owner: "acme",
      repo: "foo",
    });
  });

  test("accepts www.github.com host", () => {
    expect(parseRepoUrlImpl("https://www.github.com/acme/foo")).toEqual({
      owner: "acme",
      repo: "foo",
    });
  });

  test("rejects non-github hosts", () => {
    expect(() => parseRepoUrlImpl("https://gitlab.com/acme/foo")).toThrow(
      /Expected github\.com/
    );
  });

  test("rejects URLs without repo component", () => {
    expect(() => parseRepoUrlImpl("https://github.com/acme")).toThrow(
      /parse owner\/repo/
    );
  });
});

describe("TRANSLATOR_VERSION", () => {
  test("is a stable non-empty identifier", () => {
    expect(TRANSLATOR_VERSION).toBe("spec-112-v1");
  });
});
