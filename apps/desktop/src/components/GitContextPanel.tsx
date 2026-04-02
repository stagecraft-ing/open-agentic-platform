import React from 'react';
import { GitContextSurface } from '@/features/git/GitContextSurface';

/** Git Context tab — native git is source-of-truth; optional gitctx MCP enrichment via Rust bridge (T006). */
export const GitContextPanel: React.FC<{ projectPath?: string }> = ({ projectPath }) => {
  return <GitContextSurface projectPath={projectPath} />;
};
