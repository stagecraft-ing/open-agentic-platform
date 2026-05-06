// Spec 139 Phase 1 — T010 round-trip parity test.
//
// LOCKS the no-consumer-break invariant for spec 108's external API surface.
// Walk a fixture upstream → run BOTH the legacy translator and the new
// substrate-then-projection chain → assert byte-identical TranslationResult
// (modulo skill-key insertion order, which is non-deterministic by readdir
// in the legacy `translateTemplate`; we sort keys on both sides before
// comparison — see research.md §6 risk 1).
//
// This is a pure-functional test (no DB). It runs under `npm test` and
// `encore test`. Halt trigger: if this test cannot be made green for a
// fixture that exercises every legacy bucket, the spec 108 round-trip
// invariant is unattainable and Phase 1 stops.

import { describe, expect, test, beforeAll, afterAll } from "vitest";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  translateUpstreams,
  translateUpstreamsToSubstrate,
  type TranslationResult,
} from "./translator";
import { projectSubstrateToLegacy } from "./projection";

async function writeTree(
  root: string,
  files: Record<string, string>,
): Promise<void> {
  for (const [rel, body] of Object.entries(files)) {
    const abs = join(root, rel);
    const parent = abs.substring(0, abs.lastIndexOf("/"));
    await mkdir(parent, { recursive: true });
    await writeFile(abs, body, "utf8");
  }
}

/**
 * Sort `manifest.skills` keys alphabetically on every adapter so the
 * comparison is independent of `readdir` insertion order. See
 * `research.md` §6 risk 1.
 */
function normalize(result: TranslationResult): TranslationResult {
  return {
    adapters: result.adapters.map((a) => {
      const m = (a.manifest ?? {}) as Record<string, unknown>;
      const skills = m.skills as Record<string, unknown> | undefined;
      const sortedSkills = skills
        ? Object.fromEntries(
            Object.entries(skills).sort(([k1], [k2]) => k1.localeCompare(k2)),
          )
        : undefined;
      const manifest: Record<string, unknown> = { ...m };
      if (sortedSkills) manifest.skills = sortedSkills;
      return { ...a, manifest };
    }),
    contracts: [...result.contracts].sort((c1, c2) =>
      c1.name.localeCompare(c2.name),
    ),
    processes: result.processes.map((p) => ({ ...p })),
  };
}

