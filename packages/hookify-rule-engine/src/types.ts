export type HookEventType =
  | "PreToolUse"
  | "PostToolUse"
  | "UserPromptSubmit"
  | "Stop";

export type DiagnosticSeverity = "error" | "warning" | "info";

export interface Diagnostic {
  code: string;
  severity: DiagnosticSeverity;
  message: string;
  filePath?: string;
  ruleId?: string;
}

export interface Matcher {
  tool?: string;
  input?: Record<string, unknown>;
  output?: Record<string, unknown>;
}

export interface ConditionLeaf {
  field: string;
  "=="?: unknown;
  "!="?: unknown;
  contains?: string;
  matches?: string;
  glob?: string;
}

export interface ConditionAllNode {
  all: ConditionNode[];
}

export interface ConditionAnyNode {
  any: ConditionNode[];
}

export interface ConditionNotNode {
  not: ConditionNode;
}

export type ConditionNode =
  | ConditionLeaf
  | ConditionAllNode
  | ConditionAnyNode
  | ConditionNotNode;

export type ActionType = "block" | "warn" | "modify";

export interface ModifyTransform {
  type: string;
  [key: string]: unknown;
}

export interface Action {
  type: ActionType;
  transform?: ModifyTransform;
}

export interface Rule {
  id: string;
  event: HookEventType;
  matcher: Matcher;
  conditions: ConditionNode;
  action: Action;
  priority: number;
  rationale: string;
  sourcePath: string;
}

export interface HookEvent {
  type: HookEventType;
  payload: Record<string, unknown>;
}

export interface EvaluationResult {
  allowed: boolean;
  blockedByRuleId?: string;
  /** When `allowed` is false due to a block action, human-facing rationale from the rule body. */
  blockRationale?: string;
  warnings: string[];
  diagnostics: Diagnostic[];
  payload: Record<string, unknown>;
  terminalDecision?: TerminalDecision;
  matchedRuleIds?: string[];
}

export type TerminalDecision = "allowed" | "blocked";

export interface ActionExecutionResult {
  payload: Record<string, unknown>;
  warnings: string[];
  diagnostics: Diagnostic[];
  terminalDecision: TerminalDecision;
  matchedRuleIds: string[];
  blockedByRuleId?: string;
}

export interface ParseRuleResult {
  rule: Rule | null;
  diagnostics: Diagnostic[];
}
