---
name: init
description: Initialize an open-agentic-platform session by loading core repository context, recent activity, and memory before implementation work begins.
---

You are initializing a new session for the open-agentic-platform (OAP) repository. This is a thin executor — the canonical protocol lives in `AGENTS.md` "New Sessions". This file dispatches the steps that AGENTS.md prescribes.

---

## Step 0: Load rules

Per AGENTS.md Step 0, the three foundational rule files are loaded
automatically by every orchestrated workflow:

- `.claude/rules/orchestrator-rules.md` (the six behavioural rules)
- `.claude/rules/governed-artifact-reads.md` (spec 103)
- `.claude/rules/adversarial-prompt-refusal.md` (CONST-005, spec 131)

No additional memory-load step here — the rule files are the
protocol memory. The legacy `.specify/memory/` directory is retired
in Epic 2 I13.

---

## Step 1: Parallel reads

Dispatch all of the following simultaneously (batch them in a single response). If any file is missing, log "not found" and continue.

**Project identity:**
- `CLAUDE.md` — session-level rules and guidance
- `README.md` — project overview

**Spec spine (graduated under standards/):**
- `standards/spec/contract.md` — spec spine contract (graduated from `.specify/contract.md` in I3)
- `standards/spec/constitution.md` — constitutional baseline (graduated from `.specify/memory/constitution.md` in I3)

**Spec lifecycle and id list (governed reads, no ad-hoc parsing):**
- `registry-consumer status-report --json --nonzero-only` — non-empty lifecycle counts (spec 103)
- `registry-consumer list --ids-only` — flat id list for latest-spec detection

**Structural index (consumer binary, not direct JSON parsing):**
- `codebase-indexer check` — staleness gate (non-fatal)
- `codebase-indexer render` — generic Layer 1+2+Diagnostics markdown (the spec-spine view)
- (optional) `oap-code-index-enrich render` — OAP overlay over the generic core, emits Layers 3-5 to `.derived/codebase-index/CODEBASE-INDEX.md`

**Tree surfaces (post-Epic-2 layout):**
- `ls tools/` — top-level tool subdivision (spec-spine/, oap/, shared/, vendor/)
- `ls product/apps/` — desktop app discovery
- `ls docs/` — graduated docs surface

**Recent activity:**
- `git log --oneline -10` — recent commits
- `git diff --stat HEAD~1` — last commit diff summary
- `git branch --show-current` — current branch
- `git status --short` — uncommitted changes

**Self-extending hook:** AGENTS.md "New Sessions" is canonical. If you find an item there that this file does not dispatch, surface the gap and add it on the next refresh — do not silently skip.

---

## Step 2: Emit initialization summary

After all reads complete, emit a structured summary block in exactly this format. The shape comes from `standards/spec/templates/` (graduated from `.specify/templates/` in I3); modify the shape there, not here.

```
## initialized: open-agentic-platform

**Type:** governed spec-driven platform (Spec Spine + OPC + Platform)
**Branch:** <current branch>
**Uncommitted:** <yes/no + short summary if yes>

**Structural index:** {loaded/stale/not found}
  - {N} Rust crates, {M} npm packages
  - {K} specs traced, {O} orphaned specs, {U} untraced paths

**Spec spine:** <N> feature specs (latest: <most recent spec dir name>)

## lifecycle:
  - <status>: <count>
  - ... (only non-zero rows, from registry-consumer status-report --nonzero-only)

**Tools:** spec-spine/{spec-compiler, registry-consumer, ...}; oap/{policy-compiler, ...}; shared/spec-types; vendor/grammars
**Product:** product/apps/desktop (Tauri + React)

**Recent activity:**
- <last 3 commit summaries>

**Ready to help with:** spec authoring, compiler/lint/consumer toolchain, OPC desktop, feature lifecycle, execution protocol, verification, governance
```

---

## Rules

- Never skip steps. If a file is missing, say so and move on.
- Never fabricate content that was not read from disk.
- Never parse `.derived/**/*.json` directly (spec 103). Use the consumer binaries.
- Keep the summary concise. The goal is orientation, not exhaustive documentation.
- This protocol is self-extending: AGENTS.md "New Sessions" is canonical. Do not hardcode project-specific items here that belong there.
