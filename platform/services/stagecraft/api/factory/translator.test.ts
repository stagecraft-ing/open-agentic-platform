import { describe, expect, test, beforeAll, afterAll } from "vitest";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  translateFactorySource,
  translateLegacyManifest,
  translateTemplate,
  translateUpstreams,
  type GoaSoftwareFactoryManifest,
  type GoaWorkingState,
  type FactoryAdapterRow,
} from "./translator";

async function writeTree(
  root: string,
  files: Record<string, string>
): Promise<void> {
  for (const [rel, body] of Object.entries(files)) {
    const abs = join(root, rel);
    const parent = abs.substring(0, abs.lastIndexOf("/"));
    await mkdir(parent, { recursive: true });
    await writeFile(abs, body, "utf8");
  }
}

describe("translateFactorySource", () => {
  let repo: string;
  const sha = "abcdef0123456789abcdef0123456789abcdef01";

  beforeAll(async () => {
    repo = await mkdtemp(join(tmpdir(), "translator-factory-"));
    await writeTree(repo, {
      "README.md": "ignored",
      "Factory Agent/factory-orchestration.md": "root orchestrator body",
      "Factory Agent/Orchestrator/factory-orchestration-s1.md": "s1 body",
      "Factory Agent/Orchestrator/factory-orchestration-s2.md": "s2 body",
      "Factory Agent/Orchestrator/factory-orchestration-s3.md": "s3 body",
      "Factory Agent/Orchestrator/factory-orchestration-s4.md": "s4 body",
      "Factory Agent/Orchestrator/factory-orchestration-s5.md": "s5 body",
      "Factory Agent/Orchestrator/factory-orchestration-tm.md": "tm body",
      "Factory Agent/Orchestrator/factory-orchestration-xf.md": "xf body",
      "Factory Agent/Orchestrator/factory-orchestration-cd.md": "excluded client doc",
      "Factory Agent/Orchestrator/scripts/docx-generator.py": "excluded script",
      "Factory Agent/Controllers/api-builder.md": "builder body",
      "Factory Agent/Controllers/api-web-standards.md": "excluded standards",
      "Factory Agent/Controllers/api-standards-compliance.md": "excluded compliance",
      "Factory Agent/Requirements/Service/audience-identification.md": "audience body",
      "Factory Agent/Requirements/Service/sitemap-template-v1.json": "excluded sitemap template",
      "Factory Agent/Requirements/Client/ministry-x.md": "excluded client content",
      "Factory Agent/Database/database_architect.md": "db architect body",
      "Factory Agent/Client_Interface/design-system.md": "design body",
      "Security Agent/orchestration.md": "excluded security",
      "Factory Agent/Security/security-bridge.md": "excluded factory security",
      "eval_framework/harness.py": "excluded eval",
      ".github/workflows/ci.yml": "excluded workflow",
      "Factory Agent/contracts/build-spec.schema.json":
        '{"$schema":"http://json-schema.org/draft-07/schema#","type":"object"}',
    });
  });

  afterAll(async () => {
    await rm(repo, { recursive: true, force: true });
  });

  test("captures root orchestrator + all 7 stage files sorted by id", async () => {
    const { process } = await translateFactorySource(repo, sha);
    expect(process.name).toBe("7-stage-build");
    expect(process.sourceSha).toBe(sha);

    const def = process.definition as {
      orchestrator: { body: string };
      stages: Array<{ id: string }>;
    };
    expect(def.orchestrator.body).toContain("root orchestrator body");

    const ids = def.stages.map((s) => s.id);
    expect(ids).toEqual(["s1", "s2", "s3", "s4", "s5", "tm", "xf"]);
  });

  test("applies spec 088 §5 exclusions for client docs, security, evals", async () => {
    const { process } = await translateFactorySource(repo, sha);
    const def = process.definition as {
      stages: Array<{ path: string; body: string }>;
      agents: {
        controllers: Array<{ path: string }>;
        requirements: Array<{ path: string }>;
      };
    };

    const bodies = JSON.stringify(def);
    expect(bodies).not.toContain("excluded client doc");
    expect(bodies).not.toContain("excluded standards");
    expect(bodies).not.toContain("excluded compliance");
    expect(bodies).not.toContain("excluded security");
    expect(bodies).not.toContain("excluded factory security");
    expect(bodies).not.toContain("excluded eval");
    expect(bodies).not.toContain("excluded client content");
    expect(bodies).not.toContain("excluded sitemap template");
    expect(bodies).not.toContain("excluded workflow");
    expect(bodies).not.toContain("excluded script");

    const ctrlPaths = def.agents.controllers.map((c) => c.path);
    expect(ctrlPaths).toContain("Factory Agent/Controllers/api-builder.md");
    expect(ctrlPaths).not.toContain(
      "Factory Agent/Controllers/api-web-standards.md"
    );
  });

  test("emits contract rows for discovered *.schema.{json,yaml}", async () => {
    const { contracts } = await translateFactorySource(repo, sha);
    expect(contracts).toHaveLength(1);
    expect(contracts[0].name).toBe("build-spec");
    expect(contracts[0].sourceSha).toBe(sha);
  });
});

