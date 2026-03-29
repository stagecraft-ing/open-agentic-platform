# Prompt: Antigravity — deep cross-cutting analysis

You are doing **broad repo exploration** on **open-agentic-platform** — adjacent opportunities, hidden coupling, and leverage — **without** becoming a second product owner. Scope and commitments remain in **`specs/`**.

## Read first

1. `.ai/handoff/current.md`
2. `specs/032-opc-inspect-governance-wiring-mvp/spec.md` (scope boundaries)
3. Outputs from Claude (if any) in `.ai/findings/`

## Your job

Search and reason across the repo for:

- **Under-expressed strengths**: crates, packages, or flows that could support inspect/governance with small wiring.
- **Hidden leverage**: one change that unlocks multiple surfaces (desktop, MCP, registry).
- **Cross-cutting risks**: drift between UI, sidecar, and compiled registry outputs.
- **Alternative implementation paths** that stay within **032**-style narrow slices.
- Items **not yet** discussed in spec or `.ai/` — mark as new signals.

## Write outputs to

- `.ai/findings/under-integrated-assets.md`
- `.ai/findings/integration-risks.md` (if not redundant)
- `.ai/reviews/antigravity-review.md` for a tight summary

## Rules

- **Source-grounded** references; flag speculation.
- **Non-authoritative** — list promotion candidates; do not rewrite `tasks.md` wholesale.
- **Update the baton** before commit; hand to `claude-opus` for synthesis or `cursor` for a thin implementation PR.
