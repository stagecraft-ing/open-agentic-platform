// Spec 139 Phase 2 — pure-functional sanitisers for OAP-native adapter
// ingestion (T054 + T055 / D-4 fixes).
//
// Lives in its own module (separate from `oapNativeIngest.ts`) so the
// pure-function tests can run under bare vitest without pulling in the
// encore runtime via the DB import chain.
//
// Spec 140 AC-2 — `OAP_NATIVE_ADAPTERS` moved to `oapNativeAdapters.ts`
// so `translator.ts` and `projection.ts` can read the canonical
// scaffold-source-id constants without an import cycle. Re-exported here
// to keep existing callers (`oapNativeIngest.ts`, dispatch tests) stable.

import { parse as parseYaml, stringify as stringifyYaml } from "yaml";
import { extractFrontmatter } from "./translator";
import type { OapNativeAdapterConfig } from "./oapNativeAdapters";

export {
  OAP_NATIVE_ADAPTERS,
  type OapNativeAdapterConfig,
} from "./oapNativeAdapters";

export type SanitiseInput = {
  rel: string;
  body: string;
  adapterName: string;
  config: OapNativeAdapterConfig;
};

export type SanitiseOutput = {
  body: string;
  frontmatter: Record<string, unknown> | null;
};

export async function sanitiseForIngest(
  input: SanitiseInput,
): Promise<SanitiseOutput> {
  // 1. Manifest.yaml — runtime bump + key injection + drop validation block.
  if (input.rel === "manifest.yaml") {
    return sanitiseManifest(input);
  }
  // 2. Patterns — auto-generate minimal frontmatter.
  if (/^patterns\//.test(input.rel) && /\.md$/i.test(input.rel)) {
    return sanitisePattern(input);
  }
  // 3. Everything else — pass through verbatim.
  return { body: input.body, frontmatter: null };
}

function sanitiseManifest(input: SanitiseInput): SanitiseOutput {
  let parsed: Record<string, unknown>;
  try {
    parsed = (parseYaml(input.body) ?? {}) as Record<string, unknown>;
  } catch {
    return { body: input.body, frontmatter: null };
  }

  // D-4 fix #1 — bump runtime when configured.
  if (input.config.runtimeOverride !== null && parsed.stack) {
    const stack = parsed.stack as Record<string, unknown>;
    stack.runtime = input.config.runtimeOverride;
  }

  // T055 — inject Phase 2 manifest keys at the top level.
  parsed.orchestration_source_id = input.config.orchestrationSourceId;
  parsed.scaffold_source_id = input.config.scaffoldSourceId;
  parsed.scaffold_runtime = input.config.scaffoldRuntime;

  // D-4 fix #3 — drop the duplicate validation block; the canonical
  // copy lives at `validation/invariants.yaml`.
  delete parsed.validation;

  const newBody = stringifyYaml(parsed);
  return { body: newBody, frontmatter: parsed };
}

function sanitisePattern(input: SanitiseInput): SanitiseOutput {
  // D-4 fix #4 — auto-generate minimal frontmatter on patterns. Existing
  // frontmatter is preserved if present.
  const existing = extractFrontmatter(input.body);
  if (existing.frontmatter !== null) {
    return { body: input.body, frontmatter: existing.frontmatter };
  }

  const relPath = input.rel.replace(/^patterns\//, "").replace(/\.md$/, "");
  const id = `${input.adapterName}-pattern-${relPath.replace(/\//g, "-")}`;
  const category = relPath.split("/")[0];
  const frontmatter: Record<string, unknown> = {
    id,
    adapter: input.adapterName,
    category,
  };

  const yamlBody = stringifyYaml(frontmatter).trimEnd();
  const newBody = `---\n${yamlBody}\n---\n\n${existing.bodyOnly}`;
  return { body: newBody, frontmatter };
}
