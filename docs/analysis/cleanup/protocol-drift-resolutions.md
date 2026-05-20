# Protocol Drift Resolutions (D-2.1 through D-2.11)

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** Read of `docs/analysis/init-trace.md` §S5 (drift table) + §S6 + §Open decisions, cross-referenced with current `AGENTS.md`, `.claude/commands/init.md`, `CLAUDE.md`, `.claude/rules/governed-artifact-reads.md`. Reconciled against post-cleanup paths from D1–D8.

## Single-source-of-truth principle

After Epic 2 Phase I10:

- **`AGENTS.md` "New Sessions"** is the canonical init protocol. Every Step belongs here.
- **`.claude/commands/init.md`** is a thin executor that defers to AGENTS.md — it parses the "New Sessions" hook (already documented as "self-extending") and executes the items listed there. Its body is reduced to the executor mechanics + the summary template.
- **`CLAUDE.md`** references rule conventions and the orchestrator behavioral rules but does not duplicate the init protocol. It points readers at AGENTS.md for the init protocol.
- **`.claude/rules/governed-artifact-reads.md`** is the consumer-binary table only. It does not duplicate any init step.

D-2.* resolutions converge on this principle: AGENTS.md is the protocol; init.md is the executor; CLAUDE.md is the conventions; the rules files are the constraints.

## D-2.1 — Rules pre-load divergence

**Drift:** Three different prescriptions for which rule files load on init.
- `init.md:10-12` says "Load memory from `.specify/memory/`" (no rule pre-load mentioned)
- `AGENTS.md:9` says "Load rules — read `orchestrator-rules.md` AND `governed-artifact-reads.md`"
- `CLAUDE.md:23-28` says "all orchestrated workflows load `governed-artifact-reads.md` (spec 103) and `adversarial-prompt-refusal.md` (CONST-005, spec 131) automatically"

**Resolution:** Make AGENTS.md "New Sessions" Step 0 list **all three** rule files in canonical order; init.md defers; CLAUDE.md is reworded to back-reference rather than enumerate. The three rule files are: `orchestrator-rules.md`, `governed-artifact-reads.md`, `adversarial-prompt-refusal.md`.

**Files affected:**
- `AGENTS.md:9` (canonical) — change Step 0 to list all three rule files.
- `.claude/commands/init.md:10-12` — replace memory-load body with a one-liner that defers to AGENTS.md "New Sessions" Step 0 (the executor still loads memory; see D-2.8).
- `CLAUDE.md:23-28` — change the "all orchestrated workflows load …" paragraph to "see AGENTS.md 'New Sessions' Step 0 for the canonical rule-preload list."

**Cross-phase:** I10. Files moved by I3 (.specify/) and not the rule files themselves; resolution is text-only in I10.

## D-2.2 — Structural-index read path (spec-103 violation in init.md)

**Drift:** `init.md:30` reads `build/codebase-index/index.json` directly, violating spec 103 (governed-artifact-reads) which forbids ad-hoc parsing of `build/**/*.json` in orchestrated workflows.

**Resolution:** Replace the `build/codebase-index/index.json -- compiled structural inventory (if exists)` line in `init.md` with a governed read. The post-cleanup form invokes the consumer binary, not the raw path. Per D8, the governed read becomes `codebase-indexer render` (generic) + `oap-code-index-enrich render` (OAP overlay), both writing to the same path which init.md then reads as plain markdown.

**Specific change to init.md:30:**
```
- `build/codebase-index/index.json` -- compiled structural inventory (if exists)
```
→
```
- `codebase-indexer check` + `codebase-indexer render` — refresh + read the generic structural summary
- (OAP context only) `oap-code-index-enrich render` — apply OAP-overlay; result at `.derived/codebase-index/CODEBASE-INDEX.md`
- read `.derived/codebase-index/CODEBASE-INDEX.md` for the rendered structural view
```

**Files affected:**
- `.claude/commands/init.md:30` (post-cleanup path: same — init.md does not move).
- Path also depends on I9 (`build/` → `.derived/`).

**Cross-phase:** I10, requires I9 (rename) AND I11 (codebase-indexer render restore) to land first. **Order: I9 → I11 → I10.**

## D-2.3 — Identity reads diverge

**Drift:** `init.md:20-23` reads `AGENTS.md`, `CLAUDE.md`, `README.md`. `AGENTS.md:12-13` reads only `CLAUDE.md` and `README.md` (no `AGENTS.md` self-read).

**Resolution (b) per init-trace §Open decisions:** Document the implicit-protocol-source read in a single note. AGENTS.md is implicitly read by `/init` as the protocol source itself; that read happens *before* the listed "New Sessions" steps execute. Adding `AGENTS.md` to its own identity-read list would be self-referential.

