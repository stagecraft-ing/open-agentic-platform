# Tasks: Tenant-hello demo service

**Input**: [`spec.md`](./spec.md), [`plan.md`](./plan.md)

> Format: `[ID] [P?] Description`. `[P]` = parallelisable with adjacent
> tasks. Tasks track plan.md phases.

## Phase 0 — Lock the contract *(this PR)*

- [x] T001 Author `spec.md` with C-001…C-005 contract, FR-001…FR-005,
  SC-001…SC-003.
- [x] T002 Add `oap.spec = "136-tenant-hello-demo-service"` to
  `platform/services/tenant-hello/package.json`.
- [x] T003 Extend `tools/codebase-indexer/src/manifest.rs`
  `discover_npm_packages` to include `tenant-hello` alongside
  `stagecraft`. Verify Layer 1 NPM table and Layer 2 traceability list
  the fixture.
- [x] T004 Author `plan.md` and `tasks.md` (this file).
- [ ] T005 Reviewer pass — confirm C-001…C-005 wording, then flip
  `status: draft → approved` in a separate PR.

**Checkpoint:** Phase 0 closes when T005 ships. Phases 1–3 are blocked
behind this checkpoint per Principle III.

---

## Phase 1 — Helm chart for tenant-hello

- [ ] T010 Create `platform/charts/tenant-hello/Chart.yaml` (apiVersion
  v2, version pinned to a fresh `0.1.0`).
- [ ] T011 [P] `values.yaml` with image, replica count (1), service
  port (8080 default, overridable), ingress disabled by default, security
  context (non-root, read-only filesystem).
- [ ] T012 [P] `templates/deployment.yaml` — single Deployment with
  `PORT` env injection (clause C-003), readiness probe on `/healthz`
  (C-002), liveness probe on the same path with a longer
  `initialDelaySeconds`.
- [ ] T013 [P] `templates/service.yaml` — ClusterIP service exposing the
  service port.
- [ ] T014 [P] `templates/serviceaccount.yaml` — bound SA, no extra
  permissions.
- [ ] T015 `templates/ingress.yaml` (gated on
  `values.ingress.enabled`) — host driven by a stagecraft-supplied
  per-tenant value.
- [ ] T016 `helm lint platform/charts/tenant-hello/` clean. Add to
  `make ci` (or the `ci-charts` recipe if it exists).
- [ ] T017 `helm template platform/charts/tenant-hello --values
  platform/charts/tenant-hello/values.yaml` rendered into a fixture file
  for golden comparison; check the rendered Deployment/Service/SA
  shape.

**Checkpoint:** Helm chart lints, renders deterministically, matches
the C-clause obligations.

---

## Phase 2 — Stagecraft chart-per-tenant wiring

- [ ] T020 Add `chartSelector` to
  `platform/services/stagecraft/api/projects/` (location subject to
  spec 119 conventions; pick the existing project-creation surface).
  Input: a project's manifest descriptor; output: a chart name for
  `deployd-api-rs` to apply. tenant-hello is the first registered
  shape.
- [ ] T021 Stagecraft API surface change documented inline in the
  service's CLAUDE.md (per the in-tree governance docs convention).
- [ ] T022 `deployd-api-rs` deploy invocation accepts the resolved chart
  name from stagecraft. Existing `--chart` flag wiring re-used; if not
  present, add one with a unit test.
- [ ] T023 End-to-end happy path: register a project pointing at
  `platform/services/tenant-hello/`, click "deploy" in stagecraft, get
  a pod live behind the cluster's ingress whose `/healthz` returns 200.
  Capture the trace as evidence under
  `specs/136-tenant-hello-demo-service/execution/verification.md`.

**Checkpoint:** SC-002 first half (positive path) is evidenced.

---

## Phase 3 — Negative-path validation

- [ ] T030 Pick one C-clause to violate per pass. Recommended order:
  C-002 first (omit `/healthz` — surfaces the readiness-probe failure
  shape most cleanly), then C-003 (hard-code a port), then C-001
  (privileged Dockerfile build step).
- [ ] T031 For each violation, run the deploy pipeline; assert the
  failure is **localised** (cites the C-clause and the offending
  artifact path) rather than a generic platform crash. Record each
  failure in `execution/verification.md`.

**Checkpoint:** SC-002 second half (negative path) is evidenced.

---

## Phase 4 — Status flip *(separate PR)*

- [ ] T040 Amend `spec.md` frontmatter:
  `implementation: in-progress → complete`,
  `status: draft → approved`, add `approved:`/`completed:` dates.
- [ ] T041 Add a delivery-record section under §"Clarifications" or
  in a `delivery-record.md` companion citing the verification artifact
  paths.
- [ ] T042 Confirm spec-code coupling gate (specs 127/130/133) is happy
  with the lifecycle flip — i.e. the PR also touches code paths bound
  to spec 136, so the amender→amended evidence is present.

**Checkpoint:** spec 136 is closed.

---

## Dependencies & ordering

- T001–T004: complete.
- T005 unblocks T010+ (Principle III: no implementation under draft).
- T010–T015 are mostly parallel (different chart files); T016/T017 wait
  on the chart skeleton.
- T020 depends on T010+ (stagecraft needs a chart to point at). T022
  depends on T020.
- Phase 3 depends on a complete Phase 1+2.
- Phase 4 depends on Phase 3 evidence being captured.

## Notes

- Tasks marked `[P]` touch separate files within the chart and can be
  drafted in parallel by one or many hands.
- "Spec evidence" in this project means a file under
  `execution/verification.md` per spec 005-verification-reconciliation-mvp;
  not a separate test framework.
- Avoid skipping T005. The reviewer pass is the lone defence against
  the C-clauses being weakened on contact with chart values.
