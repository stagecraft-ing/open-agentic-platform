import { invoke } from '@tauri-apps/api/core';

export type McpServerName = 'gitctx';

export interface McpClient {
  listTools(): Promise<unknown>;
  callTool(toolName: string, args: Record<string, unknown>): Promise<unknown>;
  readResource(uri: string): Promise<unknown>;
}

export class McpClientError extends Error {
  readonly type:
    | 'SidecarNotReady'
    | 'TransportError'
    | 'McpRpcError'
    | 'Timeout'
    | 'Unknown';

  constructor(
    message: string,
    type: McpClientError['type'] = 'Unknown',
    readonly causeValue?: unknown,
  ) {
    super(message);
    this.name = 'McpClientError';
    this.type = type;
  }
}

function classifyError(e: unknown): McpClientError {
  if (e instanceof McpClientError) return e;
  const message = e instanceof Error ? e.message : String(e);
  if (/Timed out/i.test(message)) {
    return new McpClientError(message, 'Timeout', e);
  }
  if (/Unsupported MCP server/i.test(message)) {
    return new McpClientError(message, 'McpRpcError', e);
  }
  if (/spawn|stdin|stdout|not found/i.test(message)) {
    return new McpClientError(message, 'TransportError', e);
  }
  if (/MCP .* error/i.test(message)) {
    return new McpClientError(message, 'McpRpcError', e);
  }
  return new McpClientError(message, 'Unknown', e);
}

async function mcpInvoke<T>(command: string, payload: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, payload);
  } catch (e) {
    throw classifyError(e);
  }
}

export function createMcpClient(server: McpServerName): McpClient {
  return {
    async listTools() {
      return await mcpInvoke('mcp_list_tools', { server });
    },
    async callTool(toolName: string, args: Record<string, unknown>) {
      return await mcpInvoke('mcp_call_tool', { server, toolName, args });
    },
    async readResource(uri: string) {
      return await mcpInvoke('mcp_read_resource', { server, uri });
    },
  };
}