describe("translateTemplate", () => {
  let repo: string;
  const sha = "1111111111111111111111111111111111111111";

  beforeAll(async () => {
    repo = await mkdtemp(join(tmpdir(), "translator-template-"));
    await writeTree(repo, {
      "README.md": "excluded readme",
      "package.json": "excluded pkg",
      "package-lock.json": "excluded lock",
      "docker-compose.yml": "excluded compose",
      "apps/web/index.tsx": "excluded app",
      "modules/auth.ts": "excluded module",
      "orchestration/template-orchestrator.md": "orchestrator body",
      "orchestration/skills/analyze.md": "analyze body",
      "orchestration/skills/configure.md": "configure body",
      "orchestration/skills/scaffold-feature.md": "scaffold body",
      "orchestration/skills/trim.md": "trim body",
      "orchestration/skills/validate.md": "validate body",
      ".claude/orchestration/internal.md": "excluded internal",
    });
  });

  afterAll(async () => {
    await rm(repo, { recursive: true, force: true });
  });

  test("produces aim-vue-node adapter with skill keys and orchestrator", async () => {
    const { adapter } = await translateTemplate(repo, sha);
    expect(adapter.name).toBe("aim-vue-node");
    expect(adapter.sourceSha).toBe(sha);

    const m = adapter.manifest as {
      entry: string;
      orchestrator: { body: string } | null;
      skills: Record<string, { path: string; body: string }>;
    };
    expect(m.entry).toBe("orchestration/template-orchestrator.md");
    expect(m.orchestrator?.body).toContain("orchestrator body");

    expect(Object.keys(m.skills).sort()).toEqual([
      "analyze",
      "configure",
      "scaffold-feature",
      "trim",
      "validate",
    ]);
    expect(m.skills.analyze.body).toBe("analyze body");
  });

  test("excludes application source, node modules, and docs", async () => {
    const { adapter } = await translateTemplate(repo, sha);
    const blob = JSON.stringify(adapter);
    expect(blob).not.toContain("excluded readme");
    expect(blob).not.toContain("excluded pkg");
    expect(blob).not.toContain("excluded lock");
    expect(blob).not.toContain("excluded compose");
    expect(blob).not.toContain("excluded app");
    expect(blob).not.toContain("excluded module");
    expect(blob).not.toContain("excluded internal");
  });
});

