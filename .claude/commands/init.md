---
name: init
description: Initialize an open-agentic-platform session by loading core repository context, recent activity, and memory before implementation work begins.
---

You are initializing a new session for the open-agentic-platform (OAP) repository. This is a self-extending protocol: the "New Sessions" section of AGENTS.md defines the init checklist. Any item added there is automatically picked up on the next init.

---

## Step 0: Load memory

Read all files in `.specify/memory/` if the directory exists. If no memory files are found, note "no prior memory for this project" and continue.

---

## Step 1: Parallel reads

Dispatch all of the following simultaneously (batch them in a single response). If any file is missing, log "not found" and continue.

**Project identity:**
- `AGENTS.md` -- agent instructions and the canonical "New Sessions" checklist that drives this protocol
- `CLAUDE.md` -- session-level rules and guidance
- `README.md` -- project overview

**Spec spine:**
- `.specify/contract.md` -- constitutional contract summary
- `ls specs/` -- list all feature spec directories (do not read each spec)

**Structural index:**
- `build/codebase-index/index.json` -- compiled structural inventory (if exists)

**Build and tool state:**
- `ls tools/` -- available toolchain
- `ls apps/` -- application targets
- `ls docs/` -- documentation index

**Recent activity:**
- `git log --oneline -15` -- recent commits
- `git diff --stat HEAD~1` -- last commit diff summary
- `git branch --show-current` -- current branch
- `git status --short` -- uncommitted changes

**Self-extending hook:** If `AGENTS.md` exists and contains a "New Sessions" section, parse that section for any additional files or commands listed there and execute them as part of this step. This is the extensibility mechanism -- contributors add new init items to AGENTS.md and they are automatically loaded on next session init.

---

## Step 2: Emit initialization summary

After all reads complete, emit a structured summary block in exactly this format:

```
## initialized: open-agentic-platform

**Type:** governed spec-driven platform (Spec Spine + OPC + Platform)
**Branch:** <current branch>
**Uncommitted:** <yes/no + short summary if yes>

**Structural index:** {loaded/not found}
  - {N} Rust crates, {M} npm packages
  - {K} specs traced, {O} orphaned specs, {U} untraced paths
**Spec spine:** <N> feature specs (latest: <most recent spec dir name>)
**Tools:** <list from tools/>
**Apps:** <list from apps/>

**Recent activity:**
- <last 3 commit summaries>

**From memory:** <key points from .specify/memory/ or "none loaded">

**Ready to help with:** spec authoring, compiler/lint/consumer toolchain, OPC desktop, feature lifecycle, execution protocol, verification, governance
```

If AGENTS.md was found, append any project-specific guidance or active priorities mentioned in its "New Sessions" section to the "Ready to help with" line.

---

## Rules

- Never skip steps. If a file is missing, say so and move on.
- Never fabricate content that was not read from disk.
- Keep the summary concise. The goal is orientation, not exhaustive documentation.
- This protocol is self-extending: the AGENTS.md "New Sessions" section is the single place to add new init-time reads or checks. Do not hardcode project-specific items here that belong there.
