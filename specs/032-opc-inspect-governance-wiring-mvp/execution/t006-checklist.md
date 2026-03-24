# T006 working contract — gitctx sidecar + `@opc/mcp-client` (PR-4 gate)

**Feature:** `032-opc-inspect-governance-wiring-mvp`  
**Task:** T006 — Complete frontend MCP/sidecar client path used by the git context panel  
**Status:** This document is the **PR-4 Definition of Done**. Implementation and review should satisfy it; it is not informal chat guidance.

**Related:** `tasks.md` (T006), `plan.md`, `execution/verification.md`

### PR-4: Chosen decisions (copy into PR description)

Reviewers use this to avoid mid-PR drift. **Paste into the PR body** and fill before or during review; keep it aligned with the code.

```markdown
### Chosen decisions (T006 / PR-4)

- **Transport owner:** A (TS → localhost MCP) **or** B (Rust proxy only) — *fill*
- **gitctx transport mode:** *fill — must match the real listener (e.g. Streamable HTTP path on `{port}`)*
- **Spawn policy:** *fill — when `spawn_gitctx` runs (e.g. startup vs first use)*
- **Reconnect policy:** *fill — e.g. poll `getSidecarPorts`, backoff, cancel in-flight on port change*
```

---

## Why this exists

- **Transport reality today does not match the advertised sidecar contract.** Tauri expects `gitctx-mcp` to print `OPC_GITCTX_PORT=<port>` and implies a listener on that port; the current `gitctx-mcp` binary (`crates/gitctx/src/mcp_main.rs`) serves **stdio MCP only**. Finishing `@opc/mcp-client` against stdio while the app expects a port is a **fake** completion.
- This file locks **reviewable acceptance criteria** for a seam-heavy integration slice and prevents “misc MCP fixes” from substituting for an end-to-end contract.

---

## Lock before any implementation starts

Decisions **must** be written down (e.g. ADR snippet in PR description or comments on the chosen modules). **Do not merge half of each pattern.**

### 1. Transport ownership

Choose **exactly one**:

| Option | Meaning |
|--------|---------|
| **A — TS owns transport** | TypeScript (`@opc/mcp-client`) connects to `127.0.0.1:{port}` using the MCP transport that matches what `gitctx-mcp` exposes on that port (e.g. Streamable HTTP / SSE — **must match server**). |
| **B — Rust owns transport** | Tauri proxies MCP; the webview calls **only** Tauri commands. `@opc/mcp-client` is a thin typed wrapper around those commands, **not** a second HTTP client to the sidecar. |

**Recommendation:** Prefer **A** if the official MCP SDK path is clean in the Tauri webview bundle; otherwise prefer **B** as the **single** transport owner. **Forbidden at PR-4 merge:** TS opening localhost to the sidecar *and* Rust implementing parallel proxy commands without one path being clearly deprecated.

### 2. gitctx sidecar contract (Rust binary)

The spawned sidecar must satisfy **both**:

1. Emit on stdout a line **`OPC_GITCTX_PORT=<u16>`** (contract already documented in `apps/desktop/src-tauri/src/sidecars.rs`).
2. Run an **actual listener** on that port using the **same** MCP-compatible transport chosen in (1).

Until (1) and (2) are true, `@opc/mcp-client` cannot be validated against a real contract.

---

## PR-4 implementation order (recommended)

1. Make **gitctx** satisfy the advertised port + listener contract (see locks above).
2. Wire **`spawn_gitctx`** from `lib.rs` (or equivalent single entry) so ports can become non-null when the app runs.
3. Complete **`@opc/mcp-client`** against that **real** contract (typed API, errors, lifecycle).
4. Add a **small desktop hook** that consumes `commands.getSidecarPorts()` / `get_sidecar_ports` and connects the client when `gitctx` is available.
5. Layer **additive sidecar enrichment** on top of the **already-shipped** native git panel (PR-3); do not replace local git as authority for core state (see below).
6. Keep **governance** and unrelated MCP surfaces **out of scope** (see Non-goals).

---

## Semantic rule: native git is source of truth (PR-3 preserved)

**Authoritative in T006 for “core repo state” (must not be semantically undermined by PR-4):**

