export type {
  Action,
  ActionExecutionResult,
  ActionType,
  ConditionLeaf,
  ConditionNode,
  Diagnostic,
  DiagnosticSeverity,
  EvaluationResult,
  HookEvent,
  HookEventType,
  Matcher,
  ParseRuleResult,
  Rule,
  TerminalDecision,
} from "./types.js";

export { parseRuleFile, parseRuleSet } from "./parser.js";
export { evaluateConditionNode } from "./conditions.js";
export { matchesRuleEventType, matchesRuleMatcher } from "./matcher.js";
export { executeRuleAction } from "./actions.js";
export { evaluate } from "./engine.js";
export type { EvaluateInput } from "./engine.js";

export {
  createRuleRuntime,
  DEFAULT_RULES_DIR,
  discoverMarkdownRuleFiles,
  loadRules,
} from "./loader.js";
export type { LoaderConfig, RuleRuntime, RulesetSnapshot } from "./loader.js";
