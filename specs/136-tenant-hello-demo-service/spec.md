---
id: "136-tenant-hello-demo-service"
title: "Tenant-hello — stagecraft-deployable tenant reference service"
status: approved
implementation: in-progress
owner: bart
created: "2026-05-04"
approved: "2026-05-06"
kind: platform
risk: low
depends_on:
  - "087"  # unified-workspace-architecture (stagecraft as the web governance plane)
  - "078"  # platform-completion-plan (the broader platform-finishing context)
implements:
  - path: platform/services/tenant-hello
  - path: platform/charts/tenant-hello
  - path: platform/services/stagecraft/api/deploy/chartSelector.ts
  - path: platform/services/stagecraft/api/deploy/chartSelector.test.ts
summary: >
  Document `platform/services/tenant-hello` as the deliberately-minimal
  reference of what a project codebase looks like when stagecraft is
  responsible for deploying it. The express service itself is trivial; the
  spec exists to pin the contract a tenant codebase must honour to round-trip
  through stagecraft (containerised entrypoint, health probe, port-from-env,
  no privileged dependencies) and to call out the current gap between
  "tenant codebase exists" and "stagecraft can deploy it end-to-end."
---

# Feature Specification: Tenant-hello demo service

**Feature Branch**: `136-tenant-hello-demo-service`
**Created**: 2026-05-04
**Status**: Draft
**Input**: Repurpose the existing `platform/services/tenant-hello` express
service from "an example tenant" into a governed reference: this is the
codebase shape that stagecraft deployment is meant to handle, and
tenant-hello is the always-green fixture used to demonstrate (and
regression-test) the tenant-deploy pipeline.

## Purpose and charter

`tenant-hello` is an Express 4 service with a `/healthz` probe, a JSON
root, and a single-stage Dockerfile (`platform/services/tenant-hello/Dockerfile`).
It does not have business logic; that is deliberate. Its job in the OAP
tree is to answer one question: **what is the contract a project codebase
must honour for stagecraft to take it from "source on disk" to "running
behind the platform's ingress" without bespoke per-tenant work?**

This spec captures that contract and uses tenant-hello as its
canonical-reference fixture.

**Explicitly in scope:**

- The shape any tenant codebase must present so stagecraft + deployd-api
  can deploy it (containerisation contract, health/readiness contract,
  environment contract).
- Pinning tenant-hello as the always-green reference fixture for that
  contract — i.e. if stagecraft deployment of tenant-hello is broken,
  the tenant-deploy pipeline is broken.

**Explicitly out of scope:**

- Tenant-hello-specific business behaviour (there is none, by design).
- The stagecraft-side deployment UI / UX — covered by spec 087 and the
  factory-as-platform-feature thread (spec 108).
- Multi-service tenant codebases (tenant-hello stands in for the
  single-service shape; multi-service is a future expansion, not this
  spec).

## Current state vs intent

**Current state (2026-05-04):**

- `platform/services/tenant-hello/src/index.js` exists and serves the two
  routes documented above.
- `platform/services/tenant-hello/Dockerfile` builds a runnable image
  (`node:20-alpine` base, port 8080 exposed).
- There is **no** Helm chart for tenant-hello in `platform/charts/`
  (charts are present only for `stagecraft`, `deployd-api`, and `rauthy`).
- `deployd-api-rs` (`platform/services/deployd-api-rs/`) is the
  rust-axum K8s deployment orchestrator that would be responsible for
  applying a tenant chart through stagecraft.

**Intent (this spec's aspiration):**

The end-to-end loop *from a stagecraft user clicking "deploy" on a project
that points at this codebase, to a running pod responding on the cluster*
must work without per-tenant scaffolding. tenant-hello's role is to be
the smallest codebase that exercises that loop end-to-end.

The gap between current state and intent — namely, the missing
`platform/charts/tenant-hello/` Helm chart and the
stagecraft-side wiring that selects a chart per tenant — is real and is
declared as the implementation-in-progress surface of this spec.

## Tenant codebase contract *(normative)*

Any tenant codebase that wants to deploy through stagecraft MUST present:

- **C-001 (containerised entrypoint):** A `Dockerfile` at the codebase
  root that produces a runnable image with no privileged build steps.
  Multi-stage builds are allowed; the final image MUST run as a
  non-root user-by-policy (deferred to `platform/charts/` baseline values
  rather than enforced by tenant-hello itself).
- **C-002 (health probe):** An HTTP `GET /healthz` endpoint that returns
  HTTP 200 and a non-empty body when the service is ready to handle
  traffic. Used by the platform's k8s readiness probe.
- **C-003 (port from environment):** The service MUST bind to the port
  specified by the `PORT` environment variable. A documented default
  (tenant-hello uses `8080`) is fine for local development; the platform
  injects `PORT` at deploy time.
- **C-004 (stateless or externalised state):** The service image MUST be
  treated as ephemeral. Any persistent state lives in
  platform-managed backing services (PostgreSQL / object store /
  rauthy session store), not on the pod's local disk.
- **C-005 (declared dependencies):** Every runtime dependency must be
  installable via the codebase's package manifest (`package.json`,
  `Cargo.toml`, `pyproject.toml`, etc.). Vendored binaries and out-of-band
  install steps fall outside this spec's reference.

tenant-hello satisfies C-001, C-002, C-003, and C-004 today; C-005
trivially holds since it has only one dependency (`express`).

## Functional Requirements *(MVP)*

- **FR-001:** `platform/services/tenant-hello/` MUST remain present as
  the canonical reference of the C-001…C-005 contract. Removing it
  requires either superseding this spec or providing a replacement
  reference fixture.
- **FR-002:** The service MUST expose `/healthz` returning 200 with a
  non-empty body and a JSON root response identifying itself, so the
  pipeline can assert "request reached the pod" deterministically.
- **FR-003:** The service MUST honour the `PORT` env var with a documented
  default for local dev, matching the contract's C-003 requirement.
- **FR-004:** A future deliverable in this spec's implementation plan is
  the `platform/charts/tenant-hello/` Helm chart and the stagecraft-side
  wiring needed to deploy a project bound to this reference. Until that
  ships, `implementation: in-progress` is honest.
- **FR-005:** The `oap.spec` field in
  `platform/services/tenant-hello/package.json` MUST point at this spec
  (`136-tenant-hello-demo-service`), so the codebase-indexer's spec/code
  coupling sees tenant-hello as governed code, not orphaned.

## Success Criteria

- **SC-001:** A reader of this spec can identify the C-001…C-005 contract
  any tenant codebase must honour without reading any other spec.
- **SC-002:** When the stagecraft tenant-deploy loop ships (FR-004),
  invoking it against `platform/services/tenant-hello/` deploys a running
  pod whose `/healthz` returns 200 — and the same pipeline run against a
  codebase that *violates* one of C-001…C-005 fails with a localised
  error, not a generic platform crash.
- **SC-003:** `codebase-indexer compile` lists
  `platform/services/tenant-hello` against this spec under Layer 2
  traceability (no longer in the untraced/orphaned columns).

## Clarifications

### Session 2026-05-04

- The user-stated framing for this spec is *"what stagecraft deployment
  should enable us to perform when it comes to the codebase of the
  project."* tenant-hello is therefore not a feature in its own right;
  it is the smallest possible witness of the platform's tenant-deploy
  obligation. The contract section (C-001…C-005) is the actual content;
  the express service is the fixture.
- This spec is filed as `draft` rather than `approved` because the
  contract clauses (C-001…C-005) are first authoring and benefit from
  one reviewer pass before lock-in. The fixture itself is unchanged.