- Branch (or detached HEAD handling)
- Dirty / clean (working tree)
- Ahead / behind (with existing degraded semantics when upstream missing)

**Sidecar contributes only enrichment**, for example:

- Remote / upstream metadata not covered by local commands
- GitHub / user / org context
- Anything **beyond** the current local snapshot that the native panel already shows

If sidecar data conflicts with local git, **local wins**; UI may show sidecar as unavailable or partial, not as a second truth for branch/dirty/ahead-behind.

---

## Acceptance criteria (PR-4 merge)

### Package and workspace

1. **`@opc/mcp-client`** exposes a real public API: e.g. factory returning `connect`, `disconnect`, `readResource`, `callTool`, and optionally `listTools` — not a stub with only `initialize()`.
2. **`apps/desktop`** declares a `workspace:*` dependency on `@opc/mcp-client` and uses it through that API (no scattered ad-hoc MCP calls in UI components).
3. **`@modelcontextprotocol/sdk` (or chosen stack)** is **pinned** in `packages/mcp-client/package.json` — not unbounded `latest`.

### Ports and lifecycle

4. **Source of truth for port:** `SidecarPorts.gitctx` from `get_sidecar_ports` / `commands.getSidecarPorts()` — no duplicate hard-coded ports in the frontend.
5. **Semantics documented in code or PR:** meaning of `gitctx: null` (not announced yet vs failed); stale-port handling (retry, cancel in-flight, reconnect).
6. **`spawn_gitctx`** is **invoked** at a defined lifecycle point with explicit failure behavior.
7. **Connect / reconnect:** wait for port when needed; on port change or process death, cancel in-flight work and reconnect cleanly.

### Errors and UI

8. **Typed errors:** e.g. sidecar not ready, stale port, transport failure, MCP RPC error, parse error, timeout — mapped to safe UI copy.
9. **Timeouts** on MCP round-trips used by the git panel path.
10. **`unavailable` vs `degraded`:** sidecar unreachable must **not** force the whole panel into error if native git already succeeded; show enrichment as unavailable or degraded per rules above.
11. **Request/response typing:** at minimum `readResource` for `gitctx://context/current` (see `crates/gitctx/src/mcp/resources.rs`) with TS types or Zod; any tools used must be named with typed args/results.

### Tests / verification

12. **Tests or harness:** mock `getSidecarPorts` — `null`, then valid port, then stale port (e.g. connection refused after success); partial MCP payload → degraded, not crash.
13. **Record** T006 verification steps in `execution/verification.md` when implementing.

### Spec hygiene

14. **`tasks.md` paths:** keep file references accurate (`GitContextPanel` vs `features/git/*` as in repo).

---

## Non-goals (explicitly out of scope for T006 / PR-4)

- **Governance panel** integration or wiring `@opc/mcp-client` for governance flows.
- **Axiomregent** MCP completion or port wiring (ports may exist in `SidecarPorts` but T006 does not require axiomregent behavior).
- **Generic multi-server MCP abstraction** beyond what **gitctx** needs for the git context journey.
- **Event-driven sidecar health** if **simple polling** of `getSidecarPorts()` (and reconnect logic) is sufficient for this slice — do not add a health event system “for free.”

---

## Repo facts snapshot (for reviewers)

- `packages/mcp-client/src/index.ts` is currently a **skeleton** (stdio-oriented; notes browser incompatibility).
- `apps/desktop` **does not** yet depend on `@opc/mcp-client`.
- `gitctx-mcp` **stdio-only** in `mcp_main.rs` vs Tauri **port line** expectation in `sidecars.rs`.
- `spawn_gitctx` / `spawn_axiomregent` are **defined** but were **not** wired from `lib.rs` at the time this checklist was written.
- Tauri `commands/mcp.rs` contains **placeholder** `mcp_list_tools` / `mcp_call_tool` / `mcp_read_resource` — resolve overlap with transport choice (1) so two bridges do not half-exist.

---

## PR-4 gate

**Merge PR-4 when:** the locks above are decided and implemented, the acceptance criteria are met, non-goals are respected, and verification is recorded. **Do not merge** if `@opc/mcp-client` is “complete” while `gitctx-mcp` still only speaks stdio and never honors `OPC_GITCTX_PORT` + listener.
