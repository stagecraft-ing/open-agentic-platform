---
id: "208-org-agent-kill-switch"
title: "Org-Wide Agent Kill Switch and Quarantine (ASI10 containment)"
feature_branch: "feat/208-org-agent-kill-switch"
status: draft
implementation: pending
kind: governance
domain: platform
created: "2026-06-11"
authors: ["open-agentic-platform"]
language: en
summary: >
  Containment today is precise but narrow: an operator can
  force-disconnect one session (spec 172), the four-key revocation
  lattice quarantines factory content (spec 198 FR-010), and Rauthy
  sessions can be revoked one principal at a time. What does not exist
  is the emergency stop ASI10 m4 and ASI04 m8 require: one
  admin-gated action that halts agent execution org-wide — pausing
  in-flight runs at the next boundary, refusing new sessions and grant
  issuance, revoking agent credentials — with propagation inside a
  stated bound, a quarantine record, and reintegration that requires
  fresh validation plus human approval. The switch composes the levers
  that already exist (grant renewal, duplex channels, revocation rows,
  session force-disconnect) rather than adding a parallel mechanism,
  and it is drilled: a kill switch that has never been pulled is
  decoration, so exercising it is an acceptance criterion, not an
  operational afterthought.
code_aliases: ["ORG_AGENT_KILL_SWITCH"]
compliance:
  - framework: "owasp-asi-2026"
    controls: ["ASI10", "ASI08"]
depends_on:
  - "198-factory-governance-envelope"
  - "172-opc-live-agent-session-introspection"
  - "106-rauthy-native-oidc-and-membership"
extends:
  # The org scope is an additive widening of the revocation lattice
  # spec 198 FR-010 establishes.
  - spec: "198-factory-governance-envelope"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/api/factory/revocations.ts }
  # Same precedent as specs 196, 194, 193, 187, 183: a new spec adds a row
  # to the featuregraph golden.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
refines:
  - aspect: "org-halt-propagation"
    unit: { kind: file, path: platform/services/stagecraft/api/sync/service.ts }
  - aspect: "halt-aware-session-termination"
    unit: { kind: file, path: product/apps/opc/src-tauri/src/commands/live_sessions.rs }
references:
  - role: machinery
    unit: { kind: file, path: platform/services/stagecraft/api/factory/grantDuplexHandlers.ts }
  - role: context
    unit: { kind: file, path: platform/services/stagecraft/api/factory/auditActions.ts }
  - role: context
    unit: { kind: file, path: docs/owasp-agentic-top-10-2026.md }
---

# Feature Specification: Org-Wide Agent Kill Switch and Quarantine

**Feature Branch**: `208-org-agent-kill-switch`
**Created**: 2026-06-11
**Status**: Draft (follow-on filed by the ASI gap-closure pass)
**Input**: The ASI 2026 gap analysis (2026-06-10) found: "Four-key
revocation covers factory content; there is no 'halt all agents / revoke
all agent credentials org-wide' lever. Per-session force-disconnect (172)
is the only kill-switch." Cross-cutting principle 10: containment and
recovery are designed in before deployment, not improvised after
compromise.

## Purpose

The incident that motivates this spec is the one OAP's posture already
assumes possible: a supply-chain compromise (ASI04) or rogue-agent
divergence (ASI10) discovered *while agents are running across the org*.
The operator's first question is not "which session?" — it is "how do I
stop everything, now, without losing forensic state?" Today the honest
answer is a loop over sessions and a prayer that nothing re-connects.

The design constraint is composition: every primitive the switch needs
exists. Grant renewal (198 FR-005) gives a natural pause boundary;
revocation rows (198 FR-010) give fail-closed serve/bind/renewal checks;
the duplex channel reaches every connected engine; force-disconnect (172)
terminates an unresponsive one; checkpointing preserves state at the
kill. The switch is the org-scoped composition of these, plus the
state machine (active → halted → reintegrating) and its audit story.

## Functional requirements (sketch — refine before implementation)

- **FR-001 — Halt verb.** An admin-gated platform action sets org halt.
  While active: new agent sessions are refused, new run-grants and grant
  renewals are refused (in-flight runs therefore pause at the next stage
  boundary, the 198 FR-005 semantics), and a halt notice is pushed over
  every connected duplex channel so engines pause at the next
  instruction boundary without waiting for renewal. Engines checkpoint
  on halt (the 172 force-disconnect sequence) — containment must not
  destroy the forensic state it exists to protect.
- **FR-002 — Credential revocation propagation.** Halt revokes
  outstanding run-grants (serve-time checks refuse them immediately;
  renewal refusal catches the rest) and, once spec 205 lands, agent
  NHIs via the delegation index. Scope lattice: the verb takes `org`,
  `project`, or `agent-profile` scope — the ASI10 quarantine case is
  rarely all-or-nothing, and a narrower halt that works beats a broad
  one nobody dares pull.
- **FR-003 — Propagation bound, stated and measured.** The halt
  contract states its bound (connected engines: next instruction
  boundary; disconnected engines: at reconnect handshake, which checks
  halt state fail-closed before any other traffic). The audit record
  captures per-engine acknowledgment timestamps so the realized bound is
  measurable after every pull, drill or real.
- **FR-004 — Quarantine record and reintegration.** The halt writes a
  quarantine record (who pulled it, why, scope, evidence links). Lifting
  follows the spec 198 FR-010 lift contract verbatim: fresh two-sided
  validation of the affected factories plus an explicit human actor id —
  a service identity cannot lift (the spec 200 AC-4 pattern), and
  reintegration is staged (sessions re-admitted per scope, not a global
  flip).
- **FR-005 — Drill requirement.** The e2e harness (spec 187 territory)
  exercises the full pull-and-lift cycle on every release candidate:
  halt during a seeded multi-session run, assert the propagation bound,
  lift, assert staged reintegration. ASI01 m8's "verify rollback works"
  applied to containment.

## Acceptance criteria (sketch)

- **AC-1.** With three concurrent sessions (one mid-run, one idle, one
  disconnected), org halt: the mid-run session pauses and checkpoints at
  the next boundary; the idle session's next action is refused; the
  disconnected session is refused at reconnect handshake.
- **AC-2.** During halt, grant issuance, grant renewal, and new session
  registration are all refused with errors naming the quarantine record.
- **AC-3.** A `project`-scoped halt leaves sibling projects' sessions
  running (scope lattice proven).
- **AC-4.** Lift requires a human actor id and re-validation; the
  scanner/service identity is rejected; per-engine acknowledgment
  timestamps exist for the halt and the lift.
- **AC-5.** The drill (FR-005) runs in the e2e harness and is green on a
  release candidate.

## Out of scope

- Single-run budget circuit-breaking (spec 202; the governor bounds one
  run, the switch stops the fleet).
- Per-agent credential issuance (spec 205; FR-002 consumes its index
  when available, and degrades to grant+session revocation until then).
- Automated halt triggers (anomaly-detection-initiated halt is future
  work; this spec's actor is a human admin — automation proposing,
  human pulling, is the ASI08-honest division).
- Content quarantine semantics (spec 198 FR-010 owns the four-key
  lattice; this spec widens scope, it does not redefine keys).

## Sequencing

After spec 198 reaches `implementation: complete` (grant machinery and
revocation rows are the substrate). The drill (FR-005) lands with the
e2e harness integration, not as a later promise.
