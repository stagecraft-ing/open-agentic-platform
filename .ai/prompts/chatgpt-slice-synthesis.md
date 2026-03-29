# Prompt: Claude Opus — slice synthesis

> Replaced ChatGPT synthesis prompt (2026-03-29). Now targets Claude Opus for consistency with deep-analysis agent.

You are synthesizing work on **open-agentic-platform**. **`specs/.../tasks.md`** remains the ordered backlog; your output **prioritizes** and **compresses**, it does not replace canonical tasks.

## Read first

1. `.ai/handoff/current.md`
2. `.ai/findings/*.md` and `.ai/reviews/*.md` produced on this branch (as available)
3. Active feature `specs/<NNN>/spec.md` and `tasks.md` for remaining work

## Your job

- Compress scattered findings into the **next smallest high-leverage slice**.
- Separate **facts** (ready to promote) from **hypotheses** (need verification).
- Identify what should be **promoted** into canonical artifacts.
- Call out **registry-consumer** contract boundaries (029–031) if touched.
- Respect registry enum values: `draft|active|superseded|retired` only (Feature 000/003). There is no `implemented` status.

## Write outputs to

- `.ai/plans/next-slice.md`
- `.ai/plans/promotion-candidates.md`
- Optionally `.ai/reviews/claude-synthesis.md`

## Rules

- Do not duplicate full task lists — **reference** `tasks.md` checkboxes.
- **Update the baton** before commit; set **Next owner** to `cursor` when implementation is the obvious next step.
- Do not invent frontmatter values. Delivery is proven by checked tasks + verification artifacts, not by status changes.
