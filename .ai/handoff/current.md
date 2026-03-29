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

- Current owner: **claude-opus**
- Next owner: **cursor** (for P0 spec implementation) or **claude-opus** (for next-slice prioritization among 042–063)
- Last baton update: 2026-03-29 — **claude-opus**: Stagecraft-ing extraction + integration complete. 65 new files. 22 draft specs (042–063). All 17 source projects safe to delete. Awaiting direction on which spec to implement first.
- Recommended files to read:
  - `docs/extractions/CONSOLIDATION.md` — master priority list with cross-cutting themes
  - `CLAUDE.md` — new project governance
  - `AGENTS.md` — agent protocol and available commands
  - Any spec in `specs/042-*` through `specs/063-*` for implementation candidates

## Requested next agent output

**Prioritization needed.** 22 draft specs (042–063) are ready. Recommended implementation order based on cross-cutting validation (7+ projects confirmed the pattern):

1. **045 — Claude Code SDK Bridge** (unblocks desktop app agent execution)
2. **042 — Multi-Provider Agent Registry** (unblocks multi-backend support)
3. **044 — Multi-Agent Orchestration** (unblocks sophisticated agent workflows)
4. **046 — Context Compaction** (unblocks long sessions)
5. **047 — Governance Control Plane** (extends spec-compiler with runtime enforcement)

Each P0 spec has full interface definitions, architecture, and success criteria ready for implementation.

## P2 items captured as ideas only (not yet specs)

20 P2 items from the consolidation (plugin marketplace, three-tier model strategy, encrypted keychain, WebSocket reconnection, i18n, quick pane, NDJSON streaming, Bun compilation, pain-to-pattern methodology, agent communication bus, FI/FDI guardrails, SQLite schema, AI session naming, security scanning MCP, markdown tool responses, VS Code extension, slash command fuzzy search, design tokens, scheduling system). See `docs/extractions/CONSOLIDATION.md` § P2 for details.

## Promotion candidates for canonical artifacts

- All 22 new specs already live in `specs/` (canonical)
- All commands/agents/rules already live in `.claude/` (canonical)
- Code modules already live in `apps/desktop/src/lib/` (canonical)
- Extraction reports in `docs/extractions/` are reference material (non-canonical, deletable after source projects are removed)

---

## Recent outputs

- 2026-03-29 (claude-opus): Stagecraft-ing full extraction + integration. 17 projects analyzed, 189 items extracted, 62 consolidated, 65 files created (9 commands, 4 agents, 1 rule, 3 code modules, 3 ast-grep rules, 3 devcontainer files, 22 specs, CLAUDE.md, AGENTS.md). All source projects confirmed safe to delete.
- 2026-03-29 (claude-opus): Slice H complete. V-005 message wording fixed. All residuals cleared.
- 2026-03-29 (claude-opus): Post-041 synthesis complete. Authority-map, next-slice, integration-debt all updated for 032–041.
