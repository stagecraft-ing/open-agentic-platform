import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button } from '@opc/ui/button';

export const GovernancePanel: React.FC = () => {
  const [featuresPath, setFeaturesPath] = useState('');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<any>(null);

  const handleOverview = async () => {
    if (!featuresPath) return;
    setLoading(true);
    try {
      const res = await invoke('featuregraph_overview', { featuresYamlPath: featuresPath });
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
      <h1 className="text-2xl font-bold">Featuregraph Governance</h1>
      <div className="flex gap-2">
        <input 
          className="flex-1 px-3 py-2 bg-background border border-input rounded-md text-foreground"
          value={featuresPath}
          onChange={e => setFeaturesPath(e.target.value)}
          placeholder="Enter path to features yaml..."
        />
        <Button onClick={handleOverview} disabled={loading || !featuresPath}>
          {loading ? 'Loading...' : 'Load Overview'}
        </Button>
      </div>
      <div className="flex gap-2 text-sm text-muted-foreground items-center mb-2">
        <Button variant="outline" size="sm" onClick={() => console.log('Connect to MCP')}>
          Connect to AxiomRegent MCP
        </Button>
        <span className="italic">Uses @opc/mcp-client for advanced custom rule queries.</span>
      </div>
      <div className="flex-1 overflow-auto bg-muted p-4 rounded-md border text-foreground">
        {result ? (
          <pre className="text-sm whitespace-pre-wrap font-mono">
            {JSON.stringify(result, null, 2)}
          </pre>
        ) : (
          <div className="text-muted-foreground text-center mt-10">
            Enter features configuration path to load governance overview.
          </div>
        )}
      </div>
    </div>
  );
};