**Specific change:** Add a note at AGENTS.md "New Sessions" header (line 3-5 area):

> Note: `AGENTS.md` itself is read implicitly by `/init` as the protocol source before Step 0 fires. It is not duplicated in the identity-read list of Step 1.

And remove `AGENTS.md` from init.md's identity-reads list (line 21), since the implicit read covers it.

**Files affected:**
- `AGENTS.md:3-5` — add the implicit-read note.
- `.claude/commands/init.md:21` — remove the `AGENTS.md` line from the parallel-reads block.

**Cross-phase:** I10.

## D-2.4 — `.specify/contract.md` read missing from AGENTS.md

**Drift:** `init.md:26` reads `.specify/contract.md`. `AGENTS.md` does not list it.

**Resolution (a) per init-trace §Open decisions:** Add to AGENTS.md "New Sessions" Step 1 the contract.md read. Post-I3 the path is `standards/spec/contract.md` (per master plan §Locked target layout). The trace's S2 classifies this file as generic and worth loading; bringing it into AGENTS.md makes the canonical source complete.

**Specific change to AGENTS.md "New Sessions" Step 1 (parallel reads):**

Add:
```
- `standards/spec/contract.md` — spec spine constitutional contract summary
```

And remove from init.md:26:
```
- `.specify/contract.md` -- constitutional contract summary
```

**Files affected:**
- `AGENTS.md:13-19` (Step 1 reads block) — add `standards/spec/contract.md` line.
- `.claude/commands/init.md:26` — remove the `.specify/contract.md` line.

**Cross-phase:** I10, requires I3 (`.specify/contract.md` → `standards/spec/contract.md`) to land first. **Order: I3 → I10.**

## D-2.5 — Spec-list path: `ls` vs governed consumer

**Drift:** `init.md:27` uses `ls specs/`. `AGENTS.md:16` uses `registry-consumer list --ids-only`.

**Resolution (a) per init-trace §Open decisions:** Use the consumer — `registry-consumer list --ids-only`. It's the typed surface and matches lifecycle counts. (Spec 103 strict reading: `specs/` is authored, not compiled, so `ls specs/` is not a violation — but the consumer path is the typed surface and aligns with the spirit of governed reads.)

**Specific change to init.md:27:**

```
- `ls specs/` -- list all feature spec directories (do not read each spec)
```
→
```
- `registry-consumer list --ids-only` — list all feature spec ids via the typed consumer
```

**Files affected:**
- `.claude/commands/init.md:27` — replace the line.
- AGENTS.md already correct; no change needed.

**Cross-phase:** I10. Requires registry-consumer binary build (`/setup` invariant) which is independent.

## D-2.6 — Lifecycle counts missing from init.md

**Drift:** `init.md` does not invoke `registry-consumer status-report`. `AGENTS.md:15` does.

**Resolution (a):** Add the call to `init.md` Step 1 and surface lifecycle counts in the summary template (Step 2). The summary template in init.md:51-71 already exists; lifecycle counts become a new section in that template.

**Specific change to init.md:**
- Step 1 parallel reads: add the call.
- Step 2 summary template: add a `## lifecycle:` section.

```
- `registry-consumer status-report --json --nonzero-only` — lifecycle counts per spec status
```

**Files affected:**
- `.claude/commands/init.md` (Step 1 reads block + Step 2 template) — ~5 lines.

**Cross-phase:** I10.

## D-2.7 — Git log verb count

**Drift:** `init.md:38` uses `-15`; `AGENTS.md:18` uses `-10`. Cosmetic.

**Resolution (b):** Align on `-10` for consistency with AGENTS.md (the canonical source). `-10` is sufficient for the recent-activity summary; `-15` adds noise.

**Specific change to init.md:38:**
```
- `git log --oneline -15` -- recent commits
```
→
```
- `git log --oneline -10` -- recent commits
```

**Files affected:**
- `.claude/commands/init.md:38` — one-character edit.

**Cross-phase:** I10.

## D-2.8 — Memory load only in init.md

**Drift:** `init.md:10-12` reads `.specify/memory/`. `AGENTS.md` does not.

