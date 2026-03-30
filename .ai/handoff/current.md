> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

**Stagecraft-ing extraction complete.** 17 projects from `~/Dev2/stagecraft-ing/` analyzed exhaustively and all valuable content integrated into OAP or captured as draft specs. The platform now has a full governance foundation (CLAUDE.md, AGENTS.md, orchestrator rules), 9 slash commands, 4 agent definitions, 3 code modules, ast-grep enforcement rules, a devcontainer, and 22 new spec outlines (042–063) covering the next generation of platform capabilities.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **032–041:** All `status: active`, all complete, all synthesized.
- **042–063:** All `status: draft`, newly created from stagecraft-ing extraction. See below for details.

## What was delivered in this session

### Phase 1: Extraction (17 projects → 18 reports)

All projects in `~/Dev2/stagecraft-ing/` were analyzed file-by-file. Extraction reports live in `docs/extractions/`. Master consolidation: `docs/extractions/CONSOLIDATION.md` (189 raw items deduplicated to 62: 14 P0, 28 P1, 20 P2).

**Source projects (all confirmed safe to delete):** agents, asterisk-mcp-server, claude-code, claude-code-by-agents, claude-code-single-binary, claude-code-sub-agents, claudecodeui, claudepal, crystal, deepreasoning, developer-claude-code-commands, equilateral-agents-open-core, gitctx (already byte-identical to OAP), product-manager-claude-code-commands, research (empty), ruflo, skills.

### Phase 2: Integration (65 new files)

**Governance foundation:**
- `CLAUDE.md` — project-level Claude Code configuration with architecture overview, conventions
- `AGENTS.md` — self-extending agent protocol with init checklist, available agents/commands
- `.claude/rules/orchestrator-rules.md` — 6-rule governance preamble for multi-step workflows

**Slash commands (`.claude/commands/`):**
| Command | What it does | Source projects |
|---------|-------------|-----------------|
| `code-review.md` | Parallel 6-agent review with adaptive selection | developer-cc-commands, skills |
| `validate-and-fix.md` | Phased quality-gate pipeline with rollback | developer-cc-commands, claudepal |
| `init.md` | Session init protocol (self-extending via AGENTS.md) | claudepal |
| `commit.md` | Governed conventional commits, impact-focused | skills, developer-cc-commands |
| `cleanup.md` | Delegated dead-code/duplicate analyzer | claudepal |
| `review-branch.md` | Exhaustive read-only branch review (10 cross-platform categories) | developer-cc-commands |
| `refactor-claude-md.md` | CLAUDE.md modularization with path-scoped rules | skills |
| `implement-plan.md` | Plan file executor with status state machine | skills |
| `research.md` | Parallel sub-agent research with query classification | skills, product-manager-cc-commands |

**Agent definitions (`.claude/agents/`):**
| Agent | Role | Model |
|-------|------|-------|
| `architect.md` | Read-only planning, task decomposition | sonnet |
| `explorer.md` | Codebase analysis, context gathering | sonnet |
| `implementer.md` | Focused code changes from plans | sonnet |
| `reviewer.md` | Post-change review (bugs, security, perf) | sonnet |

**Code modules (`apps/desktop/src/lib/`):**
- `censor.ts` — Secret censoring (Anthropic keys, OpenAI, GitHub, AWS, PEM, etc.)
- `shellPath.ts` — Cross-platform PATH resolution for Tauri (with Rust backend invoke fallback)
- `shellEscape.ts` — Safe shell argument escaping and git commit building

**ast-grep rules (`apps/desktop/.ast-grep/`):**
- `zustand/no-destructure.yml` — prevents render cascades
- `architecture/hooks-in-hooks-dir.yml` — enforces hooks boundary
- `architecture/no-store-in-lib.yml` — forces getState() in lib/

**DevContainer (`.devcontainer/`):**
- `Dockerfile` — Node 20 + Rust + pnpm + git-delta + Claude Code
- `devcontainer.json` — VS Code config with rust-analyzer
- `init-firewall.sh` — iptables sandbox (GitHub, npm, crates.io, Anthropic only)

### New specs (042–063)

| Spec | Title | Priority | Action |
|------|-------|----------|--------|
| 042 | Multi-Provider Agent Registry | P0 | outline-spec |
| 043 | Agent Organizer / Meta-Orchestrator | P0 | outline-spec |
| 044 | Multi-Agent Orchestration (file-based artifacts) | P0 | outline-spec |
| 045 | Claude Code SDK Bridge | P0 | outline-spec |
| 046 | Context Compaction | P0 | outline-spec |
| 047 | Governance Control Plane (policy compiler) | P0 | outline-spec |
| 048 | Hookify Rule Engine | P1 | outline-spec |
| 049 | Permission System | P1 | outline-spec |
| 050 | Tool Renderer System | P1 | outline-spec |
| 051 | Worktree Agents | P1 | outline-spec |
| 052 | State Persistence | P1 | outline-spec |
| 053 | Verification Profiles | P1 | outline-spec |
| 054 | Agent Frontmatter Schema | P1 | outline-spec |
| 055 | YAML Standards Schema | P1 | outline-spec |
| 056 | Session Memory | P1 | outline-spec |
| 057 | Notification System | P1 | outline-spec |
| 058 | File Mention System | P1 | outline-spec |
| 059 | Git Panel | P1 | outline-spec |
| 060 | Panel Event Bus | P1 | outline-spec |
| 061 | Conductor Track Lifecycle | P1 | outline-spec |
| 062 | Multi-Model Chaining | P1 | outline-spec |
| 063 | Coherence Scoring | P1 | outline-spec |

