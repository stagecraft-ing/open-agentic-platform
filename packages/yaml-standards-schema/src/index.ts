// Types
export type {
  RuleVerb,
  StandardPriority,
  StandardStatus,
  StandardRule,
  AntiPattern,
  StandardExample,
  CodingStandard,
  DiagnosticSeverity,
  StandardDiagnostic,
  ParseStandardResult,
} from "./types.js";

// Schema validation
export { validateStandardObject, diag } from "./schema.js";

// Parser
export { parseStandardFile } from "./parser.js";

// Loader (Phase 2)
export type { TierName, TierResult, LoadResult } from "./loader.js";
export { loadStandardsFromDir, loadAllTiers } from "./loader.js";

// Resolver (Phase 2)
export type { StandardsFilter, ResolveResult } from "./resolver.js";
export { resolveStandards } from "./resolver.js";

// Contributor pipeline (Phase 4)
export type {
  FindingSource,
  ExecutionFinding,
  AggregatedFinding,
  AggregateResult,
  GenerateCandidateOptions,
  GeneratedCandidate,
  GenerateCandidatesResult,
} from "./pipeline.js";
export {
  aggregateFindings,
  generateCandidates,
  runContributorPipeline,
} from "./pipeline.js";
