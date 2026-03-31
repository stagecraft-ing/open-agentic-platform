// Session Memory — @opc/session-memory

export type {
  MemoryEntry,
  MemoryKind,
  ImportanceLevel,
  StoreMemoryInput,
  QueryMemoryInput,
  ListMemoryInput,
} from "./types.js";

export {
  EXPIRY_DEFAULTS,
  IMPORTANCE_ORDER,
  PROMOTION_ACCESS_THRESHOLD,
} from "./types.js";

export { MemoryStorage, dbPath } from "./storage/sqlite.js";
export { applyMigrations, getCurrentVersion, MIGRATIONS } from "./storage/migrations.js";

export { MemoryServer, TOOL_DEFINITIONS } from "./server.js";
export type { McpToolDefinition, McpRequest, McpResponse, MemoryServerOptions } from "./server.js";

export { handleMemoryStore } from "./tools/store.js";
export { handleMemoryQuery } from "./tools/query.js";
export { handleMemoryDelete } from "./tools/delete.js";
export { handleMemoryList } from "./tools/list.js";

export { harvest, signalsToStoreInputs } from "./harvesting/engine.js";
export type { HarvestedSignal, HarvestResult, HarvestOptions } from "./harvesting/engine.js";
export { BUILTIN_RULES } from "./harvesting/rules.js";
export type { HarvestRule } from "./harvesting/rules.js";
export { loadHarvestRules, configToRule, validateRuleConfig } from "./harvesting/rules-loader.js";
export type { RuleConfig, HarvestRulesConfig, LoadRulesResult } from "./harvesting/rules-loader.js";

export { ExpirySweeper } from "./expiry/sweeper.js";
export type { SweeperOptions, SweeperResult } from "./expiry/sweeper.js";
export { runPromotion, getNextImportance } from "./expiry/promotion.js";
export type { PromotionResult } from "./expiry/promotion.js";

export { loadSessionMemories, formatMemoriesForPrompt, composeSessionPrompt } from "./integration.js";
export type { SessionLoadOptions, SessionLoadResult } from "./integration.js";
