// Pure-helper tests for the spec 112 Phase 5 scaffold subflow.

import { describe, expect, test } from "vitest";
import { buildL0PipelineStateSeed } from "./seedPipelineState";
import { buildProjectOpenDeepLink } from "./deepLink";
import {
  PROFILE_MODULES,
  detectProfile,
  extrasFor,
  pickProfileFromModules,
  isKnownModule,
} from "./moduleCatalog";
import type { ScaffoldAdapterRef } from "./types";

// Spec 140 §2.2 — the legacy `templateRemote` / `templateDefaultBranch`
// fields no longer exist on `ScaffoldAdapterRef`; the clone target is
// resolved via `factory_upstreams` at warmup / create time.
const adapter: ScaffoldAdapterRef = {
  id: "00000000-0000-0000-0000-000000000000",
  name: "aim-vue-node",
  version: "3.0.0",
  sourceSha: "a".repeat(40),
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
    expect(url.protocol).toBe("opc:");
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

describe("detectProfile", () => {
  test("auth-saml → public", () => {
    expect(detectProfile(["auth-saml", "data-redis"])).toBe("public");
  });

  test("auth-entra-id → internal", () => {
    expect(detectProfile(["auth-entra-id", "data-postgres"])).toBe("internal");
  });

  test("no auth driver → minimal", () => {
    expect(detectProfile([])).toBe("minimal");
    expect(detectProfile(["data-redis"])).toBe("minimal");
  });

  test("auth-saml wins over auth-entra-id when both somehow selected", () => {
    expect(detectProfile(["auth-saml", "auth-entra-id"])).toBe("public");
  });
});

describe("pickProfileFromModules", () => {
  test("variant=dual always picks dual regardless of modules", () => {
    expect(pickProfileFromModules("dual", [])).toBe("dual");
    expect(pickProfileFromModules("dual", ["auth-saml"])).toBe("dual");
  });

  test("variant=single-public → public", () => {
    expect(pickProfileFromModules("single-public", [])).toBe("public");
  });

  test("variant=single-internal → internal", () => {
    expect(pickProfileFromModules("single-internal", [])).toBe("internal");
  });

  test("unknown variant falls through to detectProfile", () => {
    expect(pickProfileFromModules("unspecified", ["auth-saml"])).toBe("public");
    expect(pickProfileFromModules("unspecified", [])).toBe("minimal");
  });
});

describe("extrasFor", () => {
  test("dropping built-in modules: public profile already ships data-redis", () => {
    const result = extrasFor("public", ["data-redis", "user-management"]);
    // data-redis is in PROFILE_MODULES.public → dropped; user-management is not → kept
    expect(result).not.toContain("data-redis");
    expect(result).toContain("user-management");
  });

  test("sorting by INSTALL_ORDER: dependencies appear before dependents", () => {
    const result = extrasFor("minimal", [
      "user-management",
      "data-postgres",
      "auth-entra-id",
    ]);
    // INSTALL_ORDER places data-postgres → auth-entra-id → user-management
    expect(result).toEqual([
      "data-postgres",
      "auth-entra-id",
      "user-management",
    ]);
  });

  test("returns empty when every selected module is in the profile", () => {
    expect(extrasFor("public", PROFILE_MODULES.public)).toEqual([]);
  });

  test("unknown modules are dropped (filtered upstream by isKnownModule)", () => {
    const result = extrasFor("minimal", ["bogus-not-real"]);
    expect(result).toEqual([]);
  });
});

describe("spec 112 §10 runtime gate (shape)", () => {
  // The gate lives in create.ts; this test pins the shape of manifests we
  // expect to pass / fail so a translator change doesn't silently
  // re-introduce non-Node-24 adapters. Spec 140 §2.1 introduced the
  // top-level `scaffold_runtime` key; the gate accepts both the legacy
  // `scaffold.runtime` block (still emitted by some OAP-native manifests)
  // and the new top-level field.
  function evaluateRuntimeGate(manifest: Record<string, unknown>): "pass" | "reject" {
    const declared =
      (manifest as { scaffold?: { runtime?: string } }).scaffold?.runtime ??
      (typeof (manifest as { scaffold_runtime?: unknown }).scaffold_runtime ===
      "string"
        ? ((manifest as { scaffold_runtime: string }).scaffold_runtime)
        : undefined);
    if (declared && declared !== "node-24") return "reject";
    return "pass";
  }

  test("synthetic translator manifest carrying scaffold_runtime: node-24 passes", () => {
    expect(
      evaluateRuntimeGate({
        entry: "orchestration/template-orchestrator.md",
        scaffold_source_id: "aim-vue-node-template",
        scaffold_runtime: "node-24",
      })
    ).toBe("pass");
  });

  test("manifest without any runtime declaration passes (default)", () => {
    expect(
      evaluateRuntimeGate({
        entry: "orchestration/template-orchestrator.md",
      })
    ).toBe("pass");
  });

  test("explicitly declared node-24 (legacy scaffold block) passes", () => {
    expect(evaluateRuntimeGate({ scaffold: { runtime: "node-24" } })).toBe(
      "pass"
    );
  });

  test("non-node-24 top-level scaffold_runtime is rejected", () => {
    expect(evaluateRuntimeGate({ scaffold_runtime: "node-22" })).toBe("reject");
  });

  test("deno-2 / python / anything-else is rejected", () => {
    expect(evaluateRuntimeGate({ scaffold: { runtime: "deno-2" } })).toBe(
      "reject"
    );
    expect(evaluateRuntimeGate({ scaffold: { runtime: "python-3.12" } })).toBe(
      "reject"
    );
  });
});

describe("isKnownModule", () => {
  test("recognises catalogued modules", () => {
    expect(isKnownModule("auth-saml")).toBe(true);
    expect(isKnownModule("user-management")).toBe(true);
  });

  test("rejects unknown ids", () => {
    expect(isKnownModule("nope")).toBe(false);
    expect(isKnownModule("")).toBe(false);
  });

  test("rejects always-on modules that aren't user-selectable", () => {
    // security-core / auth-core are always-on per template-distributor:108-194.
    expect(isKnownModule("security-core")).toBe(false);
    expect(isKnownModule("auth-core")).toBe(false);
  });
});
