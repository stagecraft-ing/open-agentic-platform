# AGENTS.md — Open Agentic Platform

## New Sessions

Run `/init` as the mandatory first action of every new session. The command reads this section to derive its execution plan dynamically — any item added here is automatically picked up on the next init.

**Init protocol (executed by `/init`):**

0. **Load rules** — read `.claude/rules/orchestrator-rules.md`
1. **Parallel reads** (dispatch simultaneously):
   - `CLAUDE.md` — project overview and conventions
   - `README.md` — full project description
   - `git log --oneline -10` — recent history
   - `git diff --stat HEAD~1` — last change summary
2. **Emit** `## initialized: open-agentic-platform` summary block (layer overview, recent activity, ready to help with)

If any file is missing: log "not found" and continue.

## Available Agents

Agents live in `.claude/agents/`. Four pipeline agents handle the plan/explore/implement/review cycle:

- `architect` — Plans and decomposes tasks, validates approaches against specs. Read-only.
- `explorer` — Searches the codebase, traces dependencies, gathers context. Read-only.
- `implementer` — Executes focused code changes from an existing plan. Produces minimal diffs.
- `reviewer` — Post-change review for bugs, security, performance, and spec compliance. Read-only.

## Available Commands

Commands live in `.claude/commands/`. Currently none are registered — add commands here as the project grows.

<!-- Example entry:
- `/compile` — Build the spec registry and report any errors
-->

## Conventions

- Items added to the "New Sessions" init protocol are auto-loaded by `/init`.
- Agents must be self-contained within `.claude/agents/` — no cross-project dependencies (Rule 5).
- Commands must produce output files for downstream steps — no context-window-only state (Rule 2).
