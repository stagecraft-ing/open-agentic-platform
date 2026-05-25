---
name: init
description: Initialize an open-agentic-platform session by executing the cross-agent New Sessions protocol declared in AGENTS.md (parallel reads, governed status reports, structural index render).
---

# /init — Session bootstrap

This skill is a thin Claude-Code-specific dispatcher. The canonical
session-init protocol lives in **`AGENTS.md` § New Sessions**, which
is the cross-agent authority (read by Claude Code, Codex CLI, Cursor,
GitHub Copilot per the AAIF/Linux Foundation AGENTS.md standard).

## What to do

1. **Read `AGENTS.md` § New Sessions** — the section from the
   `## New Sessions` heading inclusive to the next `## ` heading
   exclusive. That section is the authoritative step list.
2. **Execute the protocol described there** using Claude Code's
   parallel tool calls where the protocol says "dispatch
   simultaneously", and sequential calls where the protocol says
   "after step N completes".
3. **Emit the structured summary** in the shape the protocol
   prescribes (currently the `## initialized: open-agentic-platform`
   block per `standards/spec/templates/`).

## Why this dispatcher is thin

The protocol body is governed by spec
**103-init-protocol-governed-reads** and stewarded under the AGENTS.md
standard. Duplicating the step list here would create a second
canonical location and require keeping two files in sync. The
dispatcher reads the authoritative source on every invocation.

Implementation note (spec 182 AC-9): this dispatcher must produce a
read list and tool-call sequence that is structurally equivalent to
the legacy `commands/init.md` flow. See
`specs/182-claude-skills-migration/baseline-init.txt` for the
captured baseline manifest and AC-9 for the equivalence criteria.
