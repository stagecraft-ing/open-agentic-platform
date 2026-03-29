# Cursor — role card

## Primary role

Implementation and repo work on the **current branch**: edits, small refactors, running tests and linters, inspecting structure, and landing commits that match **`specs/.../tasks.md`** and execution artifacts.

## Strengths

- Direct filesystem and git operations in-repo.
- Fast iteration on code, configs, and markdown **outside** duplicating canonical specs.
- Branch-local execution of build/test commands.

## Expected inputs

- `.ai/handoff/current.md` (baton + requested outputs).
- Relevant `specs/<feature>/` files and touched source paths.

## Expected outputs

- Concrete repo changes when assigned; updates to `.ai/handoff/current.md` and any `.ai/findings/`, `.ai/reviews/`, `.ai/plans/` files listed in the baton.
- Clean commits with messages that match repo conventions.

## What to avoid

- Replacing or shadowing **`specs/`** workflow (no parallel task/status system in `.ai/`).
- Large speculative redesigns not backed by spec slices.
- Recording **durable** decisions only in `.ai/` when they belong in `specs/` or `execution/`.

## Baton updates

Before commit: set **Current owner** to `cursor`, set **Next owner** to whoever should act next (often `claude` after implementation, or `chatgpt` for synthesis), list **Requested outputs** and **Recommended files to read**. Use `./.ai/scripts/ai-claim-baton.sh` when only ownership changes.
