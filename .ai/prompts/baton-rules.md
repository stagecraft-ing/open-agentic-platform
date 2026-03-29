# Baton rules — operating contract

> **Non-authoritative.** One-page rules for handoff between Cursor, Claude, Claude Opus (synthesis), and Antigravity.

## Before you start

1. `git pull` on the **current branch** you are asked to use.
2. Read **`.ai/handoff/current.md`** end-to-end.
3. Read the **canonical** feature folder under `specs/...` referenced there (especially `spec.md`, `tasks.md`, `execution/changeset.md`).

## While you work

- Write outputs only where the baton **Requested outputs** say (typically `.ai/findings/`, `.ai/reviews/`, `.ai/plans/`, or code).
- Treat **`.ai/` as scratch + coordination** — not product authority.
- Cite **files and symbols** when claiming behavior.

## Before you commit

1. Update **`## Baton`** in `.ai/handoff/current.md`:
   - **Current owner:** your tool name (`cursor` | `claude` | `claude-opus` | `antigravity`)
   - **Next owner:** who should act next
   - **Requested outputs:** concrete files the next agent should produce or update
   - **Recommended files to read:** minimal, high-signal list
2. Optionally run `./.ai/scripts/ai-claim-baton.sh <you> <next>` if only ownership changes.
3. Optionally run `./.ai/scripts/ai-log-output.sh <tool> <path>` when adding a notable artifact.

## After you commit

- Push your branch so the next agent does not stall on stale state.

## Promotion

If you reach a **durable** conclusion, do not stop at `.ai/` — note it under **Promotion candidates** and/or move it into `specs/...`, `execution/changeset.md`, `execution/verification.md`, or code.

## Loop (default narrative)

`cursor` → implement / adjust → `claude` → deep analysis → `antigravity` → wide pass → `claude-opus` → synthesize next slice → `cursor` again (flex as needed).
