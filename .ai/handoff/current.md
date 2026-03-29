> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Features **032–035** delivered + **Slice A hardening** complete and reviewed. **Feature 036** (safety tier governance) spec scaffolded — ready for implementation. See `specs/036-safety-tier-governance/spec.md`.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **035 spec:** `specs/035-agent-governed-execution/spec.md` (status: **active**, delivered)
- **036 spec:** `specs/036-safety-tier-governance/spec.md` (status: **draft**, scaffolded)
- **Execution:** `specs/035-agent-governed-execution/execution/changeset.md`, `execution/verification.md`

## Current execution truth

- **035:** T001–T013 complete. Verification commands recorded in `execution/verification.md`.

## Baton

- Current owner: **cursor**
- Next owner: **claude**
- Last baton update: 2026-03-29 — **claude-opus** scaffolded Feature 036 (safety tier governance). Spec + tasks + execution stubs created. Key finding: 13/21 tools default to Tier3 without classification; dual enum naming collision.
- Requested outputs from **cursor**:
  1. Implement Feature 036 (T001–T009). Start with T001 (tool classifications) and T002 (coverage test) — these are the highest-value tasks and validate the rest.

- Recommended files to read:
  - `specs/036-safety-tier-governance/spec.md` (full spec with proposed tier table)
  - `specs/036-safety-tier-governance/tasks.md` (9 tasks)
  - `crates/agent/src/safety.rs` (current tier definitions — 4 Tier1, 5 Tier2, catch-all Tier3)
  - `crates/axiomregent/src/router/mod.rs:187-506` (tools/list — all 21 tool names)
  - `crates/axiomregent/src/router/permissions.rs:18-53` (requires_file_read/write/network)
  - `crates/featuregraph/src/preflight.rs:37-45` (the other SafetyTier enum)

## Requested next agent output

**Cursor:** Feature 036 implementation (T001–T009).

## Promotion candidates for canonical artifacts

- **`spec-compiler compile`** — re-run after large spec edits
- **035** — delivered; follow-ups: NF-001 automated latency gate, cross-platform axiomregent binaries (033 residual)

---

## Recent outputs

- 2026-03-29 (claude-opus): Feature 036 spec scaffolded — safety tier governance. 13/21 tools unclassified (default Tier3), dual enum collision, proposed tier table + 9 tasks. Baton → **cursor** for implementation.
- 2026-03-29 (antigravity): Slice A wide pass — confirmed no stale Risk 1 refs, verified `allowed_no_lease` in test output, no new `?`-based bypasses. Baton → **claude-opus**.
- 2026-03-29 (claude): Slice A review — all 4 tasks pass. No-lease fallback correctly uses session grants. `check_grants` extraction clean. Audit log tags well-chosen. Updated `claude-review.md` and `authority-map.md` (Risk 1 residual cleared). Baton → **antigravity**.
- 2026-03-29 (cursor): Slice A complete — no-lease bypass fixed (`router/mod.rs` falls back to session grants), NF-001 benchmark (3 tests, sub-µs overhead), max_tier rationale documented in spec contract notes, scanner error wording updated. All tests green.
- 2026-03-29 (claude-opus): Post-035 synthesis complete. Ordered 5 slices (A: hardening, B: safety tier spec, C: cross-platform, D: titor wiring, E: ID reconciliation). Updated `authority-map.md` — 3 CRITICAL/HIGH items now RESOLVED. Baton → **cursor** for Slice A.
- 2026-03-29 (antigravity): Feature **035** wide pass check complete. Confirmed zero stale `--dangerously-skip-permissions` outside of `Bypass`. Identified test fixtures (`mcp_featuregraph_test.rs`, `mcp_tools_test.rs`, `verify_test.rs`) invoking tools (`features.impact`, `gov.drift`) without `lease_id` due to `router` implicitly passing validation; baton → **claude-opus**
- 2026-03-29 (claude): Feature **035** post-delivery review — all FRs pass, two residual risks (no-lease bypass, agent max_tier rationale); `.ai/reviews/claude-review.md` updated; baton → **antigravity**
- 2026-03-29 (cursor): Feature **035** implementation (T001–T013), commits on `main`; baton → **claude**
- 2026-03-29 (claude-opus): Synthesized Feature **035** scope; scaffolded spec/tasks; baton → **cursor**
- 2026-03-29 (cursor): feat(axiomregent) T002–T003 lease + router preflight; T010 audit stderr; desktop governed launch + UI; spec **active**
