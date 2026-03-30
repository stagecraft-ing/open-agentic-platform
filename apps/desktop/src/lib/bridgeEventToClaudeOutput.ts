import type { BridgeEvent } from "@opc/claude-code-bridge/types";

/**
 * Maps spec 045 {@link BridgeEvent} values to JSONL lines compatible with the
 * existing `claude-output` / `useClaudeMessages` path (stream-json shape).
 *
 * When the Tauri backend runs the bridge (sidecar or embedded Node), it should
 * emit each returned string as one `claude-output` payload so the UI parser
 * stays unchanged.
 */
export function bridgeEventToClaudeOutputLines(event: BridgeEvent): string[] {
  switch (event.kind) {
    case "start":
      return [];
    case "message":
      return [JSON.stringify(event.message)];
    case "permission-request":
      return [
        JSON.stringify({
          type: "bridge_permission_request",
          request_id: event.requestId,
          tool_name: event.toolName,
          tool_input: event.toolInput,
        }),
      ];
    case "session-complete": {
      const s = event.summary;
      return [
        JSON.stringify({
          type: "result",
          subtype: s.isError ? "error" : "success",
          session_id: s.sessionId,
          total_cost_usd: s.totalCostUsd,
          total_input_tokens: s.inputTokens,
          total_output_tokens: s.outputTokens,
          num_turns: s.numTurns,
          duration_ms: s.durationMs,
          duration_api_ms: 0,
        }),
      ];
    }
    case "error":
      return [
        JSON.stringify({
          type: "error",
          error: event.error,
          fatal: event.fatal,
        }),
      ];
    default: {
      const _never: never = event;
      return _never;
    }
  }
}