## Baton

- Current owner: **claude** (042 Phase 4 reviewed ✅)
- Next owner: **cursor** — **042 Phase 5**: Gemini + Bedrock adapters, or **Phase 6** (integration wiring into agent execution) if Phase 5 can be deferred. SC-001 already satisfied (3 adapters: Anthropic, OpenAI, Claude Code SDK).
- Last baton update: 2026-03-29 — **claude**: Phase 4 review complete. 13/13 tests pass. SC-001 satisfied (two+ adapters). P4-002 (vision content blocks stringified, LOW). Review: `.ai/findings/042-phase4-review.md`.
- Recommended files to read:
  - `.ai/findings/042-phase4-review.md` — Phase 4 review
  - `specs/042-multi-provider-agent-registry/spec.md` — Phases 5–6

## Requested next agent output

**042 Phase 5 or 6** — Gemini + Bedrock adapters (Phase 5), or integration wiring (Phase 6). Phase 4 reviewed and approved. SC-001 satisfied.

Priority order for P0 specs (unchanged):

1. **045 — Claude Code SDK Bridge** — ✅ `status: active` — end-to-end complete
2. **042 — Multi-Provider Agent Registry** — Phase 1–4 ✅ reviewed; SC-001 satisfied; Phase 5–6 remaining
3. **044 — Multi-Agent Orchestration**
4. **046 — Context Compaction**
5. **047 — Governance Control Plane**

Land **042** phases per `specs/042-multi-provider-agent-registry/spec.md`; after each slice, **claude** reviews against `spec.md`.

## P2 items captured as ideas only (not yet specs)

20 P2 items from the consolidation (plugin marketplace, three-tier model strategy, encrypted keychain, WebSocket reconnection, i18n, quick pane, NDJSON streaming, Bun compilation, pain-to-pattern methodology, agent communication bus, FI/FDI guardrails, SQLite schema, AI session naming, security scanning MCP, markdown tool responses, VS Code extension, slash command fuzzy search, design tokens, scheduling system). See `docs/extractions/CONSOLIDATION.md` § P2 for details.

## Promotion candidates for canonical artifacts

- All 22 new specs already live in `specs/` (canonical)
- All commands/agents/rules already live in `.claude/` (canonical)
- Code modules already live in `apps/desktop/src/lib/` (canonical)
- Extraction reports in `docs/extractions/` are reference material (non-canonical, deletable after source projects are removed)

---

## Recent outputs

