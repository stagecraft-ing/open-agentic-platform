# `.ai/` — auxiliary multi-agent handoff workspace

**Non-authoritative.** This directory is a **collaboration bus** for Cursor, Claude, Claude Opus (synthesis), and Antigravity. It does **not** replace or compete with canonical product truth.

## What belongs elsewhere (canonical)

- **`specs/...`** — feature specs, plans, tasks, and per-feature execution artifacts (`execution/changeset.md`, `execution/verification.md`, checklists, runbooks).
- **Code and tests** — implementation truth.
- **PRs and official execution records** — review and merge history.

Do **not** duplicate canonical specs, task lists, or official status here. Durable decisions and facts belong in those artifacts after promotion.

## What `.ai/` is for

Temporary cross-tool coordination: handoff notes, synthesis drafts, compressed reviews, analysis staging, prompt snippets, and **git-based baton passing**. Use it to avoid chat copy-paste; promote outcomes into canonical locations when they stabilize.

## Operating model

1. Each agent reads **`.ai/handoff/current.md`** first.
2. Each agent writes only the outputs requested in the baton (findings, reviews, plans, code — as assigned).
3. Before commit, each agent **updates the baton** (owner, next owner, requested outputs, recommended reads).
4. Each agent **commits and pushes** branch-local changes so the next agent can `git pull`.
5. Next agent pulls and continues the loop.

Helper scripts live in **`.ai/scripts/`** (see names below). They are optional conveniences; the files in `.ai/` are the source of truth for handoff text.

## Scripts (optional)

| Script | Purpose |
|--------|---------|
| `ai-handoff-status.sh` | Branch, git status, baton summary, `.ai/` changes, preview of `current.md` |
| `ai-handoff-next.sh` | Branch, latest commit, diff vs `origin/main`, inventory of findings/reviews/plans, baton |
| `ai-claim-baton.sh` | Set current/next baton owner and timestamp in `current.md` |
| `ai-log-output.sh` | Record which tool wrote which file under “Recent outputs” |
| `ai-promote-checklist.sh` | Reminder checklist for promoting durable items into canonical artifacts |

---

**Reminder:** If something must be true for the product going forward, **promote it** into `specs/...`, execution artifacts, or code — not only into `.ai/`.