describe("translateUpstreams", () => {
  let factory: string;
  let template: string;

  beforeAll(async () => {
    factory = await mkdtemp(join(tmpdir(), "translator-combined-factory-"));
    template = await mkdtemp(join(tmpdir(), "translator-combined-template-"));

    await writeTree(factory, {
      "Factory Agent/factory-orchestration.md": "root",
      "Factory Agent/Orchestrator/factory-orchestration-s1.md": "s1",
      "Factory Agent/contracts/build-spec.schema.json":
        '{"type":"object","title":"build-spec"}',
    });
    await writeTree(template, {
      "orchestration/template-orchestrator.md": "orchestrator",
      "orchestration/skills/analyze.md": "analyze",
      "orchestration/contracts/build-spec.schema.json":
        '{"type":"object","title":"build-spec-dup"}',
      "orchestration/contracts/pipeline-state.schema.yaml":
        "type: object\ntitle: pipeline-state",
    });
  });

  afterAll(async () => {
    await rm(factory, { recursive: true, force: true });
    await rm(template, { recursive: true, force: true });
  });

  test("combines both repos and de-dupes contracts by name", async () => {
    const result = await translateUpstreams({
      factorySourcePath: factory,
      factorySourceSha: "f".repeat(40),
      templatePath: template,
      templateSha: "e".repeat(40),
    });

    expect(result.adapters.map((a) => a.name)).toEqual(["aim-vue-node"]);
    expect(result.processes.map((p) => p.name)).toEqual(["7-stage-build"]);

    const contractNames = result.contracts.map((c) => c.name).sort();
    expect(contractNames).toEqual(["build-spec", "pipeline-state"]);

    // build-spec should come from the factory source since it wins the dedupe
    const buildSpec = result.contracts.find((c) => c.name === "build-spec")!;
    expect(buildSpec.sourceSha).toBe("f".repeat(40));
  });

  test("throws when a source path does not exist", async () => {
    await expect(
      translateUpstreams({
        factorySourcePath: "/no/such/dir",
        factorySourceSha: "f".repeat(40),
        templatePath: template,
        templateSha: "e".repeat(40),
      })
    ).rejects.toThrow(/factory source/);
  });
});

