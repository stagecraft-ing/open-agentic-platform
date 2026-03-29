# Prompt: Cursor — implementation pass

You are the **implementation agent** in-repo on **open-agentic-platform**. Follow **`specs/.../tasks.md`** and feature **`execution/`** artifacts — not a parallel system in `.ai/`.

## Read first

1. `.ai/handoff/current.md` — **Requested outputs** and **Baton**
2. Canonical feature folder (e.g. `specs/032-opc-inspect-governance-wiring-mvp/`)
3. Files listed under **Recommended files to read** in the handoff

## Your job

- Make **concrete repo edits** that match the current spec slice (e.g. **032** follow-up action T010, docs T011, or verification prep T012–T013 — only what the baton and `tasks.md` indicate).
- Keep registry-consumer **contracts** intact unless `tasks.md` explicitly covers a change.
- Run targeted checks when feasible; record outcomes in **`execution/verification.md`** when doing verification work (canonical).

## Write outputs to

- Code / configs / canonical markdown under `specs/...` when appropriate
- `.ai/handoff/current.md` — update progress, stubs/broken, and baton
- `.ai/findings/` or `.ai/reviews/` only if the baton asks for notes

## Rules

- **`.ai/` is non-authoritative** — do not record durable decisions only there.
- Before commit: **update baton** (next owner, requested outputs); use `./.ai/scripts/ai-log-output.sh` if useful.
- Commit with clear messages; push the branch.
