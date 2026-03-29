> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Keep **Feature 032** (OPC inspect + governance wiring MVP) moving toward completion on `main`: close remaining slices (follow-up action, docs, verification) without diluting spec-first governance. Use this file to pass baton between tools; **promote** durable outcomes into `specs/032-opc-inspect-governance-wiring-mvp/` and its `execution/` artifacts.

## Canonical feature authority

- **Spec:** `specs/032-opc-inspect-governance-wiring-mvp/spec.md`
- **Tasks / status:** `specs/032-opc-inspect-governance-wiring-mvp/tasks.md`
- **Plan:** `specs/032-opc-inspect-governance-wiring-mvp/plan.md`
- **Execution:** `specs/032-opc-inspect-governance-wiring-mvp/execution/changeset.md`, `execution/verification.md`

Do not treat this handoff file as a second task system — follow **`tasks.md`** for ordered work.

## Current execution truth

- Feature 032 is **in progress** per `tasks.md`: inspect, git, and governance wiring slices are largely done; **T010–T013** (action path, docs, targeted/full verification) remain open.
- Registry-consumer contract work (**029–031**) is prerequisite substrate; avoid weakening settled consumer contracts while wiring OPC inspect.
- This workspace (`.ai/`) was bootstrapped to support **repo-local baton passing** — it does not change product scope.

## Files touched

- `.ai/**` — new auxiliary handoff workspace only (this commit).

## What is working

- Spec-first layout with per-feature `spec.md`, `tasks.md`, `plan.md`, and `execution/` artifacts.
- Feature 032 describes an end-to-end inspect journey with native git authority and governance panels backed by real paths (per tasks and merged PRs referenced in `tasks.md`).

## What is stubbed / broken

- **Not claiming broken code here** — verify in repo and in `execution/verification.md` when doing T012/T013.
- Remaining product gaps for 032 are explicitly **T010** (bounded follow-up action), **T011** (docs), and verification tasks — see canonical `tasks.md`.

## Decisions made

- Adopt `.ai/` as **non-authoritative** collaboration surface; baton defaults to **cursor → claude** after bootstrap so Claude can deepen runtime/authority analysis before synthesis (ChatGPT) and broader repo passes (Antigravity).

## Open questions

- What is the smallest **T010** action that satisfies FR-005 without scope creep?
- Any **runtime vs displayed governance** gaps left between compiled registry output and UI?
- What verification commands must be recorded in `execution/verification.md` for green baseline on `main`?

## Baton

- Current owner: cursor
- Next owner: claude
- Last baton update: 2026-03-28 — workspace bootstrap; baton passed to Claude for deep reads
- Requested outputs:
  - Update `.ai/findings/runtime-path.md` with source-grounded notes on inspect → panels → sidecar/MCP paths (verified vs inferred).
  - Update `.ai/findings/authority-map.md` with enforced vs displayed governance and where truth lives (files + brief bullets).
  - Optional: short notes in `.ai/reviews/claude-review.md` if review-style framing helps the next agent.
- Recommended files to read:
  - `.ai/handoff/current.md` (this file)
  - `specs/032-opc-inspect-governance-wiring-mvp/spec.md`, `tasks.md`, `plan.md`
  - `specs/032-opc-inspect-governance-wiring-mvp/execution/changeset.md`, `execution/verification.md`
  - Recent touched app paths under `apps/desktop` / Tauri as referenced in `tasks.md` and changesets

## Requested next agent output

Claude: produce the two findings updates above (runtime path + authority map), tighten open questions if contradictions appear, then run **`./.ai/scripts/ai-claim-baton.sh claude antigravity`** (or hand to **chatgpt** if the next step is synthesis only), commit, and push.

## Promotion candidates for canonical artifacts

- Nothing from this bootstrap file requires promotion until agents produce **verified** findings; then merge factual runtime/authority conclusions into `execution/changeset.md` or spec addenda as appropriate.

---

## Recent outputs

- (none yet)
