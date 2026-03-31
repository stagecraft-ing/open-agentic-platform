export type {
  CreateAgentWorktreeOptions,
  OapWorktreeEntry,
  RemoveAgentWorktreeOptions,
} from "./types.js";
export { FifoConcurrencyLimiter } from "./concurrency.js";
export type { ConcurrencyMetrics } from "./concurrency.js";
export {
  WorktreeManagerError,
  WORKTREES_DIR_NAME,
  branchNameForAgent,
  createAgentWorktree,
  defaultWorktreePath,
  ensureWorktreesDirectoryIgnored,
  listOapWorktrees,
  parseWorktreeListPorcelain,
  reconcileOrphanWorktreeDirs,
  removeAgentWorktree,
  sameRealPath,
  worktreeDirectoryName,
} from "./worktree-manager.js";
