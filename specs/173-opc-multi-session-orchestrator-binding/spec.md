---
id: "173-opc-multi-session-orchestrator-binding"
slug: opc-multi-session-orchestrator-binding
title: "OPC multi-session-by-project-path — orchestrator binding (formalisation)"
status: draft
implementation: pending
owner: bart
created: "2026-05-22"
kind: governance
risk: low
depends_on:
  - "052"  # state-persistence (the spec this spec refines)
  - "095"  # checkpoint-branch-of-thought
  - "157"  # opc-session-model (the existing closure spec)
code_aliases: ["MULTI_SESSION_ORCHESTRATOR_BINDING"]
refines:
  - aspect: "orchestrator-session-binding"
    unit: { kind: file, path: specs/052-state-persistence/spec.md }
references:
  - role: decomposition-source
    unit: { kind: file, path: docs/owasp/factory/AIDE-VELOCITY-OAP-INTENT.md }
  - role: predecessor-spec
    unit: { kind: file, path: specs/157-opc-session-model/spec.md }
  - role: companion-mechanism
    unit: { kind: file, path: specs/095-checkpoint-branch-of-thought/spec.md }
summary: >
  Spec 157 formalises OPC's multi-session-per-project-
  path discipline at the *session model* layer
  (filesystem layout, JSONL authority, non-
  disambiguating across developers by design). Intent
  doc §9.14 names a separate concern: binding that
  multi-session discipline to the *orchestrator* (per
  spec 052), so that workflow state — currently per-
  task in spec 052 — accumulates across sessions for
  a given project path.

  Spec 157 covers what OPC reads; spec 173 covers what
  the orchestrator persists in response. The two
  compose: a developer's many OPC sessions for one
  project path each correspond to (possibly multiple)
  orchestrator workflows, and the orchestrator's
  persistence keys workflows by project-path identity
  consistent with spec 157's discipline.

  This is the formalisation §9.14 of the intent doc
  names. The discipline is already partially implicit
  in spec 052; spec 173 binds it explicitly so the
  orchestrator's session-and-workflow model and OPC's
  session model agree on identity.
---

# 173 — OPC multi-session-by-project-path: orchestrator binding

## 1. Problem

Spec 157 closes the spec-authority gap on OPC's
session model. The Tauri commands in
`product/apps/desktop/src-tauri/src/commands/claude.rs`
read Claude Code's JSONL session history and surface a
multi-session-per-project-path discipline. Spec 157
makes that discipline normative.

Separately, spec 052 (state-persistence) governs the
orchestrator's persistence of workflow state. Workflows
today are persisted *per task* — the orchestrator
dispatches a task, persists its progress, and the task
completes (or fails) as a unit.

Two model gaps follow:

1. **No mapping between OPC sessions and orchestrator
   workflows.** A developer running multiple OPC
   sessions for the same project may invoke the
   orchestrator from each; the orchestrator does not
   associate the resulting workflows with the
   sessions that initiated them. Per-session
   continuity at the orchestrator layer doesn't exist.
2. **No project-path-keyed workflow accumulation.**
   The orchestrator keys workflows by task identity,
   not by project-path identity. A developer
   accumulating institutional memory across many
   sessions on one project sees that memory at the
   OPC session list level (spec 157) but not at the
   orchestrator workflow level.

The intent doc §9.14 names this:

> *"OPC multi-session-by-project-path session model
> (formalised). Per §4.2. Captures the discipline
> that's already implicit in OPC and binds it to the
> spec spine. Kind: governance; refines spec 052."*

§4.2 says explicitly:

> *"OPC's session model is *multi-session per project
> path*. Each session is independently persisted;
> logged-in user identity disambiguates sessions
> across developers; checkpoint hooks (spec 095)
> capture branch-of-thought without forcing a single
> linear session per project."*

Spec 157 formalises this for OPC's *read* surface
(the JSONL surface). Spec 173 formalises it for the
*orchestrator's persistence* surface — the
counterpart on the workflow side.

## 2. Decision

Refine spec 052 with project-path identity as a
first-class key on persisted workflows. Workflows
gain a `project_path:` field; the orchestrator's
persistence surface accumulates workflows keyed by
this identity in addition to the existing per-task
keying.

### 2.1 Identity binding

A workflow initiated from an OPC session carries the
session's `project_path` (per spec 157 §3.1's
discipline) as the workflow's identity field. The
orchestrator persists:

```
workflow:
  workflow_id: <uuid>
  task_id: <existing-per-task-identity>
  project_path: <decoded-fs-path>   # NEW — spec 173
  originating_session: <session-uuid>  # NEW — spec 173
  ...
```

The orchestrator's existing per-task semantics are
unchanged; spec 173 adds two derived fields without
modifying task semantics.

### 2.2 Project-path-keyed accumulation

