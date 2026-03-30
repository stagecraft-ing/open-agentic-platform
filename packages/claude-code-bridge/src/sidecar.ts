/**
 * Node sidecar entry for Tauri IPC (spec 045): stdin JSONL control, stdout JSONL
 * stream compatible with `claude-output` consumers.
 *
 * Protocol: first stdin line is the query object; further lines may be
 * `permission-response` or `abort`. Stdout emits mapped JSONL lines plus a
 * final `{"done":true}` sentinel.
 */
import { createInterface } from "node:readline";
import { bridgeEventToClaudeOutputLines } from "./claude-output-lines.js";
import { queryClaudeCode } from "./index.js";
import { PermissionBroker } from "./permission-broker.js";
import type { BridgeQueryOptions, PermissionMode } from "./types.js";

interface SidecarQueryMessage {
  type: "query";
  prompt: string;
  workingDirectory: string;
  model?: string;
  sessionId?: string;
  permissionMode?: PermissionMode;
  oauthToken?: string;
}

interface PermissionResponseMessage {
  type: "permission-response";
  requestId: string;
  allowed: boolean;
}

interface AbortMessage {
  type: "abort";
}

function isAbortMessage(m: unknown): m is AbortMessage {
  return (
    typeof m === "object" &&
    m !== null &&
    (m as AbortMessage).type === "abort"
  );
}

function isPermissionResponse(m: unknown): m is PermissionResponseMessage {
  return (
    typeof m === "object" &&
    m !== null &&
    (m as PermissionResponseMessage).type === "permission-response"
  );
}

function parseQuery(raw: string): SidecarQueryMessage {
  const v = JSON.parse(raw) as unknown;
  if (
    typeof v !== "object" ||
    v === null ||
    (v as SidecarQueryMessage).type !== "query"
  ) {
    throw new Error("First stdin line must be a JSON object with type: \"query\"");
  }
  return v as SidecarQueryMessage;
}

async function run(): Promise<void> {
  const rl = createInterface({
    input: process.stdin,
    crlfDelay: Infinity,
  });

  const it = rl[Symbol.asyncIterator]();
  const first = await it.next();
  if (first.done || typeof first.value !== "string") {
    process.stderr.write("sidecar: missing query line on stdin\n");
    process.exit(1);
  }

  let q: SidecarQueryMessage;
  try {
    q = parseQuery(first.value);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    process.stderr.write(`sidecar: ${msg}\n`);
    rl.close();
    process.exit(1);
    return;
  }

  const ac = new AbortController();
  const broker = new PermissionBroker();

  broker.setEventSink((ev) => {
    for (const line of bridgeEventToClaudeOutputLines(ev)) {
      process.stdout.write(`${line}\n`);
    }
  });

  const controlLoop = (async () => {
    for (;;) {
      const n = await it.next();
      if (n.done) break;
      const line = String(n.value);
      if (line.trim() === "") continue;
      let msg: unknown;
      try {
        msg = JSON.parse(line);
      } catch {
        continue;
      }
      if (isAbortMessage(msg)) {
        ac.abort();
        continue;
      }
      if (isPermissionResponse(msg)) {
        broker.respond(msg.requestId, msg.allowed);
      }
    }
  })();

  const opts: BridgeQueryOptions = {
    prompt: q.prompt,
    workingDirectory: q.workingDirectory,
    model: q.model,
    sessionId: q.sessionId,
    permissionMode: q.permissionMode ?? "default",
    abortController: ac,
    oauthToken: q.oauthToken,
    canUseTool: (toolName, toolInput) => broker.request(toolName, toolInput),
  };

  try {
    for await (const ev of queryClaudeCode(opts)) {
      for (const line of bridgeEventToClaudeOutputLines(ev)) {
        process.stdout.write(`${line}\n`);
      }
    }
  } finally {
    broker.denyAll();
    rl.close();
    await controlLoop.catch(() => undefined);
  }

  process.stdout.write(`${JSON.stringify({ done: true })}\n`);
}

run().catch((e) => {
  const msg = e instanceof Error ? e.message : String(e);
  process.stderr.write(`sidecar: fatal: ${msg}\n`);
  process.exit(1);
});
