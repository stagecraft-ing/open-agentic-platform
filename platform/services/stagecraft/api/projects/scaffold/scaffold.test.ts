// Pure-helper tests for the spec 112 Phase 5 scaffold subflow.

import { describe, expect, test } from "vitest";
import { buildL0PipelineStateSeed } from "./seedPipelineState";
import { buildProjectOpenDeepLink } from "./deepLink";
import {
  INSTALL_ORDER,
  MODULE_CATALOG,
  PRESETS,
  PROFILE_MODULES,
  PROFILES,
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
  name: "aim-vue-encore",
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
      name: "aim-vue-encore",
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

describe("moduleCatalog — template-encore cutover (spec 199 FR-007)", () => {
  test("the catalog is exactly template-encore's modules/ directory", () => {
    expect(MODULE_CATALOG.map((m) => m.id).sort()).toEqual([
      "api-gateway",
      "data-postgres",
      "data-redis",
      "security-core",
      "user-management",
    ]);
  });

  test("retired express-session-era and auth-module ids are gone", () => {
    for (const retired of [
      "auth-saml",
      "auth-entra-id",
      "session-store-redis",
      "session-store-postgres",
      "service-auth",
      "api-docs",
    ]) {
      expect(isKnownModule(retired)).toBe(false);
    }
  });

  test("no module ships by default: profile built-ins and presets are empty", () => {
    for (const profile of PROFILES) {
      expect(PROFILE_MODULES[profile]).toEqual([]);
      expect(PRESETS[profile]).toEqual([]);
    }
  });

  test("INSTALL_ORDER covers the catalog and puts security-core before api-gateway", () => {
    expect([...INSTALL_ORDER].sort()).toEqual(
      MODULE_CATALOG.map((m) => m.id).sort()
    );
    expect(INSTALL_ORDER.indexOf("security-core")).toBeLessThan(
      INSTALL_ORDER.indexOf("api-gateway")
    );
  });
});

describe("pickProfileFromModules", () => {
  test("variant=dual always picks dual regardless of modules", () => {
    expect(pickProfileFromModules("dual", [])).toBe("dual");
    expect(pickProfileFromModules("dual", ["api-gateway"])).toBe("dual");
  });

  test("variant=single-public → public", () => {
    expect(pickProfileFromModules("single-public", [])).toBe("public");
  });

  test("variant=single-internal → internal", () => {
    expect(pickProfileFromModules("single-internal", [])).toBe("internal");
  });

  test("unknown variant → minimal (auth is the AUTH_DRIVER axis, not a module)", () => {
    expect(pickProfileFromModules("unspecified", ["api-gateway"])).toBe("minimal");
    expect(pickProfileFromModules("unspecified", [])).toBe("minimal");
  });
});

describe("extrasFor", () => {
  test("every selected module is an extra (no profile built-ins exist)", () => {
    const result = extrasFor("public", ["data-redis", "user-management"]);
    expect(result).toEqual(["data-redis", "user-management"]);
  });

  test("sorting by INSTALL_ORDER: security-core installs before api-gateway", () => {
    const result = extrasFor("minimal", [
      "user-management",
      "api-gateway",
      "security-core",
    ]);
    expect(result).toEqual([
      "security-core",
      "api-gateway",
      "user-management",
    ]);
  });

  test("returns empty when nothing is selected", () => {
    expect(extrasFor("public", [])).toEqual([]);
  });

  test("unknown (incl. retired) modules are dropped", () => {
    expect(extrasFor("minimal", ["bogus-not-real", "session-store-redis"])).toEqual([]);
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
        scaffold_source_id: "legacy-template-source",
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
    // security-core is a regular opt-in module in template-encore's
    // catalog (the old always-on framing was template-distributor-era).
    expect(isKnownModule("security-core")).toBe(true);
    expect(isKnownModule("user-management")).toBe(true);
  });

  test("rejects unknown ids", () => {
    expect(isKnownModule("nope")).toBe(false);
    expect(isKnownModule("")).toBe(false);
    expect(isKnownModule("auth-core")).toBe(false);
  });
});