describe("translateLegacyManifest", () => {
  const aimAdapter: FactoryAdapterRow = {
    name: "aim-vue-node",
    version: "3.0.0",
    sourceSha: "a".repeat(40),
  };

  function fullyExecutedManifest(): GoaSoftwareFactoryManifest {
    return {
      pipelineStatus: "COMPLETE",
      completedAt: "2026-04-22T00:00:00Z",
      factoryInputs: {
        clientTechStack: "Vue 3 + GoA Design System",
        templateVariant: "dual",
      },
      stages: {
        stage1_businessRequirements: {
          status: "PASSED",
          startedAt: "2026-04-21T13:16:00Z",
          completedAt: "2026-04-21T13:32:00Z",
          artifacts: [
            "requirements/business_requirements_document.md",
            "requirements/data_io_schema.json",
          ],
        },
        stage2_serviceRequirements: {
          status: "PASSED",
          startedAt: "2026-04-21T13:35:00Z",
          completedAt: "2026-04-21T14:15:00Z",
        },
        stage3_databaseDesign: {
          status: "PASSED",
          startedAt: "2026-04-21T14:48:00Z",
          completedAt: "2026-04-21T15:05:00Z",
        },
        stage4_apiControllers: {
          status: "PASSED",
          startedAt: "2026-04-21T15:30:00Z",
          completedAt: "2026-04-22T00:00:00Z",
        },
        stage5_clientInterface: {
          status: "PASSED",
          startedAt: "2026-04-21T18:00:00Z",
          completedAt: "2026-04-21T22:15:00Z",
        },
      },
    };
  }

  const workingState: GoaWorkingState = {
    schemaVersion: "1.2",
    completedAt: "2026-04-21T22:15:00Z",
    templateVariant: "dual",
  };

  test("fully-complete manifest produces 7-stage completed pipeline-state", () => {
    const out = translateLegacyManifest(fullyExecutedManifest(), workingState, [aimAdapter]);
    expect(out.schema_version).toBe("1.0.0");
    expect(out.pipeline.status).toBe("completed");
    expect(out.pipeline.adapter.name).toBe("aim-vue-node");
    expect(out.pipeline.adapter.version).toBe("3.0.0");
    // Seven stages: pre-flight + 5 mapped + adapter-handoff.
    expect(Object.keys(out.stages).sort()).toEqual(
      [
        "adapter-handoff",
        "api-specification",
        "business-requirements",
        "data-model",
        "pre-flight",
        "service-requirements",
        "ui-specification",
      ]
    );
    for (const [, entry] of Object.entries(out.stages)) {
      // adapter-handoff is pending without fileOwnership; every other stage
      // must be completed when the legacy run was fully executed.
      if (entry === out.stages["adapter-handoff"]) {
        expect(entry.status).toBe("pending");
      } else {
        expect(entry.status).toBe("completed");
      }
    }
  });

  test("preserves stage completion timestamps and artefact references", () => {
    const out = translateLegacyManifest(fullyExecutedManifest(), workingState, [aimAdapter]);
    const br = out.stages["business-requirements"];
    expect(br.started_at).toBe("2026-04-21T13:16:00Z");
    expect(br.completed_at).toBe("2026-04-21T13:32:00Z");
    expect(br.artifacts).toEqual([
      {
        path: "requirements/business_requirements_document.md",
        type: "document",
        hash: "",
      },
      {
        path: "requirements/data_io_schema.json",
        type: "data",
        hash: "",
      },
    ]);
  });

  test("partial legacy manifest produces paused pipeline with per-stage status", () => {
    const manifest = fullyExecutedManifest();
    manifest.pipelineStatus = "IN_PROGRESS";
    manifest.stages!.stage3_databaseDesign.status = "IN_PROGRESS";
    delete manifest.stages!.stage3_databaseDesign.completedAt;
    delete manifest.stages!.stage4_apiControllers;
    delete manifest.stages!.stage5_clientInterface;

    const out = translateLegacyManifest(manifest, workingState, [aimAdapter]);
    expect(out.pipeline.status).toBe("paused");
    expect(out.stages["business-requirements"].status).toBe("completed");
    expect(out.stages["service-requirements"].status).toBe("completed");
    expect(out.stages["data-model"].status).toBe("in_progress");
    expect(out.stages["api-specification"].status).toBe("pending");
    expect(out.stages["ui-specification"].status).toBe("pending");
  });

  test("synthesises adapter-handoff as completed when fileOwnership is present", () => {
    const manifest = fullyExecutedManifest();
    manifest.fileOwnership = {
      "apps/public/src/views/Home.vue": "factory",
    };
    const out = translateLegacyManifest(manifest, workingState, [aimAdapter]);
    expect(out.stages["adapter-handoff"].status).toBe("completed");
  });

  test("falls back to first adapter when no Vue hint matches", () => {
    const manifest = fullyExecutedManifest();
    manifest.factoryInputs = { clientTechStack: "React" };
    const other: FactoryAdapterRow = { name: "rust-axum", version: "0.1.0" };
    const out = translateLegacyManifest(manifest, workingState, [other, aimAdapter]);
    expect(out.pipeline.adapter.name).toBe("rust-axum");
  });

  test("conservative fallback adapter when orgAdapters is empty", () => {
    const out = translateLegacyManifest(fullyExecutedManifest(), workingState, []);
    expect(out.pipeline.adapter).toEqual({ name: "aim-vue-node", version: "0.0.0" });
  });

  test("emits a unique pipeline.id per invocation", () => {
    const a = translateLegacyManifest(fullyExecutedManifest(), workingState, [aimAdapter]);
    const b = translateLegacyManifest(fullyExecutedManifest(), workingState, [aimAdapter]);
    expect(a.pipeline.id).not.toBe(b.pipeline.id);
    expect(a.pipeline.id).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
    );
  });

  test("defers build-spec unification — emits placeholder path+hash", () => {
    const out = translateLegacyManifest(fullyExecutedManifest(), workingState, [aimAdapter]);
    expect(out.pipeline.build_spec).toEqual({ path: "requirements/", hash: "" });
  });
});
