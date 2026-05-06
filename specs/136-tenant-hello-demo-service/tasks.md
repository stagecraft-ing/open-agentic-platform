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
- [x] T005 Reviewer pass — confirm C-001…C-005 wording, then flip
  `status: draft → approved` in a separate PR. **Done 2026-05-06**:
  contract verified against the tenant-hello fixture (Dockerfile
  non-privileged, `/healthz` returns 200 with non-empty body, PORT env
  honoured, stateless, single declared dependency). Lifecycle: status
  draft → approved; implementation stays `in-progress` until Phase 3
  evidence per §Phase 4.

**Checkpoint:** Phase 0 closes when T005 ships. Phases 1–3 are blocked
behind this checkpoint per Principle III.

---

## Phase 1 — Helm chart for tenant-hello

- [x] T010 Create `platform/charts/tenant-hello/Chart.yaml` (apiVersion
  v2, version pinned to a fresh `0.1.0`). **Done 2026-05-06.**
- [x] T011 [P] `values.yaml` with image, replica count (1), service
  port (8080 default, overridable), ingress disabled by default, security
  context (non-root, read-only filesystem). **Done 2026-05-06.**
- [x] T012 [P] `templates/deployment.yaml` — single Deployment with
  `PORT` env injection (clause C-003), readiness probe on `/healthz`
  (C-002), liveness probe on the same path with a longer
  `initialDelaySeconds`. **Done 2026-05-06.**
- [x] T013 [P] `templates/service.yaml` — ClusterIP service exposing the
  service port. **Done 2026-05-06.**
- [x] T014 [P] `templates/serviceaccount.yaml` — bound SA, no extra
  permissions. **Done 2026-05-06.**
- [x] T015 `templates/ingress.yaml` (gated on
  `values.ingress.enabled`) — host driven by a stagecraft-supplied
  per-tenant value. **Done 2026-05-06.**
- [x] T016 `helm lint platform/charts/tenant-hello/` clean.
  **Done 2026-05-06** — `helm v4.1.1` lint exits 0 (only an INFO
  about a missing chart icon, no warnings or errors). Skipped the
  `make ci` integration: there's no existing `ci-charts` recipe and
  the other charts (stagecraft, deployd-api, rauthy) aren't lint-gated
  in CI either; introducing a one-chart precedent would be inconsistent.
- [x] T017 `helm template` rendered cleanly with default values:
  ServiceAccount + Service + Deployment + (optional) Ingress, with the
  expected `oap.spec` label, non-root securityContext, `PORT` env
  injection, and `/healthz` probes. Skipped committing a golden fixture
  for the same consistency reason as T016.

**Checkpoint:** Helm chart lints, renders deterministically, matches
the C-clause obligations.

---

## Phase 2 — Stagecraft chart-per-tenant wiring

- [x] T020 Add `chartSelector` to
  `platform/services/stagecraft/api/deploy/chartSelector.ts` (placed
  under `api/deploy/` rather than `api/projects/` because chart
  resolution is part of the deploy invocation surface, not project
  creation). Pure function: input is a `{shape: TenantShape}`
  descriptor; output is a `{chart, version}` selection. tenant-hello
  is the first registered shape; unknown shapes throw rather than
  silently fall back. Unit test under `chartSelector.test.ts`
  (3 tests, all passing). **Done 2026-05-06.**
- [ ] T021 Stagecraft API surface change documented inline in the
  service's CLAUDE.md.
- [ ] T022 `deployd-api-rs` deploy invocation accepts the resolved chart
  name from stagecraft and applies it via Helm. **Deferred to
  Phase 2.b follow-up** — the orchestrator currently builds raw K8s
  objects via kube-rs (`platform/services/deployd-api-rs/src/k8s.rs`).
  Switching to Helm-driven application is a substantial refactor:
  decide between shelling `helm` CLI vs a Rust Helm library, ship
  chart files into the deployd-api image (or pull from a chart
  registry), map deployment-request fields to Helm values, and
  rework status reporting to track Helm release state. Not session-
  scoped and needs cluster validation. Tracked separately so this
  PR can land Phase 1 + chartSelector without holding on the
  refactor.
- [ ] T023 End-to-end happy path — depends on T022 + a live cluster.
  Captured under `execution/verification.md` when run.

**Checkpoint:** SC-002 first half (positive path) is evidenced.
Phase 2 is **partially complete** at this PR boundary: the chart
exists, stagecraft can resolve a chart name per project, but
deployd-api still builds raw K8s objects. Phase 2.b (T022) is the
remaining gate before SC-002 can be evidenced.

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
