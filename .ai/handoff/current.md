> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

**Feature 032 closure:** T010–T013 are **done** on `main` (view-spec follow-up, docs, vitest, verification recorded). **Claude** should re-read canonical execution artifacts and `.ai/` plans to refresh post-032 analysis, update `.ai/findings/` if runtime/authority notes drifted, and hand to ChatGPT or Cursor per next priorities.

## Canonical feature authority

- **Spec:** `specs/032-opc-inspect-governance-wiring-mvp/spec.md`
- **Tasks / status:** `specs/032-opc-inspect-governance-wiring-mvp/tasks.md` (T000–T013 complete)
- **Plan:** `specs/032-opc-inspect-governance-wiring-mvp/plan.md`
- **Execution:** `specs/032-opc-inspect-governance-wiring-mvp/execution/changeset.md`, `execution/verification.md`

## Current execution truth

- **T010:** Registry `featureSummaries` (id, title, specPath) emitted from `read_registry_summary` / `featuregraph_overview`; **View spec** buttons on Xray (`InspectSurface` after successful scan + governance fetch) and **Governance** surfaces; opens spec in `MarkdownEditor` via `claude-md` tab with `specMarkdownAbsolutePath`.
- **T011:** `apps/desktop/README.md` + root `README.md` pointer; `execution/changeset.md` updated.
- **T012:** Vitest (`pnpm -C apps/desktop test`): `inspect-actions.test.ts`, `RegistrySpecFollowUp.test.tsx`.
- **T013:** Commands recorded in `execution/verification.md` (2026-03-28): desktop build/test, `cargo check` Tauri, `commands::analysis::tests`, registry-consumer tests, spec-compiler compile.

## Files touched (implementation summary)

- `apps/desktop/src-tauri/src/commands/analysis.rs` — `featureSummaries` in registry summary
- `apps/desktop/src/features/inspect/{actions.ts,RegistrySpecFollowUp.tsx,InspectSurface.tsx}`
- `apps/desktop/src/features/governance/GovernanceSurface.tsx`
- `apps/desktop/src/components/{MarkdownEditor.tsx,TabContent.tsx}`
- `apps/desktop/src/contexts/TabContext.tsx`, `hooks/useTabState.ts`, `services/tabPersistence.ts`
- `apps/desktop/{package.json,vitest.config.ts,src/test/setup.ts,README.md}`
- `specs/032-opc-inspect-governance-wiring-mvp/{tasks.md,execution/changeset.md,execution/verification.md}`
- `README.md` (repo root)

## What is working

- Inspect success path can show registry-linked **View spec** actions when `build/spec-registry/registry.json` exists for the scanned repo.
- Governance tab shows the same follow-up when compiled registry is **ok**.
- Markdown editor can load/save arbitrary absolute spec paths (Tauri `read_claude_md_file` / `save_claude_md_file`).

## What is stubbed / broken

- **featuregraph** may remain unavailable without `spec/features.yaml` — still an expected bounded degraded path (see `verification.md`).
- **Post-032** stubs (titor, blockoli, axiomregent) unchanged — out of scope for 032.

## Decisions made

- Extended backend summary with **`featureSummaries`** so the UI does not parse full `registry.json` client-side.
- **Singleton `claude-md`** tab remains for system prompt; spec files use additional `claude-md` tabs distinguished by `specMarkdownAbsolutePath`.

## Open questions

- Should **spec-lint** be added to CI verification once CLI invocation is standardized? (Current `spec-lint` invocation in `.ai` notes used wrong subcommand — verify with `spec-lint --help` before documenting.)
- Any **UX** follow-up: cap/limit number of “View spec” buttons (currently first 24) or add search?

## Baton

- Current owner: cursor
- Next owner: claude
- Last baton update: 2026-03-28 — T010–T013 implemented on `main`; baton to Claude for post-032 deep pass / `.ai` refresh
- Requested outputs:
  - Reconcile `.ai/findings/runtime-path.md` and `.ai/findings/authority-map.md` with new `featureSummaries` + spec tab path if needed
  - Update `.ai/plans/promotion-candidates.md` with post-032 items (optional)
  - Short update to `.ai/reviews/claude-review.md` if gaps found
- Recommended files to read:
  - `specs/032-opc-inspect-governance-wiring-mvp/execution/verification.md`
  - `specs/032-opc-inspect-governance-wiring-mvp/execution/changeset.md`
  - `apps/desktop/src/features/inspect/actions.ts`, `RegistrySpecFollowUp.tsx`
  - `apps/desktop/src-tauri/src/commands/analysis.rs`

## Requested next agent output

Claude: confirm analysis docs still match code; refresh `.ai/findings/` as needed; run **`./.ai/scripts/ai-claim-baton.sh claude chatgpt`** when synthesis is the next step, or hand to **cursor** for follow-up fixes. Commit and push `.ai/` updates if you change them.

## Promotion candidates for canonical artifacts

- Post-032 integration debt items → new feature specs / plans (already listed in `.ai/plans/integration-debt.md` — validate against repo)

---

## Recent outputs

- 2026-03-28 (claude): `.ai/findings/runtime-path.md`, `.ai/findings/authority-map.md`, `.ai/findings/under-integrated-assets.md`, `.ai/findings/open-questions.md`, `.ai/findings/integration-risks.md`, `.ai/reviews/claude-review.md`, `.ai/plans/next-slice.md`, `.ai/plans/promotion-candidates.md`, `.ai/plans/integration-debt.md`
- 2026-03-28 (cursor): T010–T013 implementation + canonical `specs/032-.../execution/*` updates on `main`
