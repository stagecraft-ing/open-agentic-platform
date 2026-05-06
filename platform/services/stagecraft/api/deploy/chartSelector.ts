// Spec 136 Phase 2 — chartSelector resolves a Helm chart for a tenant
// project's deployment. tenant-hello is the first (and currently only)
// registered shape; the selector exists as a building block so future
// shapes can be added without rewriting the deployd-api wire contract.
//
// Input: a minimal descriptor of the project's deployable shape
// (factory adapter id + a shape hint that future codepaths can derive
// from the project's package manifest, Dockerfile heuristics, or an
// explicit operator choice). Output: a chart name deployd-api-rs
// applies via Helm.
//
// Out of scope here:
// - Loading the chart from a registry vs. embedded chart files.
// - Driving Helm from deployd-api-rs (today the orchestrator builds raw
//   K8s objects via kube-rs; the migration to Helm is tracked under
//   spec 136 §Phase 2.b — a follow-up that lands the actual `applies it
//   via Helm` half).

/**
 * Tenant codebase shapes recognised by the chartSelector. tenant-hello
 * is the canonical reference shape; other shapes will land alongside
 * future tenant-* charts.
 */
export type TenantShape = "tenant-hello";

/**
 * Minimal per-project descriptor for chart resolution. Future shape
 * detection (multi-service, Rust tenant, Python tenant, etc.) layers
 * additional fields onto this.
 */
export type ChartSelectorInput = {
  /** The project's recognised shape, today always "tenant-hello". */
  shape: TenantShape;
};

export type ChartSelection = {
  /** Helm chart name as packaged under platform/charts/. */
  chart: string;
  /** Pinned chart version; aligns with platform/charts/<chart>/Chart.yaml. */
  version: string;
};

const CHART_REGISTRY: Record<TenantShape, ChartSelection> = {
  "tenant-hello": { chart: "tenant-hello", version: "0.1.0" },
};

/**
 * Resolve the Helm chart that deployd-api should apply for this
 * project's deployment. Throws on unknown shapes — refusing to silently
 * fall back is the load-bearing safety property: a project whose shape
 * isn't registered must surface that failure at deploy time, not get
 * shoehorned into a default chart.
 */
export function selectChart(input: ChartSelectorInput): ChartSelection {
  const sel = CHART_REGISTRY[input.shape];
  if (!sel) {
    throw new Error(
      `chartSelector: no chart registered for shape "${input.shape}". ` +
        `Registered shapes: ${Object.keys(CHART_REGISTRY).join(", ")}.`,
    );
  }
  return sel;
}

/** Listing of all registered shapes — useful for admin UIs and tests. */
export function listShapes(): TenantShape[] {
  return Object.keys(CHART_REGISTRY) as TenantShape[];
}
