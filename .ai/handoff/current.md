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

- Current owner: **claude**
- Next owner: **antigravity** (optional wide pass) or **claude-opus** (synthesis)
- Last baton update: 2026-03-29 — **cursor** completed Feature **035** (governed dispatch, UI permissions, audit stderr, execution docs)
- Requested outputs from **claude**:
  1. Runtime/architecture review vs `specs/035-agent-governed-execution/spec.md` — confirm FRs and residual risks (especially MCP subprocess per session, `opc-web` sidecar stub).
  2. Update `.ai/reviews/claude-review.md` with a **035** section and file-backed citations.
- Recommended files to read:
  - `apps/desktop/src-tauri/src/governed_claude.rs`
  - `crates/axiomregent/src/router/permissions.rs`
  - `specs/035-agent-governed-execution/execution/verification.md`

## Requested next agent output

**Claude:** post-delivery review for **035**; then hand to **antigravity** or **claude-opus** per loop.

## Promotion candidates for canonical artifacts

- **`spec-compiler compile`** — re-run after large spec edits
- **035** — delivered; follow-ups: NF-001 automated latency gate, cross-platform axiomregent binaries (033 residual)

---

## Recent outputs

- 2026-03-29 (cursor): Feature **035** implementation (T001–T013), commits on `main`; baton → **claude**
- 2026-03-29 (claude-opus): Synthesized Feature **035** scope; scaffolded spec/tasks; baton → **cursor**
- 2026-03-29 (cursor): feat(axiomregent) T002–T003 lease + router preflight; T010 audit stderr; desktop governed launch + UI; spec **active**