- 2026-03-29 (claude): **042 Phase 4 review** — OpenAI adapter mirrors Anthropic pattern. 13/13 tests pass. SC-001 satisfied (3 adapters). P4-002 (vision content stringified, LOW), P4-003 (mergeAbortSignals dup, COSMETIC). Review: `.ai/findings/042-phase4-review.md`. Baton → cursor for Phase 5/6.
- 2026-03-29 (cursor): **042 Phase 4** — OpenAI Chat Completions: `createOpenAIProvider`, `OpenAIStreamNormalizer`, `completionToAgentEvents`, vitest. SC-001 (two adapters) satisfied by Anthropic + OpenAI.
- 2026-03-29 (review): **042 Phase 3 review** — `.ai/findings/042-phase3-review.md`. FR-002/FR-003/FR-007 satisfied; FR-006/SC-006 vs optional API key documented LOW; P3-002/003 INFO. Next: Phase 4 OpenAI adapter.
- 2026-03-29 (cursor): **042 Phase 3** — Claude Code SDK provider: `queryClaudeCode()` + `ClaudeCodeBridgeNormalizer`, session resume map, abort forwarding, `ambient-claude-code-sdk.d.ts` for tsc with linked bridge. Tests: `claude-code-events.test.ts`. 10/10 vitest pass.
- 2026-03-29 (claude): **042 Phase 2 review** — Adapter implements all 4 Provider ops, normalizer handles text/tool/thinking. 7/7 tests pass. P2-001 (input tokens 0 in streaming) + P2-002 (no text_complete in streaming), both LOW. Review: `.ai/findings/042-phase2-review.md`. Baton → cursor for Phase 3.
- 2026-03-29 (cursor): **042 Phase 2** — Anthropic Messages provider (`@anthropic-ai/sdk`), `AnthropicStreamNormalizer`, `messageToAgentEvents`, exports + tests.
- 2026-03-29 (claude): **042 Phase 1 review** — All types match spec byte-for-byte. FR-001–FR-007 covered. 6/6 tests pass. No issues. Review: `.ai/findings/042-phase1-review.md`. Baton → cursor for Phase 2 (Anthropic adapter).
- 2026-03-29 (cursor): **042 Phase 1** — New package `@opc/provider-registry` (`InMemoryProviderRegistry`, types, vitest).
- 2026-03-29 (claude): **045 → active** — Confirmed `ClaudeCodeSession` uses `executeClaudeBridge` for new + resumed prompts, no legacy CLI paths remain. Promoted spec to `status: active`. 045 complete. Baton → cursor for 042.
- 2026-03-29 (cursor): **045 session UI → bridge** — `ClaudeCodeSession` calls `executeClaudeBridge` for new and resumed prompts (replaces CLI `execute` / `resume`).
- 2026-03-29 (claude): **045 final confirmation** — F-011 verified (abort JSON before stdin drop). API bindings verified (`executeClaudeBridge`, `respondToBridgePermission`, web adapter routes). All 9 FRs pass, all HIGH/MEDIUM findings resolved. 045 feature-complete, ready for `status: active` on UI wiring. Full tracker: `.ai/findings/045-final-status.md`. Baton → cursor for session UI + 042.
- 2026-03-29 (cursor): **F-011 + api.ts** — Abort JSON on bridge stdin before close; `executeClaudeBridge` / `respondToBridgePermission` on desktop API; `apiAdapter` REST mappings for web fallback.
- 2026-03-29 (claude): **045 sidecar review** — All 9 FRs verified ✅. FR-004 PermissionBroker now fully wired (F-003 resolved). New finding F-011: `cancel_claude_execution` drops stdin without sending abort JSON (MEDIUM). F-012–F-014 cosmetic. Architecture matches IPC plan. Full review: `.ai/findings/045-sidecar-review.md`. Baton → cursor for F-011 + frontend api.ts.
- 2026-03-29 (cursor): **045 Tauri ↔ Node sidecar** — `sidecar.ts`, `claude-output-lines.ts`, `execute_claude_bridge` / `respond_to_bridge_permission`, `ClaudeBridgeIpcState`, `spawn_claude_bridge_process`; desktop re-exports mapper from `@opc/claude-code-bridge/claude-output-lines`. Requires `dist/sidecar.js` (run `tsc` in bridge package). Next: claude review; frontend API wiring.
- 2026-03-29 (claude): **045 post-F001/F002 review** — Verified F-001, F-002, F-005, F-006 all resolved. F-003 (PermissionBroker) confirmed still dead code but deferred to sidecar slice. New findings: F-008 (start event drop OK by design), F-009 (stale closure risk in error branch, LOW), F-010 (index signature masks type errors, blocked on consumer audit). Spec parity: 9/9 FR done, FR-004 partial. Full review: `.ai/findings/045-post-f001f002-review.md`. Next: cursor implements sidecar.
- 2026-03-29 (cursor): **045 F-001 + F-002** — `useClaudeMessages` parity with SDK/stream-json; `cli-adapter` duplicate `session-complete` eliminated. Types consolidated via `AgentExecution` `ClaudeStreamMessage`. Next: Tauri/Node sidecar + F-003 optional.
- 2026-03-29 (claude): **045 review** — 7 findings in `.ai/findings/045-bridge-parity-gaps.md`. Critical: useClaudeMessages checks wrong message types (F-001), CLI adapter double session-complete (F-002), PermissionBroker dead code (F-003). Produced sidecar IPC plan in `.ai/findings/045-tauri-node-ipc-plan.md` recommending Node sidecar with stdin/stdout JSONL protocol. Next: fix F-001/F-002, then implement sidecar.
- 2026-03-29 (cursor): 045 integration slice — `@opc/claude-code-bridge` wired into `apps/desktop` (workspace dep); `bridgeEventToClaudeOutputLines()` maps bridge events to JSONL strings for existing `claude-output` consumers; `packages/claude-code-bridge` exports `./types`, ambient SDK declaration, `cli-adapter` import cleanup. Tests: `apps/desktop/src/lib/bridgeEventToClaudeOutput.test.ts`. Next: Tauri/Node bridge process + permission round-trip.
- 2026-03-29 (claude): First vertical slice for 045 landed — `packages/claude-code-bridge/` (5 files). Implements: typed `queryClaudeCode()` async generator (FR-001), `BridgeQueryOptions` (FR-002), session resumption (FR-003), `canUseTool` permission broker (FR-004), permission mode fallback (FR-005), AbortController cancellation (FR-006), cost tracking from SDKResultMessage (FR-007), CLI fallback when SDK absent (FR-008), discriminated union BridgeEvent (FR-009). Next: Tauri backend + frontend integration.
- 2026-03-29 (claude-opus): Stagecraft-ing full extraction + integration. 17 projects analyzed, 189 items extracted, 62 consolidated, 65 files created (9 commands, 4 agents, 1 rule, 3 code modules, 3 ast-grep rules, 3 devcontainer files, 22 specs, CLAUDE.md, AGENTS.md). All source projects confirmed safe to delete.
- 2026-03-29 (claude-opus): Slice H complete. V-005 message wording fixed. All residuals cleared.
- 2026-03-29 (claude-opus): Post-041 synthesis complete. Authority-map, next-slice, integration-debt all updated for 032–041.
