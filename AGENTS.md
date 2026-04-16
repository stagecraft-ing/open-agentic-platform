# AGENTS.md — Open Agentic Platform

## New Sessions

Run `/init` as the mandatory first action of every new session. The command reads this section to derive its execution plan dynamically — any item added here is automatically picked up on the next init.

**Init protocol (executed by `/init`):**

0. **Load rules** — read `.claude/rules/orchestrator-rules.md` AND `.claude/rules/governed-artifact-reads.md`.
1. **Parallel reads** (dispatch simultaneously):
   - `CLAUDE.md` — project overview and conventions
   - `README.md` — full project description
   - `codebase-indexer check` — staleness gate for the structural index (non-fatal)
   - `codebase-indexer render` → `build/codebase-index/CODEBASE-INDEX.md` — rendered structural summary (run render only if the markdown is missing)
   - `registry-consumer status-report --json --nonzero-only` — lifecycle counts per spec status
   - `registry-consumer list --ids-only` — spec id list (for latest-spec detection)
   - `git log --oneline -10` — recent history
   - `git diff --stat HEAD~1` — last change summary
2. **Emit** `## initialized: open-agentic-platform` summary block (layer overview, recent activity, ready to help with).

**Read discipline (spec 103):** the init protocol MUST NOT parse `build/**/*.json` directly (no `python`, `jq`, `awk`, `sed` against compiled artifacts). All structural and lifecycle data comes from the consumer binaries and the rendered markdown view.

**Staleness surface:** if `codebase-indexer check` exits non-zero, include `Structural index: stale — run `codebase-indexer compile`` in the summary and continue. If `CODEBASE-INDEX.md` is missing and `render` fails (no `index.json`), report `Structural index: not built` and continue without structural counts.

**Binary missing:** if a consumer binary is not built, instruct the user to `cargo build --release --manifest-path tools/<name>/Cargo.toml` and continue — do NOT fall back to ad-hoc parsing.

If any file is missing: log "not found" and continue.

## Available Agents

Agents live in `.claude/agents/`. Four pipeline agents handle the plan/explore/implement/review cycle, plus a domain specialist:

- `architect` — Plans and decomposes tasks, validates approaches against specs. Read-only.
- `explorer` — Searches the codebase, traces dependencies, gathers context. Read-only.
- `implementer` — Executes focused code changes from an existing plan. Produces minimal diffs.
- `reviewer` — Post-change review for bugs, security, performance, and spec compliance. Read-only.
- `encore-expert` — Encore.ts framework specialist for stagecraft service development. Read-only.

## Available Commands

Commands live in `.claude/commands/`:

- `/init` — Initialize a session (load context, recent activity, memory)
- `/commit` — Create a git commit with impact-focused conventional message
- `/code-review` — Multi-aspect code review using parallel sub-agents
- `/review-branch` — Review all changes in the current branch
- `/implement-plan` — Execute a plan file step-by-step with progress tracking
- `/research` — Deep research with parallel sub-agents and query classification
- `/validate-and-fix` — Run quality checks and automatically fix issues
- `/cleanup` — Dead code and duplicate detection with categorized recommendations
- `/refactor-claude-md` — Modularize large CLAUDE.md files with path-scoped rules
- `/factory-sync` — Detect and translate upstream factory changes into OAP

## Conventions

- Items added to the "New Sessions" init protocol are auto-loaded by `/init`.
- Agents must be self-contained within `.claude/agents/` — no cross-project dependencies (Rule 5).
- Commands must produce output files for downstream steps — no context-window-only state (Rule 2).
- Orchestrated workflows must read compiled artifacts (`build/**`) through consumer binaries, never via ad-hoc parsers — see `.claude/rules/governed-artifact-reads.md` (spec 103).
