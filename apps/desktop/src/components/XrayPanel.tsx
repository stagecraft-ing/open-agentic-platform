import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
// Using standard HTML input for simplicity
import { Button } from '@opc/ui/button';

export const XrayPanel: React.FC = () => {
  const [path, setPath] = useState('');
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<any>(null);

  const handleScan = async () => {
    if (!path) return;
    setLoading(true);
    try {
      const res = await invoke('xray_scan_project', { path });
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
      <h1 className="text-2xl font-bold">Xray Architecture Analysis</h1>
      <div className="flex gap-2">
        <input 
          className="flex-1 px-3 py-2 bg-background border border-input rounded-md text-foreground"
          value={path}
          onChange={e => setPath(e.target.value)}
          placeholder="Enter absolute project path..."
        />
        <Button onClick={handleScan} disabled={loading || !path}>
          {loading ? 'Scanning...' : 'Scan Project'}
        </Button>
      </div>
      <div className="flex-1 overflow-auto bg-muted p-4 rounded-md border text-foreground">
        {result ? (
          <pre className="text-sm whitespace-pre-wrap font-mono">
            {JSON.stringify(result, null, 2)}
          </pre>
        ) : (
          <div className="text-muted-foreground text-center mt-10">
            Enter a project path and click Scan to see the architecture index.
          </div>
        )}
      </div>
    </div>
  );
};
