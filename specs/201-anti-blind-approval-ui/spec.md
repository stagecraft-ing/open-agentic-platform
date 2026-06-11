---
id: "201-anti-blind-approval-ui"
title: "Anti-Blind-Approval UI (ASI09 human-agent trust surfaces)"
feature_branch: "feat/201-anti-blind-approval-ui"
status: draft
implementation: pending
kind: platform
domain: platform
created: "2026-06-10"
authors: ["open-agentic-platform"]
language: en
summary: >
  Close the declared ASI09 gap in the spec 198 all-ten analysis: the
  approval surfaces where a human ratifies agent work (HITL gates declared
  in the governance envelope, factory run approvals, override verification)
  must present plain-language risk summaries grounded in provenance — what
  changes, with what blast radius, traceable to which content hashes — and
  must never present model-generated rationale as the basis for approval.
  Preview is never effect: every approval surface renders from recorded
  facts (diffs, scopes, hashes, gate predicates), and the act of previewing
  cannot mutate state. This spec owns the presentation contract; the gates
  themselves are declared by the envelope (spec 198 FR-008) and enforced by
  the existing HITL machinery (spec 166).
code_aliases: ["ANTI_BLIND_APPROVAL_UI"]
depends_on:
  - "198-factory-governance-envelope"
  - "166-opc-stop-hook-gate-chain"
refines:
  - aspect: "hitl-approval-presentation"
    unit: { kind: file, path: platform/services/stagecraft/web/app/routes/app.factory.runs.$runId.tsx }
  - aspect: "hitl-approval-presentation"
    unit: { kind: file, path: platform/services/stagecraft/web/app/routes/app.factory.artifacts.tsx }
references:
  - role: context
    unit: { kind: file, path: docs/owasp-agentic-top-10-2026.md }
  - role: gate-declaration
    unit: { kind: file, path: standards/schemas/factory/governance-envelope.schema.yaml }
---

# Feature Specification: Anti-Blind-Approval UI

**Feature Branch**: `201-anti-blind-approval-ui`
**Created**: 2026-06-10
**Status**: Draft (follow-on stub filed by spec 198 phase 5, AC-7)
**Input**: Spec 198's all-ten ASI table is honest that ASI09 (Human-Agent
Trust Exploitation) remains **partial — declared gap**: the envelope can
declare `requires: plain-language-summaries` and `preview-no-side-effects`
on its HITL gates, and spec 166 enforces that a gate exists, but nothing
yet governs what the approving human actually *sees*. Blind approval — a
human rubber-stamping output they cannot meaningfully evaluate — is the
exploitation surface this spec closes.

## Purpose

An approval is governance evidence only if the approver had a real basis.
Three properties make a basis real, and each is a presentation-layer
contract this spec owns:

1. **Plain-language, fact-grounded summaries.** What is being approved,
   stated from recorded facts: files/scopes touched, commands to run,
   content hashes consumed (including the spec 198 FR-013 consumed
   overrides and their verified state), gate predicate being satisfied.
2. **Provenance attached, rationale excluded.** Every claim links to its
   source (artifact id, hash, audit row). Model-generated rationale —
   "this change is safe because…" — is never rendered as the approval
   basis (ASI09 m*: the persuasion channel is the attack channel). Where
   model output must be shown, it is visibly labelled untrusted content,
   segregated from the facts.
3. **Preview ≠ effect.** Rendering an approval surface performs no
   mutation; the approve action is a distinct, audited, idempotent step
   (composing with spec 198 FR-008's preview-no-side-effects predicate).

## Functional requirements (sketch — refine before implementation)

- **FR-001 — Approval summary contract.** A typed summary shape (facts,
  provenance links, blast-radius statement, gate predicate) rendered by
  every approval surface; assembled server-side from recorded state, never
  from model output.
- **FR-002 — Untrusted-content segregation.** Model-produced text on an
  approval surface is rendered in a visually distinct, explicitly labelled
  region; the approve control never appears inside that region.
- **FR-003 — Preview purity.** Loading any approval view is side-effect
  free (GET semantics, no state transitions); approving is a separate
  POST with its own audit action.
- **FR-004 — Approval evidence.** The audit row for an approval records
  what was shown (summary hash), by whom, and under which gate predicate —
  so the certificate chain can later prove the basis, not just the click.

## Acceptance criteria (sketch)

- **AC-1.** Every HITL gate surface renders the typed summary; a gate
  whose summary cannot be assembled refuses to render an approve control
  (fail-closed, attributable error).
- **AC-2.** No approval surface renders model rationale outside the
  labelled untrusted region (test posture + review checklist).
- **AC-3.** Loading any approval view leaves the database unchanged
  (effect-free preview test).
- **AC-4.** Approval audit rows carry the summary hash; the spec 198
  ASI09 table row flips from "partial — declared gap" only when this
  lands.

## Out of scope

- The gate machinery itself (spec 166 owns stop-hook enforcement; spec
  198 FR-008 owns gate declaration).
- Override verification *semantics* (spec 198 FR-013 c); this spec only
  governs how the verify surface presents its basis.
- Notification/paging of approvers (spec 057 territory).

## Sequencing

Implementable independently of spec 200; shares spec 198's
runtime-verification precondition only for the consumed-override rows it
renders (present in bundles after spec 198 phase 5).
