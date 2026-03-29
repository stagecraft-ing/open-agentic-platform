# Prompt: Claude — architecture review

You are assisting on **open-agentic-platform** with a **spec-first** workflow. Canonical product truth lives in **`specs/...`** (not in `.ai/`).

## Read first

1. `.ai/handoff/current.md`
2. The active feature folder cited there (e.g. `specs/032-opc-inspect-governance-wiring-mvp/`: `spec.md`, `plan.md`, `tasks.md`)
3. `execution/changeset.md` for that feature

## Your job

Produce an architecture assessment that focuses on:

- **Contradictions** between spec intent and likely code structure (cite paths).
- **Authority mismatches**: who owns git truth, governance/registry truth, and UI display — enforced vs advisory.
- **Control-plane truth**: what is actually wired vs described for OPC inspect, sidecar, MCP clients, and desktop shell.
- **Governance**: what is **enforced** (gates, contracts) vs **shown** (panels, banners).

## Write outputs to

- `.ai/findings/authority-map.md` and/or `.ai/reviews/claude-review.md`
- Update `.ai/findings/open-questions.md` if you surface blockers

## Rules

- **`.ai/` is non-authoritative** — label inferences; separate verified vs suspected.
- List **promotion candidates** for `spec.md` clarifications or `execution/changeset.md` updates.
- **Update the baton** in `.ai/handoff/current.md` before commit (see `.ai/prompts/baton-rules.md`).

## Out of scope

- Replacing `tasks.md` with a new backlog in `.ai/`.
