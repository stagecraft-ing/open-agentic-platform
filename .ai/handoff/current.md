> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Feature **035 agent governed execution** implemented and verified (tasks T001–T013 checked; `specs/035-agent-governed-execution/spec.md` **active**). **Next:** **Claude** post-implementation review and any **Antigravity** wide pass if baton warrants.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **035 spec:** `specs/035-agent-governed-execution/spec.md` (status: **active**, delivered)
- **Execution:** `specs/035-agent-governed-execution/execution/changeset.md`, `execution/verification.md`

## Current execution truth

- **035:** T001–T013 complete. Verification commands recorded in `execution/verification.md`.

## Baton

- Current owner: **antigravity**
- Next owner: **claude-opus** (synthesis / next-slice prioritization)
- Last baton update: 2026-03-29 — **claude** completed 035 post-delivery review (all FRs pass, two residual risks documented)
- Requested outputs from **antigravity**:
  1. Wide repo pass — verify no stale `--dangerously-skip-permissions` references remain outside `governed_claude.rs::Bypass`.
  2. Check for any call sites or test fixtures that invoke axiomregent tools without `lease_id` (relates to Risk 1 in review).
  3. Optionally scan for any other governance-adjacent gaps surfaced by 035 changes.
- Recommended files to read:
  - `.ai/reviews/claude-review.md` (035 section — risks and promotion candidates)
  - `crates/axiomregent/src/router/permissions.rs` (no-lease bypass path)
  - `apps/desktop/src-tauri/src/governed_claude.rs` (MCP subprocess pattern)

## Requested next agent output

**Antigravity:** wide exploration pass for 035 residuals; then hand to **claude-opus** for synthesis.

## Promotion candidates for canonical artifacts

- **`spec-compiler compile`** — re-run after large spec edits
- **035** — delivered; follow-ups: NF-001 automated latency gate, cross-platform axiomregent binaries (033 residual)

---

## Recent outputs

- 2026-03-29 (claude): Feature **035** post-delivery review — all FRs pass, two residual risks (no-lease bypass, agent max_tier rationale); `.ai/reviews/claude-review.md` updated; baton → **antigravity**
- 2026-03-29 (cursor): Feature **035** implementation (T001–T013), commits on `main`; baton → **claude**
- 2026-03-29 (claude-opus): Synthesized Feature **035** scope; scaffolded spec/tasks; baton → **cursor**
- 2026-03-29 (cursor): feat(axiomregent) T002–T003 lease + router preflight; T010 audit stderr; desktop governed launch + UI; spec **active**
