> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Features **032–035** delivered + **Slice A hardening** complete (no-lease bypass fixed, NF-001 benchmark added, contract docs updated). **Next:** Claude review of Slice A, then **Slice B** (safety tier governance spec). See `.ai/plans/next-slice.md` for full prioritization.

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
- Next owner: **antigravity**
- Last baton update: 2026-03-29 — **cursor** completed Slice A (post-035 hardening). All 4 tasks implemented: no-lease bypass fix, max_tier contract docs, NF-001 benchmark (3 tests), scanner wording fix. All axiomregent + featuregraph tests green.
- Requested outputs from **claude**:
  1. Review Slice A implementation — verify no-lease fallback is correct, check `check_grants` factoring, confirm audit log format.

- Recommended files to read:
  - `crates/axiomregent/src/router/mod.rs:112-170` (refactored `preflight_tool_permission`)
  - `crates/axiomregent/src/router/permissions.rs` (new `check_grants` + refactored `check_tool_permission`)
  - `crates/axiomregent/src/snapshot/lease.rs:193-195` (new `default_grants()` accessor)
  - `crates/axiomregent/tests/governed_dispatch_latency.rs` (NF-001 benchmark)
  - `specs/035-agent-governed-execution/spec.md` (updated contract notes)
  - `crates/featuregraph/src/scanner.rs:274-276` (wording fix)

## Requested next agent output

**Claude:** Review Slice A implementation for correctness and completeness.

## Promotion candidates for canonical artifacts

- **`spec-compiler compile`** — re-run after large spec edits
- **035** — delivered; follow-ups: NF-001 automated latency gate, cross-platform axiomregent binaries (033 residual)

---

## Recent outputs

- 2026-03-29 (cursor): Slice A complete — no-lease bypass fixed (`router/mod.rs` falls back to session grants), NF-001 benchmark (3 tests, sub-µs overhead), max_tier rationale documented in spec contract notes, scanner error wording updated. All tests green.
- 2026-03-29 (claude-opus): Post-035 synthesis complete. Ordered 5 slices (A: hardening, B: safety tier spec, C: cross-platform, D: titor wiring, E: ID reconciliation). Updated `authority-map.md` — 3 CRITICAL/HIGH items now RESOLVED. Baton → **cursor** for Slice A.
- 2026-03-29 (antigravity): Feature **035** wide pass check complete. Confirmed zero stale `--dangerously-skip-permissions` outside of `Bypass`. Identified test fixtures (`mcp_featuregraph_test.rs`, `mcp_tools_test.rs`, `verify_test.rs`) invoking tools (`features.impact`, `gov.drift`) without `lease_id` due to `router` implicitly passing validation; baton → **claude-opus**
- 2026-03-29 (claude): Feature **035** post-delivery review — all FRs pass, two residual risks (no-lease bypass, agent max_tier rationale); `.ai/reviews/claude-review.md` updated; baton → **antigravity**
- 2026-03-29 (cursor): Feature **035** implementation (T001–T013), commits on `main`; baton → **claude**
- 2026-03-29 (claude-opus): Synthesized Feature **035** scope; scaffolded spec/tasks; baton → **cursor**
- 2026-03-29 (cursor): feat(axiomregent) T002–T003 lease + router preflight; T010 audit stderr; desktop governed launch + UI; spec **active**
