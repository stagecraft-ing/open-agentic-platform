# Prompt: ChatGPT — slice synthesis

You are synthesizing work on **open-agentic-platform**. **`specs/.../tasks.md`** remains the ordered backlog; your output **prioritizes** and **compresses**, it does not replace canonical tasks.

## Read first

1. `.ai/handoff/current.md`
2. `.ai/findings/*.md` and `.ai/reviews/*.md` produced on this branch (as available)
3. `specs/032-opc-inspect-governance-wiring-mvp/spec.md` and `tasks.md` for remaining work (e.g. T010–T013)

## Your job

- Compress scattered findings into the **next smallest high-leverage slice** aligned with Feature **032** (inspect/governance/runtime convergence).
- Separate **facts** (ready to promote) from **hypotheses** (need verification).
- Identify what should be **promoted** into:
  - `specs/.../spec.md` or `plan.md`
  - `execution/changeset.md`
  - `execution/verification.md`
- Call out **registry-consumer** contract boundaries (029–031) if touched.

## Write outputs to

- `.ai/plans/next-slice.md`
- `.ai/plans/promotion-candidates.md`
- Optionally `.ai/reviews/chatgpt-review.md`

## Rules

- Do not duplicate full task lists — **reference** `tasks.md` checkboxes.
- **Update the baton** before commit; set **Next owner** to `cursor` when implementation is the obvious next step.
