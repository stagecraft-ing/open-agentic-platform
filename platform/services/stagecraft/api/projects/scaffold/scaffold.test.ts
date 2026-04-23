// Pure-helper tests for the spec 112 Phase 5 scaffold subflow.

import { describe, expect, test } from "vitest";
import { buildL0PipelineStateSeed } from "./seedPipelineState";
import { buildProjectOpenDeepLink } from "./deepLink";
import { declaredPrebuildProfiles, pickProfile } from "./prebuilds";
import { assertNode24Runtime } from "./runAdapterScaffold";
import type { AdapterScaffoldBlock, ScaffoldAdapterRef } from "./types";

const adapter: ScaffoldAdapterRef = {
  id: "00000000-0000-0000-0000-000000000000",
  name: "aim-vue-node",
  version: "3.0.0",
  sourceSha: "a".repeat(40),
  scaffold: {
    entry_point: "scripts/setup-app.ts",
    runtime: "node-24",
    profiles: [
      { name: "single-public", variant: "single-public" },
      { name: "single-internal", variant: "single-internal" },
      { name: "dual", variant: "dual", default: true, modules: ["security-core"] },
    ],
    emits: [{ path: ".factory/pipeline-state.json" }],
  },
};

describe("buildL0PipelineStateSeed", () => {
  test("produces a schema-1.0.0 seed with pending status and embedded adapter identity", () => {
    const seed = buildL0PipelineStateSeed(adapter);
    expect(seed.schema_version).toBe("1.0.0");
    expect(seed.pipeline.status).toBe("pending");
    expect(seed.pipeline.started_at).toBeNull();
    expect(seed.pipeline.adapter).toEqual({
      name: "aim-vue-node",
      version: "3.0.0",
      source_sha: "a".repeat(40),
    });
    expect(seed.pipeline.build_spec).toEqual({ path: null, hash: null });
    expect(seed.stages).toEqual({});
    expect(seed.pipeline.id).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
    );
  });

  test("emits a fresh pipeline id per invocation", () => {
    const a = buildL0PipelineStateSeed(adapter);
    const b = buildL0PipelineStateSeed(adapter);
    expect(a.pipeline.id).not.toBe(b.pipeline.id);
  });
});

describe("buildProjectOpenDeepLink", () => {
  test("encodes project_id, clone url, and detection level", () => {
    const link = buildProjectOpenDeepLink({
      projectId: "11111111-2222-3333-4444-555555555555",
      cloneUrl: "https://github.com/acme/example.git",
      detectionLevel: "scaffold_only",
    });
    const url = new URL(link);
    expect(url.protocol).toBe("oap:");
    expect(url.host).toBe("project");
    expect(url.pathname).toBe("/open");
    expect(url.searchParams.get("project_id")).toBe(
      "11111111-2222-3333-4444-555555555555"
    );
    expect(url.searchParams.get("url")).toBe(
      "https://github.com/acme/example.git"
    );
    expect(url.searchParams.get("level")).toBe("scaffold_only");
  });
});

describe("pickProfile", () => {
  test("explicit profileName must match variant", () => {
    expect(
      pickProfile(adapter.scaffold, { profileName: "dual", variant: "dual" })
    ).toMatchObject({ name: "dual", variant: "dual" });
    expect(
      pickProfile(adapter.scaffold, { profileName: "dual", variant: "single-public" })
    ).toBeNull();
    expect(
      pickProfile(adapter.scaffold, { profileName: "unknown", variant: "dual" })
    ).toBeNull();
  });

  test("without profileName, picks the default-for-variant entry", () => {
    expect(
      pickProfile(adapter.scaffold, { variant: "dual" })
    ).toMatchObject({ name: "dual" });
    expect(
      pickProfile(adapter.scaffold, { variant: "single-public" })
    ).toMatchObject({ name: "single-public" });
  });

  test("returns null when adapter declares no profiles", () => {
    expect(pickProfile({}, { variant: "dual" })).toBeNull();
  });
});

describe("declaredPrebuildProfiles", () => {
  test("enumerates name, variant, and isDefault for each profile", () => {
    const prebuilds = declaredPrebuildProfiles(adapter.scaffold);
    expect(prebuilds).toEqual([
      { name: "single-public", variant: "single-public", isDefault: false },
      { name: "single-internal", variant: "single-internal", isDefault: false },
      { name: "dual", variant: "dual", isDefault: true },
    ]);
  });
});

describe("assertNode24Runtime", () => {
  test("accepts node-24 adapters with an entry_point", () => {
    expect(() => assertNode24Runtime(adapter.scaffold)).not.toThrow();
  });

  test("rejects non-node-24 runtimes", () => {
    const scaffold: AdapterScaffoldBlock = { entry_point: "x", runtime: "deno-2" };
    expect(() => assertNode24Runtime(scaffold)).toThrow(/not supported/);
  });

  test("rejects missing entry_point", () => {
    const scaffold: AdapterScaffoldBlock = { runtime: "node-24" };
    expect(() => assertNode24Runtime(scaffold)).toThrow(/no scaffold\.entry_point/);
  });
});
