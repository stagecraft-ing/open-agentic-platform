// Spec 140 AC-2 — single source of truth for the OAP-native adapter
// source-id constants. Extracted from `oapNativeSanitise.ts` so the
// projection (`projection.ts`) and the upstream translator
// (`translator.ts`) can reach the canonical `scaffoldSourceId` /
// `orchestrationSourceId` / `scaffoldRuntime` values without inducing an
// import cycle through `extractFrontmatter` (which lives in
// `translator.ts`).
//
// Spec 139 §7.2 + spec 140 §2.1 — the manifest fields each adapter
// emits (`orchestration_source_id`, `scaffold_source_id`,
// `scaffold_runtime`) are sourced from this table only.

export type OapNativeAdapterConfig = {
  /** Adapter name; matches the directory name on disk. */
  adapterName: string;
  /** Required runtime override at ingest, or null to keep existing. */
  runtimeOverride: string | null;
  /** Spec 139 §7.2 manifest extension — Phase 2 keys. */
  orchestrationSourceId: string;
  scaffoldSourceId: string;
  scaffoldRuntime: string;
};

export const OAP_NATIVE_ADAPTERS: Record<string, OapNativeAdapterConfig> = {
  "next-prisma": {
    adapterName: "next-prisma",
    runtimeOverride: "node-24", // D-4 fix #1 — bump from node-22
    orchestrationSourceId: "oap-next-prisma",
    scaffoldSourceId: "oap-next-prisma-scaffold",
    scaffoldRuntime: "node-24",
  },
  "rust-axum": {
    adapterName: "rust-axum",
    runtimeOverride: null, // `native` already satisfies spec 112 §5.4
    orchestrationSourceId: "oap-rust-axum",
    scaffoldSourceId: "oap-rust-axum-scaffold",
    scaffoldRuntime: "native",
  },
  "encore-react": {
    adapterName: "encore-react",
    runtimeOverride: "node-24", // D-4 fix #1 — bump from node-20
    orchestrationSourceId: "oap-encore-react",
    scaffoldSourceId: "oap-encore-react-scaffold",
    scaffoldRuntime: "node-24",
  },
  "aim-vue-node": {
    adapterName: "aim-vue-node",
    runtimeOverride: null,
    orchestrationSourceId: "goa-software-factory",
    scaffoldSourceId: "aim-vue-node-template",
    scaffoldRuntime: "node-24",
  },
};

/**
 * Spec 140 AC-2 — canonical aim-vue-node config. Re-exported as a named
 * helper so callers can be explicit about which adapter they're sourcing
 * from (rather than indexing into the map with a string literal at every
 * read site).
 */
export const AIM_VUE_NODE_CONFIG = OAP_NATIVE_ADAPTERS["aim-vue-node"];
