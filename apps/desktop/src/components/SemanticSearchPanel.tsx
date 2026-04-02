import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@opc/ui/button';

interface SemanticSearchPanelProps {
  /** When provided, used as the project identifier for blockoli search. */
  projectPath?: string;
}

export const SemanticSearchPanel: React.FC<SemanticSearchPanelProps> = ({ projectPath }) => {
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<any>(null);

  // Derive project name from path, falling back to 'default'
  const projectName = projectPath
    ? projectPath.split('/').filter(Boolean).pop() ?? 'default'
    : 'default';

  const handleSearch = async () => {
    if (!query) return;
    setLoading(true);
    try {
      const res = await invoke('blockoli_search', { projectName, query, projectPath });
      setResult(res);
    } catch (err) {
      console.error(err);
      setResult({ error: String(err) });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="p-6 h-full flex flex-col gap-4 text-foreground">
      <h1 className="text-2xl font-bold">Blockoli Semantic Search</h1>
      {projectPath && (
        <p className="text-sm text-muted-foreground -mt-2 font-mono truncate">{projectPath}</p>
      )}
      <div className="flex gap-2">
        <input
          className="flex-1 px-3 py-2 bg-background border border-input rounded-md text-foreground"
          value={query}
          onChange={e => setQuery(e.target.value)}
          placeholder="Search codebase naturally..."
          onKeyDown={e => e.key === 'Enter' && handleSearch()}
        />
        <Button onClick={handleSearch} disabled={loading || !query}>
          {loading ? 'Searching...' : 'Search'}
        </Button>
      </div>
      <div className="flex-1 overflow-auto bg-muted p-4 rounded-md border text-foreground">
        {result ? (
          <pre className="text-sm whitespace-pre-wrap font-mono">
            {JSON.stringify(result, null, 2)}
          </pre>
        ) : (
          <div className="text-muted-foreground text-center mt-10">
            Enter a query to search semantically across the indexed codebase.
          </div>
        )}
      </div>
    </div>
  );
};
