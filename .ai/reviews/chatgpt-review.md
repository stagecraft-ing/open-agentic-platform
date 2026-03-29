# ChatGPT review (working notes)

> **Non-authoritative.** Synthesis and prioritization only; does not replace `tasks.md`.

## Scope reviewed

- **Inputs:** `.ai/handoff/current.md`, `.ai/plans/next-slice.md`, `.ai/plans/integration-debt.md`, `.ai/plans/promotion-candidates.md`, `.ai/reviews/claude-review.md`, `.ai/findings/authority-map.md`, `specs/032-opc-inspect-governance-wiring-mvp/execution/verification.md`
- **Code spot-check:** `apps/desktop/src-tauri/src/commands/analysis.rs` (`read_registry_summary` / `featureSummaries`), `apps/desktop/src/features/inspect/RegistrySpecFollowUp.tsx` — matches runtime-path notes

## Main concerns

1. **Display vs enforcement:** Governance and registry UIs are honest; **execution** still bypasses permission and tier machinery (`--dangerously-skip-permissions` — see `.ai/reviews/claude-review.md`). This is the core post-032 product gap, not a Feature 032 defect.
2. **featuregraph scanner:** Still oriented around missing/forbidden `spec/features.yaml`; **registry.json** path would improve the featuregraph half without blocking **View spec** (which uses **`featureSummaries`** from the registry half).
3. **axiomregent:** Sidecar stack exists but is not started; activation is the highest-leverage convergence move per `integration-debt.md` (suggested order **1st**).

## What appears resolved

- **Feature 032** delivery: inspect → git → governance display → **View spec** follow-up; verification recorded in canonical `execution/verification.md` (2026-03-28).
- **Registry authority** for follow-up actions: compiler-emitted `registry.json` → `featureSummaries` → UI; no client-side parsing of the full registry file.

## What still blocks convergence

- **Platform thesis** (governed execution, not just governance visibility): requires post-032 specs and implementation — axiomregent activation, optional agent routing, scanner adaptation, titor wiring, safety-tier spec, ID reconciliation (see `integration-debt.md`).

## Recommended next move

**For ChatGPT synthesis:** Compare two forks:

| Fork | Pros | Cons |
|------|------|------|
| **A — Feature 033: axiomregent activation first** (`plans/next-slice.md`) | Unlocks governed tool surface; aligns with debt order #1; one spec can bound scope (spawn + expose tools, not full agent reroute). | Binary bundling / ports / CI must be validated before promising dates. |
| **B — featuregraph scanner fix first** | Improves governance panel “full” vs degraded independent of axiomregent; user-visible win. | Does not address execution bypass; second ordering dimension. |

**Cursor recommendation:** Prefer **A** for narrative coherence, run **B** in parallel if capacity allows (orthogonal codepaths). Produce a **single ordered backlog** for promotion into `specs/` (033+), not duplicate `tasks.md` line-by-line.

## Promotion candidates

- [ ] `specs/032-opc-inspect-governance-wiring-mvp/spec.md` — optional frontmatter `status: implemented` if Product agrees (Feature 003 lifecycle); `tasks.md` already marks complete
- [ ] New `specs/033-*` (or numbered) — axiomregent activation per `next-slice.md`
- [ ] Future spec — scanner/registry alignment for featuregraph; safety tiers; feature ID bridge
