# Verification: agent governed execution

**Feature**: `035-agent-governed-execution`

## Commands

Run from repository root where applicable.

```bash
cd crates/axiomregent && cargo test
cd crates/agent && cargo test
cd apps/desktop/src-tauri && cargo check
cd apps/desktop && pnpm run check
```

## Evidence

| Criterion | Result |
|-----------|--------|
| axiomregent unit + integration tests | `cargo test` in `crates/axiomregent` — green (2026-03-29) |
| agent crate | `cargo test` in `crates/agent` — green |
| Desktop typecheck | `pnpm run check` in `apps/desktop` — green |
| T001 spike | `.ai/findings/035-mcp-spike.md` — MCP via `--mcp-config` + stdio binary; probe port not MCP transport |

### NF-001 (latency)

Governed path adds MCP subprocess startup and JSON-RPC handling; no automated p99 gate in-repo — manual profiling recommended when hardening.
