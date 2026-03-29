> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Features **032–039** delivered: Feature **039** (codeAliases / Feature ID reconciliation) implemented — ADR 0001 accepted, schema **1.1.0**, compiler + scanner + frontmatter + verification artifacts.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **037 spec:** `specs/037-cross-platform-axiomregent/spec.md` (status: **active**, complete — T003/T004 deferred to CI)
- **038 spec:** `specs/038-titor-tauri-command-wiring/spec.md` (status: **active**, complete — all tasks done, round-trip verified)
- **039 spec:** `specs/039-feature-id-reconciliation/spec.md` (status: **active**, complete — T001–T009 done; `execution/verification.md`)

## Current execution truth

- **037:** All local tasks complete (T001/T002/T005/T006/T007/T008/T009). T003/T004 deferred to CI runners. Registry compiled.
- **038:** Complete — `TitorState`, all 6 commands wired, round-trip verified (init→checkpoint→list→verify→diff→restore). Reviewed by claude, wide-passed by antigravity (race condition fixed). Registry compiled.
- **039:** Complete — ADR 0001 accepted; `registry.schema.json` + spec-compiler `code_aliases` / V-005 / V-006 / `specVersion` 1.1.0; featuregraph `codeAliases` → `FeatureEntry.aliases`; frontmatter on specs **004, 005, 032–035**; verification recorded; golden graph updated. `build/spec-registry/registry.json` remains gitignored — run `spec-compiler compile` locally/CI.

## Baton

- Current owner: **cursor**
- Next owner: **claude**
- Last baton update: 2026-03-29 — **cursor** Implemented Feature 039 (T001–T009): ADR gap closure, schema+compiler+scanner+frontmatter, `execution/verification.md`, golden refresh.
- Requested outputs from **claude**:
  1. Review Feature 039 delivery (spec vs code), residual header hygiene (`spec/verification.yaml` Spec lines; optional follow-up).
- Recommended files to read: `specs/039-feature-id-reconciliation/execution/verification.md`, `docs/adr/0001-feature-id-reconciliation.md`, `tools/spec-compiler/src/lib.rs`, `crates/featuregraph/src/registry_source.rs`

## Requested next agent output

**claude:** Review Feature 039 implementation and update `.ai/reviews/claude-review.md` if findings warrant.

## Promotion candidates for canonical artifacts

(All promoted)

---

## Recent outputs

- 2026-03-29 (cursor): Feature 039 implemented — codeAliases pipeline (ADR, schema 1.1.0, compiler V-005/V-006, scanner, frontmatter, verification, golden). Baton → **claude**.
- 2026-03-29 (claude-opus): Post-ADR synthesis. Scaffolded Feature 039 (`specs/039-feature-id-reconciliation/` — 9 tasks). ADR edits bundled as T001. Updated next-slice, integration-debt, authority-map. Baton → **cursor**.
- 2026-03-29 (claude): ADR 0001 review — decision sound, 4 gaps found (schema bump, validation codes, scanner consumer contract, population ordering). Updated `claude-review.md`. Baton → **claude-opus**.
- 2026-03-29 (cursor): Slice E — ADR 0001 drafted (`docs/adr/0001-feature-id-reconciliation.md`): kebab canonical `id`, optional `codeAliases` in registry (option a). Updated integration-debt, baton. Baton → **claude**.
- 2026-03-29 (claude-opus): Post-038 synthesis complete. All authority-map HIGH items resolved (032–038). Updated authority-map, integration-debt, next-slice. Next slice: Feature ID reconciliation (Slice E — ADR needed). Baton → **cursor**.
- 2026-03-29 (antigravity): Feature 038 wide pass complete. Fixed `get_or_init` race in `titor.rs` by holding the write lock during both insertions. Baton → **claude-opus**.
- 2026-03-29 (claude): Feature 038 review — all FRs/SCs pass. One low-severity `get_or_init` race noted. `cargo check` + tests green. Updated `claude-review.md`. Baton → **antigravity**.
- 2026-03-29 (cursor): Feature 038 — `TitorState`, wired `titor_init` and five commands, `lib.rs` manage, spec `active`, tasks T001–T011, verification + spec-compiler. Baton → **claude**.
- 2026-03-29 (claude-opus): Post-037 synthesis. Closed T009 (spec-compiler). Scaffolded Feature 038 (titor Tauri command wiring — 11 tasks). Updated next-slice, integration-debt, authority-map. Baton → **cursor**.
- 2026-03-29 (antigravity): Wide pass on Feature 037. Fixed doc comment, added CI cache, fixed timeout, verified .exe tracking. Promoted canonical artifacts. Baton → **claude-opus**.
- 2026-03-29 (claude): Feature 037 review — all FRs pass (FR-001 partial). ToolTier fix correct, 1 stale doc comment in `lease.rs:97`. CI smoke test `timeout` won't work on macOS (continue-on-error masks it). T003/T004 deferral correct. Updated `claude-review.md`. Baton → **antigravity** for wide pass.
- 2026-03-29 (cursor): Feature 037 implemented — Windows x86_64 binary built (7.3 MB), MCP handshake verified, all 21 tools confirmed. Created build script and CI workflow. Fixed stale `Tier`→`ToolTier` in `agent.rs`. All tests green. Baton → **claude** for review.
- 2026-03-29 (claude-opus): Post-036 synthesis complete. Scaffolded Feature 037. Updated authority-map, next-slice, integration-debt. Baton → **cursor**.
- 2026-03-29 (antigravity): Feature 036 wide pass complete. Baton → **claude-opus**.
- 2026-03-29 (claude): Feature 036 review — all FRs/SCs pass. Baton → **antigravity**.
- 2026-03-29 (cursor): Feature 036 implemented (T001–T007). Baton → **claude**.
- 2026-03-29 (claude-opus): Feature 036 spec scaffolded. Baton → **cursor**.
- 2026-03-29 (antigravity): Slice A wide pass. Baton → **claude-opus**.
- 2026-03-29 (claude): Slice A review. Baton → **antigravity**.
- 2026-03-29 (cursor): Slice A complete. Baton → **claude**.
- 2026-03-29 (claude-opus): Post-035 synthesis. Baton → **cursor**.
