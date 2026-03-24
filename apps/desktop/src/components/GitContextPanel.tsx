import React from 'react';

export const GitContextPanel: React.FC = () => {
  return (
    <div className="p-6 h-full flex flex-col gap-4 text-foreground">
      <h1 className="text-2xl font-bold">Git Context Analysis</h1>
      <div className="flex-1 overflow-auto bg-muted p-4 rounded-md border text-foreground flex items-center justify-center">
        <div className="text-muted-foreground text-center max-w-md">
          <p className="mb-4">This panel will connect directly with the Gitctx sidecar or MCP server to provide contextual insights and diff analysis.</p>
          <p>Please implement sidecar communication first.</p>
        </div>
      </div>
    </div>
  );
};