describe("spec 139 Phase 1 — round-trip parity (T010)", () => {
  let factory: string;
  let template: string;
  const factorySha = "f".repeat(40);
  const templateSha = "e".repeat(40);

  beforeAll(async () => {
    factory = await mkdtemp(join(tmpdir(), "spec139-rt-factory-"));
    template = await mkdtemp(join(tmpdir(), "spec139-rt-template-"));

    // Fixture exercises every legacy translator bucket so the projection
    // is forced to mirror them all.
    await writeTree(factory, {
      "README.md": "ignored",
      "Factory Agent/factory-orchestration.md":
        "---\nid: factory-orchestrator\ntype: orchestrator\nparent: none\n---\nroot orchestrator body",
      // process-stage path predicate (path wins over frontmatter)
      "Factory Agent/Orchestrator/factory-orchestration-s1.md":
        "---\nid: stage-s1\nparent: factory-orchestrator\ntype: stage-skill\n---\ns1 body",
      "Factory Agent/Orchestrator/factory-orchestration-s2.md":
        "---\nid: stage-s2\nparent: factory-orchestrator\ntype: stage-skill\n---\ns2 body",
      "Factory Agent/Orchestrator/factory-orchestration-cd.md":
        "---\nid: stage-cd\nparent: factory-orchestrator\ntype: stage-skill\n---\ncd body",
      "Factory Agent/Orchestrator/factory-orchestration-tm.md":
        "---\nid: stage-tm\nparent: factory-orchestrator\ntype: stage-skill\n---\ntm body",
      // Orchestrator/<other>.md → legacy "other" agents bucket
      "Factory Agent/Orchestrator/api-orchestrator.md":
        "---\nid: api-orchestrator\ntype: orchestrator\nparent: factory-orchestrator\n---\napi orch body",
      // Controllers
      "Factory Agent/Controllers/api-builder.md":
        "---\nid: api-builder\nparent: api-orchestrator\n---\nbuilder body",
      // Client_Interface
      "Factory Agent/Client_Interface/design-system.md":
        "---\nid: ci-design-system\nparent: ci-orchestrator\n---\ndesign body",
      // Requirements/{System,Service,Client}
      "Factory Agent/Requirements/System/business-requirements.md":
        "---\nid: business-requirements\nparent: svc-req-orchestrator\n---\nsystem body",
      "Factory Agent/Requirements/Service/audience-identification.md":
        "---\nid: svc-audience-identification\nparent: svc-req-orchestrator\n---\nservice body",
      "Factory Agent/Requirements/Client/client-document.md":
        "---\nid: sys-client-document\nparent: cd-orchestrator\n---\nclient body",
      // Database
      "Factory Agent/Database/database_architect.md":
        "---\nid: database-architect\nparent: api-orchestrator\n---\ndb body",
      // Reference-data JSON
      "Factory Agent/Requirements/Service/sitemap-template-public.json":
        '{"variant":"public"}',
      "Factory Agent/Requirements/System/service-catalog.json":
        '{"services":[]}',
      // Contract schema
      "Factory Agent/contracts/build-spec.schema.json":
        '{"$schema":"http://json-schema.org/draft-07/schema#","type":"object"}',
    });

    await writeTree(template, {
      "README.md": "excluded readme",
      "package.json": "excluded pkg",
      "package-lock.json": "excluded lock",
      "apps/web/index.tsx": "excluded app",
      "modules/auth.ts": "excluded module",
      "orchestration/template-orchestrator.md":
        "---\nid: template-orchestrator\ntype: orchestrator\nscope: template-implementation\n---\ntemplate orchestrator body",
      "orchestration/skills/analyze.md":
        "---\nid: analyze\ntype: skill\n---\nanalyze body",
      "orchestration/skills/configure.md":
        "---\nid: configure\ntype: skill\n---\nconfigure body",
      "orchestration/skills/scaffold-feature.md":
        "---\nid: scaffold-feature\ntype: skill\n---\nscaffold body",
      "orchestration/skills/trim.md":
        "---\nid: trim\ntype: skill\n---\ntrim body",
      "orchestration/skills/validate.md":
        "---\nid: validate\ntype: skill\n---\nvalidate body",
      // Schema in template tree, dedup-tested
      "orchestration/contracts/pipeline-state.schema.yaml":
        "type: object\ntitle: pipeline-state",
    });
  });

  afterAll(async () => {
    if (factory) await rm(factory, { recursive: true, force: true });
    if (template) await rm(template, { recursive: true, force: true });
  });

  test("substrate→projection produces deep-equal TranslationResult to legacy", async () => {
    // Spec 140 §2.1 — the substrate translator no longer carries
    // `templateRemote` / `templateDefaultBranch` options; the manifest
    // emits scaffold_source_id from OAP_NATIVE_ADAPTERS unconditionally.
    const opts = {
      factorySourcePath: factory,
      factorySourceSha: factorySha,
      templatePath: template,
      templateSha,
    };

    const legacy = await translateUpstreams(opts);
    const substrate = await translateUpstreamsToSubstrate(opts);
    const projected = projectSubstrateToLegacy(substrate);

    expect(normalize(projected)).toEqual(normalize(legacy));
  });

  test("substrate carries every fixture file as exactly one row", async () => {
    const opts = {
      factorySourcePath: factory,
      factorySourceSha: factorySha,
      templatePath: template,
      templateSha,
    };
    const substrate = await translateUpstreamsToSubstrate(opts);

    // 15 factory-tree files (1 root orch + 4 stages + 1 other agent + 1 controller
    // + 1 ci + 3 requirements + 1 db + 2 reference-data JSON + 1 contract-schema;
    // README.md excluded)
    // + 7 template-tree files (1 orchestrator + 5 skills + 1 contract-schema;
    // README/package*/apps/modules excluded)
    // = 22 substrate rows total.
    expect(substrate.rows).toHaveLength(22);

    // Every row carries a non-empty content_hash and a path.
    for (const row of substrate.rows) {
      expect(row.contentHash).toMatch(/^[0-9a-f]{64}$/);
      expect(row.path).not.toBe("");
      expect(row.origin).not.toBe("");
    }

    // No duplicate (origin, path) pairs.
    const ids = new Set<string>();
    for (const row of substrate.rows) {
      const key = `${row.origin}|${row.path}`;
      expect(ids.has(key)).toBe(false);
      ids.add(key);
    }
  });

  test("substrate kind classification respects path-precedence rule (D-1)", async () => {
    const opts = {
      factorySourcePath: factory,
      factorySourceSha: factorySha,
      templatePath: template,
      templateSha,
    };
    const substrate = await translateUpstreamsToSubstrate(opts);

    // process-stage wins over skill for factory-orchestration-*.md by path
    // even though their frontmatter declares `parent: factory-orchestrator`
    // (which would match the `skill` predicate).
    const stages = substrate.rows.filter((r) => r.kind === "process-stage");
    expect(stages.map((r) => r.path).sort()).toEqual([
      "Factory Agent/Orchestrator/factory-orchestration-cd.md",
      "Factory Agent/Orchestrator/factory-orchestration-s1.md",
      "Factory Agent/Orchestrator/factory-orchestration-s2.md",
      "Factory Agent/Orchestrator/factory-orchestration-tm.md",
    ]);

    // pipeline-orchestrator: top-level factory-orchestration.md +
    // template's orchestration/template-orchestrator.md.
    const orchestrators = substrate.rows.filter(
      (r) => r.kind === "pipeline-orchestrator",
    );
    expect(orchestrators.map((r) => r.path).sort()).toEqual([
      "Factory Agent/factory-orchestration.md",
      "orchestration/template-orchestrator.md",
    ]);

    // reference-data: load-bearing JSON under Requirements/{System,Service}/
    const refdata = substrate.rows.filter((r) => r.kind === "reference-data");
    expect(refdata.map((r) => r.path).sort()).toEqual([
      "Factory Agent/Requirements/Service/sitemap-template-public.json",
      "Factory Agent/Requirements/System/service-catalog.json",
    ]);

    // contract-schema: *.schema.{json,yaml,yml}
    const contracts = substrate.rows.filter((r) => r.kind === "contract-schema");
    expect(contracts.map((r) => r.path).sort()).toEqual([
      "Factory Agent/contracts/build-spec.schema.json",
      "orchestration/contracts/pipeline-state.schema.yaml",
    ]);
  });

  test("substrate excludes Orchestrator/scripts/ (D-1 sync exclusions)", async () => {
    // Add a scripts file to the factory fixture and re-run; the row count
    // must not grow.
    await mkdir(join(factory, "Factory Agent/Orchestrator/scripts"), {
      recursive: true,
    });
    await writeFile(
      join(factory, "Factory Agent/Orchestrator/scripts/excluded.py"),
      "should be excluded",
      "utf8",
    );
    await mkdir(
      join(factory, "Factory Agent/Orchestrator/scripts/ppt-assets"),
      { recursive: true },
    );
    await writeFile(
      join(factory, "Factory Agent/Orchestrator/scripts/ppt-assets/asset.png"),
      "binary placeholder",
      "utf8",
    );

    const opts = {
      factorySourcePath: factory,
      factorySourceSha: factorySha,
      templatePath: template,
      templateSha,
    };
    const substrate = await translateUpstreamsToSubstrate(opts);

    const scriptHits = substrate.rows.filter((r) =>
      r.path.includes("Orchestrator/scripts/"),
    );
    expect(scriptHits).toHaveLength(0);
  });

  test("substrate emits parsed frontmatter for .md files", async () => {
    const opts = {
      factorySourcePath: factory,
      factorySourceSha: factorySha,
      templatePath: template,
      templateSha,
    };
    const substrate = await translateUpstreamsToSubstrate(opts);

    const orchRow = substrate.rows.find(
      (r) => r.path === "Factory Agent/factory-orchestration.md",
    );
    expect(orchRow).toBeDefined();
    expect(orchRow!.frontmatter).not.toBeNull();
    expect(orchRow!.frontmatter!.id).toBe("factory-orchestrator");
    expect(orchRow!.frontmatter!.type).toBe("orchestrator");

    // Non-md (JSON) rows have null frontmatter.
    const refRow = substrate.rows.find((r) => r.path.endsWith(".json"));
    expect(refRow!.frontmatter).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Spec 140 Phase 1 — buildAdapter emit-side regression tests (T010 / T011).
// ---------------------------------------------------------------------------

describe("spec 140 Phase 1 — aim-vue-node manifest cutover (T010)", () => {
  const factorySha = "f".repeat(40);
  const templateSha = "e".repeat(40);

  test("buildAdapter emits scaffold_source_id / orchestration_source_id / scaffold_runtime — and no template_remote / template_default_branch", () => {
    const substrate = {
      rows: [
        {
          origin: "aim-vue-node-template",
          path: "orchestration/template-orchestrator.md",
          kind: "pipeline-orchestrator" as const,
          bundleId: null,
          upstreamSha: templateSha,
          upstreamBody: "orch body",
          contentHash: "a".repeat(64),
          frontmatter: null,
        },
        {
          origin: "aim-vue-node-template",
          path: "orchestration/skills/analyze.md",
          kind: "skill" as const,
          bundleId: null,
          upstreamSha: templateSha,
          upstreamBody: "analyze body",
          contentHash: "b".repeat(64),
          frontmatter: null,
        },
      ],
      factorySourceSha: factorySha,
      templateSourceSha: templateSha,
      factoryOriginId: "goa-software-factory",
      templateOriginId: "aim-vue-node-template",
    };

    const projected = projectSubstrateToLegacy(substrate);
    expect(projected.adapters).toHaveLength(1);
    const m = projected.adapters[0].manifest as Record<string, unknown>;

    // Spec 140 AC-2 — values are sourced from the
    // `OAP_NATIVE_ADAPTERS["aim-vue-node"]` constant.
    expect(m.scaffold_source_id).toBe("aim-vue-node-template");
    expect(m.orchestration_source_id).toBe("goa-software-factory");
    expect(m.scaffold_runtime).toBe("node-24");

    // The legacy fields are gone.
    expect(m.template_remote).toBeUndefined();
    expect(m.template_default_branch).toBeUndefined();
  });
});

describe("spec 140 Phase 1 — adapter de-dup priority (T011)", () => {
  const factorySha = "f".repeat(40);
  const templateSha = "e".repeat(40);

  test("oap-self adapter-manifest row wins over template-origin synthetic when both name aim-vue-node", () => {
    // The oap-self row's manifest body — distinguished from the template
    // synthetic by carrying its own `__companion` envelope and a
    // recognisable display marker the synthetic never emits.
    const oapSelfManifestBody = [
      "adapter:",
      "  name: aim-vue-node",
      "  display_name: 'oap-self winning manifest'",
      "orchestration_source_id: goa-software-factory",
      "scaffold_source_id: aim-vue-node-template",
      "scaffold_runtime: node-24",
      "",
    ].join("\n");

    const substrate = {
      rows: [
        // Template-origin synthetic content — orchestrator + a skill so
        // `buildAdapter` emits an aim-vue-node entry.
        {
          origin: "aim-vue-node-template",
          path: "orchestration/template-orchestrator.md",
          kind: "pipeline-orchestrator" as const,
          bundleId: null,
          upstreamSha: templateSha,
          upstreamBody: "template orch body",
          contentHash: "a".repeat(64),
          frontmatter: null,
        },
        // Oap-self adapter-manifest row — `buildOapNativeAdapters`
        // recognises the `adapters/<name>/manifest.yaml` shape.
        {
          origin: "oap-self",
          path: "adapters/aim-vue-node/manifest.yaml",
          kind: "adapter-manifest" as const,
          bundleId: null,
          upstreamSha: "oap-self/aim-vue-node/spec-140-migration-36",
          upstreamBody: oapSelfManifestBody,
          contentHash: "c".repeat(64),
          frontmatter: null,
        },
      ],
      factorySourceSha: factorySha,
      templateSourceSha: templateSha,
      factoryOriginId: "goa-software-factory",
      templateOriginId: "aim-vue-node-template",
    };

    const projected = projectSubstrateToLegacy(substrate);
    const aim = projected.adapters.find((a) => a.name === "aim-vue-node");
    expect(aim).toBeDefined();

    const manifest = aim!.manifest as Record<string, unknown>;

    // Spec 140 §2.4 — oap-self wins. Tell-tale: the
    // `__companion` envelope is ONLY emitted by `buildOapNativeAdapters`.
    expect(manifest.__companion).toBeDefined();

    // The oap-self row's display_name is distinctive.
    const adapterMeta = manifest.adapter as Record<string, unknown>;
    expect(adapterMeta.display_name).toBe("oap-self winning manifest");

    // The template-origin synthetic's `entry: orchestration/...` and
    // `skills` keys are NOT present (the synthetic was suppressed).
    expect(manifest.entry).toBeUndefined();
    expect(manifest.skills).toBeUndefined();
  });
});