A new orchestrator consumer surface — `workflows
by-project-path <path>` — returns all workflows for a
given project path across sessions. This is the
substrate spec 172 (live session introspection) uses
to render workflow rows per project.

### 2.3 Non-disambiguation honoured

Per spec 157 §4 ("non-disambiguating across
developers by design"), the orchestrator does not
disambiguate workflows by developer identity. Two
developers running OPC against the same workstation
filesystem path produce workflows that accumulate in
the same project-path bucket. Disambiguation is
deferred (as spec 157 defers it for sessions).

### 2.4 No per-session orchestrator state

Spec 173 binds workflows to project-paths, not to
sessions individually. A workflow knows its
*originating* session (the `originating_session`
field), but the orchestrator does not maintain
per-session state — the orchestrator's unit of
persistence is the workflow, not the session.

This keeps the model honest: sessions are OPC's
unit (spec 157); workflows are the orchestrator's
unit; the project path is the shared key.

## 3. Functional Requirements

- **FR-001** The orchestrator's workflow persistence
  schema (per spec 052) adds two fields:
  `project_path` and `originating_session`. Both are
  required for workflows initiated from OPC sessions;
  optional with documented `null` semantics for
  workflows initiated from other surfaces (CLI,
  scheduled tasks, factory-engine).
- **FR-002** When OPC initiates an orchestrator
  workflow, the workflow's `project_path` is the
  decoded JSONL-authoritative path (per spec 157
  §3.3 — JSONL content wins over directory-name
  encoding on disagreement).
- **FR-003** A new orchestrator consumer surface —
  `workflows by-project-path <path>` — returns all
  workflows for the given project path, sorted by
  recency.
- **FR-004** The orchestrator does not maintain
  per-session state. Sessions are OPC's unit; the
  orchestrator references them via
  `originating_session` but does not key persistence
  on session identity.
- **FR-005** The non-disambiguating posture of spec
  157 is honoured: two developers' workflows for the
  same workstation filesystem path accumulate in the
  same project-path bucket; the orchestrator does
  not disambiguate.
- **FR-006** Existing workflows (predating this
  spec's implementation) gain `project_path: null` in
  their persisted records. Backfilling is not
  required; future workflows carry the field.
- **FR-007** Spec 172's Live Sessions panel reads
  workflows via the new `workflows by-project-path`
  surface, not via ad-hoc orchestrator state
  parsing.

## 4. Success Criteria

- **SC-001** An OPC-initiated orchestrator workflow
  carries `project_path` matching the spec 157
  JSONL-authoritative path.
- **SC-002** Querying workflows by project path
  returns all workflows for that path across
  sessions and developers, sorted by recency.
- **SC-003** A workflow initiated outside OPC (CLI,
  scheduled task, factory-engine) persists with
  `project_path: null` and surfaces correctly
  through existing consumer surfaces.
- **SC-004** The orchestrator's persistence schema
  is backward-compatible: existing workflows
  remain valid without modification.

## 5. Scope

### In scope

- The two added fields on the orchestrator's
  workflow persistence schema.
- The `workflows by-project-path` consumer surface.
- Integration with OPC's workflow-initiation paths.
- Backward compatibility for pre-spec-173 workflows.

### Out of scope (deferred)

- **Per-session orchestrator state.** Sessions
  remain OPC's unit. The orchestrator does not key
  on session identity.
- **Developer-level disambiguation.** Spec 157
  deferred this; spec 173 honours the deferral.
- **Cross-workstation aggregation.** Like spec 172,
  spec 173 is workstation-local. Cross-workstation
  workflow aggregation is a future stagecraft-side
  concern.
- **Workflow migration UI.** Pre-spec-173 workflows
  carry `null` project_path; no UI surfaces this
  gap or offers to backfill. Migration tooling, if
  needed, is a future spec.

## 6. Relationship to spec 157

Spec 157 establishes OPC's session model: many
sessions per project path, JSONL-authoritative
identity, non-disambiguating across developers.

Spec 173 establishes the orchestrator's binding to
that model: workflows persisted with project_path +
originating_session, project-path-keyed accumulation,
inheriting the non-disambiguating posture.

The two together formalise the discipline the intent
doc §4.2 names. Spec 157 is the OPC-side closure;
spec 173 is the orchestrator-side formalisation. A
future contributor reading either spec finds the
other in the cross-references; the discipline is
expressed in two specs precisely because the two
specs cover different layers.

## 7. Cross-references

- **INTENT doc** §4.2, §9.14.
- **Spec 052** — state-persistence; refined by this
  spec.
- **Spec 157** — OPC session model; the predecessor
  closure that spec 173 mirrors at the orchestrator
  layer.
- **Spec 095** — checkpoint-branch-of-thought; the
  companion mechanism for multi-session continuity.
- **Spec 172** — live session introspection;
  consumes the project-path-keyed accumulation
  surface.
