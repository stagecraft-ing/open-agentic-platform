import React from 'react';
import { GitContextSurface } from '@/features/git/GitContextSurface';

/** Git Context tab — Feature 032 T004–T005 (native git state, no sidecar MCP in this slice). */
export const GitContextPanel: React.FC = () => {
  return <GitContextSurface />;
};
