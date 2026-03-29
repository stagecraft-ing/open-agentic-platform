# Feature 035 — T001 spike: axiomregent as MCP server for Claude Code

## Question

Can Claude Code CLI use axiomregent’s **probe port** (`OPC_AXIOMREGENT_PORT`) as an `--mcp-server` target so tool calls route through governed dispatch?

## Answer: no — the probe port is not MCP transport

- **axiomregent** speaks MCP over **stdio** (Content-Length framed JSON-RPC on stdin/stdout). See `crates/axiomregent/src/main.rs`: stdout is reserved for MCP frames; stderr prints `OPC_AXIOMREGENT_PORT=<port>` for **desktop liveness only**.
- The TCP port is a **probe listener** (accept loop holds the port open); it does **not** serve MCP over HTTP/SSE. Pointing Claude’s MCP client at `http://127.0.0.1:<port>` is not a supported integration.

## Viable wiring pattern (implemented in 035)

1. **Governed session** — When Claude Code spawns a **separate** axiomregent process as an MCP **stdio** server, that process loads permission grants from **`OPC_GOVERNANCE_GRANTS`** (JSON) at startup; `LeaseStore::issue` applies those defaults to new leases. Tool dispatch in `Router` enforces tier + flags before handler execution.

2. **Claude CLI flags** (verified against local `claude --help`, 2026-03):
   - **`--mcp-config <json-or-file>`** — supply an MCP config whose `mcpServers.<name>.command` is the **absolute path** to the bundled `axiomregent` binary (same artifact as the desktop sidecar), with optional **`env.OPC_GOVERNANCE_GRANTS`** for that subprocess.
   - Omit **`--dangerously-skip-permissions`** in governed mode; use **`--permission-mode default`** so permission flow is not globally bypassed.
   - The desktop **sidecar** remains useful as a **readiness** signal (`get_sidecar_ports`); governed execution does **not** reuse its stdio — Claude owns a dedicated MCP child.

3. **Fallback** — If the sidecar has not announced a port (or binary resolution fails), fall back to the previous behavior: **`--dangerously-skip-permissions`** and emit a **`governance-mode` / bypass** signal to the UI.

## Risk R-001 (spec)

Using MCP stdio subprocess per session is supported by Claude Code’s `--mcp-config`; the residual risk is behavioral differences across CLI versions — mitigated by pinning documented flags and verification commands in `execution/verification.md`.