**Resolution (a):** Add an explicit "Step 0' (memory)" line to AGENTS.md "New Sessions" so the canonical protocol covers memory load. Post-I3 the path is `standards/spec/memory/` (per master plan — though master plan §Locked target layout shows `standards/spec/` covering the constitution + contract; the auto-memory store remains at the user-level `~/.claude/projects/.../memory/` per the auto-memory section of CLAUDE.md system prompt, NOT the repo's `.specify/memory/`). The `.specify/memory/` content (constitution.md) graduates to `standards/spec/constitution.md` via I3.

**Important distinction:**
- **Repo `.specify/memory/constitution.md`** → moves to `standards/spec/constitution.md` in I3.
- **User-level auto-memory** → unaffected; lives outside the repo.

The init protocol's memory load is about the user-level auto-memory store, not the repo's `.specify/memory/`. Re-reading init.md:10-12: "Read all files in `.specify/memory/` if the directory exists. If no memory files are found, note 'no prior memory for this project' and continue." This is **the repo's `.specify/memory/`**, which today contains only `constitution.md` (graduating in I3).

**Resolution (refined):** After I3, the repo `.specify/memory/` is gone. The init protocol's "Step 0 — memory" then has two distinct loads:
1. **Spec spine constitution** — `standards/spec/constitution.md` (becomes part of D-2.4's identity reads).
2. **User-level auto-memory** — read via the user's `~/.claude/projects/.../memory/` store; this is per-user/per-project state outside the repo and is handled by the auto-memory subsystem, not by the init protocol.

**Specific change:**
- AGENTS.md "New Sessions" Step 1: add `standards/spec/constitution.md` to the identity-reads list (alongside `standards/spec/contract.md` from D-2.4).
- init.md Step 0 (lines 10-12): replace the `.specify/memory/` read with a deferral to AGENTS.md Step 0/1. The user-level auto-memory load is implicit (the harness handles it).

**Files affected:**
- `AGENTS.md` Step 1 — add 1 line for constitution.md.
- `.claude/commands/init.md:10-12` — replace memory-block body.

**Cross-phase:** I10, requires I3 (constitution graduation) first. **Order: I3 → I10.**

## D-2.9 — OAP-specific listings only in init.md

**Drift:** `init.md:33-35` lists project-specific directories (`ls tools/`, `ls apps/`, `ls docs/`). `AGENTS.md` does not.

**Resolution (a) per init-trace §Open decisions:** Move OAP-specific listings into AGENTS.md "New Sessions" hook so generic init.md template stays generic. Post-I7 the apps/ path becomes `product/apps/`. The listings become:

```
- `ls tools/` — toolchain
- `ls product/apps/` — application targets
- `ls docs/` — documentation index
```

**Files affected:**
- `AGENTS.md` Step 1 (parallel reads block) — add the three `ls` lines.
- `.claude/commands/init.md:33-35` — remove the three lines (or replace with "defer to AGENTS.md").

**Cross-phase:** I10, requires I7 (apps/ → product/apps/) and I5 (tools restructure, though `ls tools/` still works post-I5 because the directory still exists with internal restructure). **Order: I5 + I7 → I10.**

## D-2.10 — Render binary identity

**Drift:** `AGENTS.md:14` calls `oap-code-index-enrich render` (OAP-specific). `init.md` is silent on render. `governed-artifact-reads.md:17` lists `render` under `codebase-indexer` (generic, pre-W-07b name) — stale.

**Resolution (per D8 + init-trace §Open decisions option a):** Reconcile the rule-file table with the actual binaries post-decomposition (D8). After I11 lands:

- `codebase-indexer` regains a `render` subcommand (generic core, produces the L1+L2+Diagnostics view).
- `oap-code-index-enrich render` continues to produce the enriched L1+L2+L3+L4+L5+Diagnostics view (overlay overwrites the generic).

**Specific changes:**
- `AGENTS.md` "New Sessions" Step 1 (line 14): split the single line into two steps — generic render always, OAP-overlay render conditional:
  ```
  - `codebase-indexer render` — write generic structural summary to `.derived/codebase-index/CODEBASE-INDEX.md`
  - (OAP context only) `oap-code-index-enrich render` — overlay Layer 3/4/5 on the same file
  ```
- `.claude/rules/governed-artifact-reads.md` consumer-binary table (lines 15-18): update rows. After I11 the table reads:
  ```
  | .derived/spec-registry/registry.json | registry-consumer | list, ..., status-report |
  | .derived/spec-registry/registry-oap.json | oap-registry-enrich | enrich, compliance-report |
  | .derived/codebase-index/index.json | codebase-indexer | compile, check, render |
  | .derived/codebase-index/CODEBASE-INDEX.md | written by codebase-indexer render (generic) and overwritten by oap-code-index-enrich render (overlay); read directly |
  ```

**Files affected:**
- `AGENTS.md:14` — split into two lines (+1 line net).
- `.claude/rules/governed-artifact-reads.md:15-18` — update 4 rows + 1 example line.

**Cross-phase:** I10 + I11. I11 restores `codebase-indexer render`. I10 updates the protocol text. **Order: I9 (path rename) → I11 (binary capability) → I10 (text update).**

## D-2.11 — Summary template only in init.md

**Drift:** `init.md:51-71` contains the structured `## initialized:` template. `AGENTS.md:19` references the emission step but does not specify the template body.

**Resolution (per init-trace):** No change required. The summary template body belongs in init.md (the executor); AGENTS.md only specifies that Step 2 emits a summary. This asymmetry is intentional: AGENTS.md is the protocol, init.md is the executor with template content.

**Specific change:** None. Document the intentional asymmetry inline in AGENTS.md Step 2 (one-line note):

> Note: The structured summary template body is owned by `.claude/commands/init.md` Step 2. AGENTS.md governs *that the summary is emitted*; init.md governs *how it is shaped*.

**Files affected:**
- `AGENTS.md` Step 2 — one-line note added. (Optional; could also be deferred.)

**Cross-phase:** I10. Optional.

## Aggregate I10 changes

| File | Lines touched | Net delta | Concerns |
|---|---|---|---|
| `AGENTS.md` | ~20 lines (header note, Step 0 expansion, Step 1 additions, Step 2 note) | net +10 lines | canonical source-of-truth |
| `.claude/commands/init.md` | ~15 lines (Step 0 reduced, identity-list trimmed, contract removed, spec-list replaced, OAP-listings removed, git-log adjusted, summary template additions for lifecycle counts) | net -5 lines | thin executor |
| `CLAUDE.md` | ~3 lines (rule-paragraph reworded to back-reference) | net 0 | conventions only |
| `.claude/rules/governed-artifact-reads.md` | ~6 lines (consumer table + example block) | net +2 lines | reflects D8 + I9 |

## I10 readiness summary

- **AGENTS.md changes:** ~20 lines across header, Step 0, Step 1, Step 2.
- **init.md changes:** ~15 lines across Step 0, Step 1, Step 2.
- **CLAUDE.md changes:** ~3 lines (rule-preload paragraph).
- **governed-artifact-reads.md changes:** ~6 lines (consumer table + example).
- **Estimated commits in I10:** 1 or 2.
  - Option A (1 commit): batch all four files in one `refactor(cleanup): align /init protocol; AGENTS.md canonical`. Self-contained.
  - Option B (2 commits): split AGENTS.md changes from the others if reviewer prefers a smaller AGENTS.md-only commit. AGENTS.md edits are the most consequential.
- **Estimated complexity:** **medium**.
  - The text edits are mechanical, but the AGENTS.md "New Sessions" rewrite touches the protocol surface that `/init` parses; a typo here means `/init` breaks for everyone.
  - Test: after I10 lands, run `/init` and verify the structured summary emits correctly with all 11 drift items resolved.

## Cross-phase dependency graph (D9 sub-graph)

```
I3 (.specify/ → standards/spec/) ──┐
                                   │
I5 (tools restructure)           ──┤
                                   ├─→ I10
I7 (apps/ → product/apps/)       ──┤
                                   │
I9 (build/ → .derived/) ──┐        │
                          │        │
                          └→ I11 ──┘
                          (codebase-indexer render restored)
```

I10 cannot fire until I3, I5, I7, I9, and I11 have all landed. That puts I10 near the end of Epic 2's sequence, after the structural moves and the render-path restoration.

## Open questions (surface for operator triage)

1. **D-2.8 memory-load semantics.** Confirm the resolution: the repo's `.specify/memory/` graduates to `standards/spec/constitution.md` (I3), and the init protocol drops the `.specify/memory/` read entirely. The user-level auto-memory at `~/.claude/projects/.../memory/` is handled by the harness, not by the init protocol. If this misreads the intent, surface here.
2. **D-2.11 template ownership.** Should the summary template be hoisted into AGENTS.md so it's centrally specified (with init.md being a pure executor), or stay where it is? D9 recommends staying where it is (current resolution).
3. **D-2.9 OAP-listings placement.** After I7, the listings include `ls product/apps/`. Confirm this is acceptable; alternatively, accept that AGENTS.md becomes "OAP-flavoured" and document that an adopter extracting the spec-spine would prune the OAP-specific lines. D9's current recommendation: keep AGENTS.md as OAP-flavoured (since AGENTS.md *is* this repo's instance of the protocol, not the template).
4. **Single commit vs split.** I10's commit count (1 vs 2) is operator preference; D9 recommends 1 commit for atomicity.
