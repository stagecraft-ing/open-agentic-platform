---
id: "196-platform-metrics-stack"
slug: platform-metrics-stack
title: "Platform metrics stack — Prometheus + Grafana for the platform control plane"
status: draft
implementation: pending
owner: bart
created: "2026-06-02"
kind: platform
domain: platform
risk: medium
depends_on:
  - "106"  # rauthy-native-oidc-and-membership — runtime dependency: Grafana authenticates via Rauthy OIDC (client created manually; see FR-004)
  - "151"  # declarative-cluster-reconciliation — the Hetzner deploy mechanism (Flux HelmRelease) this rides on
  - "078"  # platform-completion-plan — stagecraft deployment + infra.config surface this refines
code_aliases: ["METRICS_STACK", "PROMETHEUS_REMOTE_WRITE", "GRAFANA_OIDC"]
# establishes: (deferred to the implementation PR — NOT declared in this draft)
#   The implementation PR creates and `establishes:`-claims at least THREE files,
#   all forward-described in the body but NOT listed here:
#     1. gitops monitoring HelmRelease
#        platform/gitops/clusters/hetzner-prod/infrastructure/monitoring.yaml
#     2. monitoring-ns NetworkPolicy: Prometheus ingress ← stagecraft-system
#        (remote_write) ONLY + Grafana ingress ← ingress-nginx; monitoring
#        egress → scrape-target metrics ports (pull). See FR-006.
#        platform/k8s/policies/monitoring/networkpolicy-monitoring.yaml
#     3. stagecraft-system egress NetworkPolicy (remote_write → monitoring),
#        plus target-namespace ingress-from-monitoring allows (FR-006 (b)).
#        platform/k8s/policies/namespace-baseline/networkpolicy-allow-metrics-egress.yaml
#   FR-006's flows span multiple namespaces, so the policy is ≥2 objects, not one. The spec-compiler existence-checks `kind: file` units
#   (V-023 errors on a missing path) and CONST-005 forbids creating these before
#   this body locks, so their `establishes:` edges land in the implementation PR
#   that creates them — exactly when the coupling gate (spec 127) wants the
#   spec↔code link.
# refines: (also deferred to the implementation PR — same principle as the
#   establishes edges above: an edge lands in the PR that touches the code, not
#   this spec-only filing). 196 WILL refine the `metrics-export` aspect of
#   platform/services/stagecraft/infra.config.hetzner.json (adding its `metrics`
#   block, FR-001), but this filing PR does NOT edit that file — declaring the
#   edge now would claim an untouched file. The edge is added at implementation,
#   when the coupling gate wants the spec↔code link. (infra.config.json / Azure
#   is never refined by 196 — FR-009 deferred-null.) The filing PR's only live
#   code-path edge is the mechanical featuregraph-golden `extends` below.
extends:
  # Mechanical featuregraph-golden refresh: appending spec 196 to the corpus
  # shifts the golden fingerprint. No semantic change to spec 034's claims.
  # Same precedent as spec 194 (PR #277), 193 (PR #276), 187, 183.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
# references: (intentionally none) — 196 owns NO identity-subsystem code path.
#   The Grafana OIDC client is created manually in the Rauthy admin UI (FR-004),
#   following the documented [manual] OIDC-client convention in
#   platform/infra/hetzner/.env.example ("[manual] OIDC clients … create in
#   Rauthy admin … re-run ./setup.sh"), exactly like stagecraft-server and the
#   GitHub/Google upstream providers. 196's only identity relationship is the
#   runtime dependency carried by depends_on: 106 — no owned edge into the
#   seeder or the Rauthy chart, hence no co-authorship and no collision surface.
summary: >
  Stand up a Prometheus + Grafana metrics stack for the platform control plane.
  Stagecraft (Encore.ts) already auto-instruments every API, PubSub, cron, and
  DB call but emits nowhere; deployd-api, Flux, and ingress-nginx already carry
  scrape annotations nobody consumes. This spec wires a full kube-prometheus-stack
  (Prometheus server + Grafana + node-exporter + kube-state-metrics) that
  RECEIVES stagecraft's Prometheus remote_write, SCRAPES the annotated pull
  targets, and SERVES Grafana — with Grafana fronted by native Rauthy OIDC.
  Metrics only: logs and distributed tracing (Encore's other two pillars) are
  out of scope and owned by later specs. Hetzner is implemented; the Azure
  binding is declared-but-deferred. Zero stagecraft application-code change.
---

# Feature Specification: Platform metrics stack (Prometheus + Grafana)

**Feature Branch**: `196-platform-metrics-stack`
**Created**: 2026-06-02
**Status**: Draft
**Input**: Wire Prometheus + Grafana into OAP. Stagecraft (Encore.ts) is
already capable of the integration; evaluate whether this is spec-worthy or a
de-facto infra-config change.

## Purpose and charter

The platform's thesis is *governed observability*. Today the platform observes
nothing: metrics are emitted into the void. This spec closes that gap for the
**metrics** pillar — and only that pillar.

The grain is deliberate. In Encore's own taxonomy, observability is three
pillars — **metrics**, **logging**, **distributed tracing** — and Encore emits
all three. A spec titled "observability-stack" that ships only metrics would
claim territory it does not deliver, forcing a future logs/traces effort to
either squat under a misleading title or supersede it. This spec is therefore
scoped and named **metrics-stack**; logs and traces are explicitly out of
scope (§Out of scope) and left to later specs.

## Current state vs intent

**What exists today:**

- **Stagecraft auto-instruments but emits nowhere.** Encore.ts instruments
  every API endpoint, PubSub topic, cron job, and DB query automatically — no
  application code is required. There is no `metrics` block in either
  `infra.config.json` or `infra.config.hetzner.json`, so nothing is exported.
- **Scrape targets emit but are unconsumed.** `deployd-api-rs` (axum), the Flux
  controllers (`prometheus.io/scrape: "true"` in `gotk-components.yaml`), and
  ingress-nginx already expose `/metrics`. Nothing scrapes them — the
  annotations describe an intent with no collector behind it.
- **The two stagecraft infra configs are byte-identical.** They sit on
  different deploy paths (`infra.config.json` → Azure/default
  `encore build docker`; `infra.config.hetzner.json` → the Hetzner build) and
  exist precisely so per-cloud infrastructure can diverge. They coincide only
  because no field has yet needed to differ.

**Intent:** a single Prometheus server that receives stagecraft's remote_write,
scrapes the annotated targets, and serves Grafana; Grafana behind Rauthy OIDC;
deployed declaratively via Flux on Hetzner. The first field to make the two
infra configs **diverge** is the `metrics` block this spec adds.

## Architecture — hybrid ingestion *(normative)*

Encore.ts metrics export is **push-only via Prometheus `remote_write`**. The
`prometheus` exporter block carries exactly three keys — `type`,
`collection_interval`, `remote_write_url` — and **no auth field** (unlike
`datadog`'s `api_key` or the GCP/AWS variants). Encore does **not** expose a
`/metrics` scrape endpoint. This forces a **hybrid ingestion topology**:

| Source | Path | Mechanism |
|---|---|---|
| stagecraft (Encore.ts) | **push** | Prometheus `remote_write` → in-cluster receiver |
| deployd-api-rs, Flux controllers, ingress-nginx, node-exporter, kube-state-metrics | **pull** | Prometheus scrape (annotations / ServiceMonitor) |

Because stagecraft pushes and the rest are scraped, the collector MUST be a
full **Prometheus server** running with `--web.enable-remote-write-receiver`,
which can simultaneously (a) receive remote_write, (b) scrape pull targets, and
(c) answer Grafana queries. Prometheus **agent mode** cannot satisfy this — it
is a scrape-and-forward shipper that neither receives remote_write nor serves
queries. The "slim agent + standalone Grafana" framing is therefore rejected as
infeasible against these requirements.

## User Scenarios & Testing

### User Story 1 — Operator sees stagecraft health (Priority: P1)

An operator opens Grafana, signs in with their Rauthy identity, and sees
stagecraft request rates, error rates, latency, PubSub backlog, and DB query
timings — without stagecraft having shipped a single line of metrics code.

**Why this priority**: this is the core gap. Stagecraft is the busiest service
and currently emits nothing observable.

**Independent Test**: deploy the stack on Hetzner, confirm stagecraft series
appear in Grafana within one `collection_interval` of a request.

**Acceptance Scenarios**:

1. **Given** the stack is deployed and stagecraft has the `metrics` block,
   **When** a request hits a stagecraft endpoint, **Then** its series is
   queryable in Prometheus and rendered in Grafana within ~`collection_interval`.

### User Story 2 — Previously-unconsumed scrape targets become visible (Priority: P1)

The deployd-api, Flux, and ingress-nginx metrics that have been emitted-but-
unconsumed are now scraped and dashboarded.

**Independent Test**: query a known deployd-api or Flux series in Grafana after
deploy; it returns data.

### User Story 3 — Identity-scoped Grafana access (Priority: P2)

A user in the Rauthy "viewer" group lands in Grafana as a Viewer; an "admin"
group user lands as Admin. No local Grafana passwords.

**Independent Test**: sign in as each group; confirm the mapped Grafana role.

### Edge Cases

- **remote_write receiver unreachable at stagecraft boot** — stagecraft MUST
  start and serve regardless; metrics export is best-effort, never a readiness
  dependency.
- **Default-deny blocks the push** — without an explicit egress allow,
  stagecraft's remote_write is dropped silently. FR-006 covers this.
- **Operator CRDs absent** — a `ServiceMonitor`/`PrometheusRule` applied before
  the Operator's CRDs exist fails. CRD install ordering is part of FR-007.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001 (stagecraft export, Hetzner):** `infra.config.hetzner.json` MUST
  gain a `metrics` block: `type: prometheus`, a bounded `collection_interval`
  (target ~`60s`; MUST NOT be sub-`15s`, which fans out abusive high-frequency
  remote_write), and a `remote_write_url` set to the **literal** in-cluster Prometheus
  remote_write endpoint (e.g. `http://<release>-prometheus.monitoring.svc.cluster.local:9090/api/v1/write`).
  A literal is correct here — the endpoint is stable cluster-internal DNS, not a
  secret, so no `$env` indirection and no new secret is introduced.

- **FR-002 (collector shape):** the collector MUST be a Prometheus **server**
  with `--web.enable-remote-write-receiver` enabled — not agent mode — so it
  serves all three roles (receive / scrape / query) per the Architecture §.

- **FR-003 (scrape the unconsumed targets):** Prometheus MUST scrape the
  already-annotated pull targets — deployd-api-rs, the Flux controllers,
  ingress-nginx — plus the stack-bundled node-exporter and kube-state-metrics.
  This closes the emitted-but-unconsumed gap, not just the stagecraft gap.

- **FR-004 (Grafana auth via Rauthy OIDC):** Grafana MUST authenticate via
  **native generic OIDC against Rauthy** (spec 106), mapping Rauthy groups to
  Grafana `Admin`/`Editor`/`Viewer` roles. Fronting Grafana with the existing
  `oauth2-proxy-gate` chart is explicitly rejected: it forces Grafana into
  proxy-auth/anonymous mode and forfeits clean role mapping. The Grafana OIDC
  client MUST be **created manually in the Rauthy admin UI**, following the
  documented `[manual]` OIDC-client convention
  (`platform/infra/hetzner/.env.example`: "[manual] OIDC clients … create in
  Rauthy admin … fill these in and re-run ./setup.sh") — exactly as
  `stagecraft-server`, the SPA/M2M clients, and the GitHub/Google upstream
  providers already are. Its `client_id`/`client_secret` are captured into the
  env/secret set; Grafana's HelmRelease consumes them plus the issuer to drive
  `generic_oauth`, with `role_attribute_path` mapping the Rauthy groups claim to
  Admin/Editor/Viewer. Grafana MUST be **OIDC-only** such that **no known or
  default credential reaches the admin API**: `auth.disable_login_form: true`
  hides the form but is insufficient alone — the built-in `admin` stays reachable
  via Basic-auth to `/api/*`. Closing it requires *also* disabling or randomizing
  the default admin (no known password) **and** `oauth_auto_login: true` (no
  login surface). 196 therefore touches **no** stagecraft code — and the seeder
  not at all; its only identity relationship is the runtime `depends_on: 106`
  (Relationships §).

- **FR-005 (declarative deploy, Hetzner):** the stack MUST deploy as a Flux
  `HelmRelease` under `platform/gitops/clusters/hetzner-prod/infrastructure/`,
  sourced from a `prometheus-community` **`HelmRepository`** — the same Flux
  source kind already used by cert-manager (jetstack), ingress-nginx, and
  reflector (emberstack); only rauthy uses an in-tree `GitRepository` chart. The
  reconciliation model (drift-reverted GitOps) is identical to every existing
  infra release per spec 151. No imperative `helm install`.

- **FR-006 (network policy, fail-safe):** the namespace default-deny posture
  MUST be preserved. This spec adds **additive** allow flows, in the directions
  the actual traffic takes:
  - **(a) remote_write (push):** `stagecraft-system` egress → Prometheus, and
    Prometheus ingress ← `stagecraft-system` **only**. The unauthenticated
    receiver's inbound surface is reachable from nowhere else (SC-008).
  - **(b) scrape (pull):** Prometheus *initiates* the connection — so
    `monitoring` egress → each scrape-target namespace's metrics port, and each
    target namespace ingress ← `monitoring`. There is **no** ingress to
    Prometheus from scrape targets; that would expose the receiver port
    (`9090`) to `flux-system`/deployd-api and undercut SC-008.
  - **(c) Grafana UI:** Grafana ingress ← ingress-nginx.
  The global default-deny is not weakened; only these named flows open. Because
  they span `stagecraft-system`, `monitoring`, and the target namespaces,
  implementation creates **multiple NetworkPolicy objects across namespaces**
  (≥2 `establishes:` edges; see the deferred-establishes note in frontmatter).

- **FR-007 (Operator CRDs as a governed dependency surface):** adopting
  kube-prometheus-stack introduces the Prometheus Operator CRDs
  (`ServiceMonitor`, `PodMonitor`, `PrometheusRule`, `Probe`, …) as a new
  cluster-wide governed dependency. Their version MUST be pinned via the
  HelmRelease and their presence treated as a tracked dependency, not an
  incidental side effect. CRD install/upgrade ordering is part of the contract.

- **FR-008 (Alertmanager off; default rules pruned):** Alertmanager MUST be
  **disabled** in the initial landing, and the ~100 bundled default
  `PrometheusRule`s MUST be pruned/disabled rather than silently inherited.
  Alert routing and SLO rules are out of scope and tracked as a **separate spec**
  (§Out of scope) — a distinct capability, not a later FR appended to this locked
  spec. The spec takes a position so the chart does not decide by default.

- **FR-009 (Azure binding — DECLARED, DEFERRED-NULL):** the Azure cloud binding
  is part of this spec's contract surface, but `infra.config.json`'s `metrics`
  block remains **null** pending resolution. The binding's *existence* is
  declared now (so no future amendment must *add* it to a frozen registry); its
  *type* is deferred (the genuinely uncertain half).
  - **Reopening trigger (forcing function):** the Azure type resolves when
    stagecraft is first deployed to Azure with metrics required, **or** when
    Azure-dev is promoted to a long-lived/validated environment — whichever
    comes first. Absent a named trigger, "deferred" and "forgotten" are
    indistinguishable at registry-read time; this clause makes them distinct.
  - **Both-branch costs (preserved, not discarded):**
    - *Azure Monitor managed Prometheus* → **+1 follow-up FR**: an Entra-ID
      `remote_write` **auth-proxy sidecar** (managed-identity → bearer),
      required precisely because Encore's `prometheus` block has no auth field
      and cannot reach an Entra-gated endpoint on its own. Microsoft lock-in on
      the observability substrate.
    - *Self-hosted in-cluster on AKS* → **zero new machinery**: reuse the
      Hetzner mechanism verbatim; only the literal `remote_write_url` DNS
      differs.
  - **Decided vs anticipated:** the sovereignty/anti-lock-in disposition
    *predicts* the resolution is self-hosted-symmetric, but prediction is not a
    reason to freeze. The registry stays honest about decided-versus-anticipated;
    the lock-in argument is applied at implementation time, where it is
    load-bearing rather than presumed.

- **FR-010 (retention & persistence):** Prometheus and Grafana MUST have
  bounded retention backed by PVCs. The retention window and PVC sizes are a
  plan-time value, not frozen here; the *requirement* for bounded, persisted
  storage is. **Retention sizing MUST be revisited at the same long-lived /
  validated promotion trigger as FR-009**, so the pre-alpha defaults cannot
  silently persist into a production-grade deployment.

- **FR-011 (zero stagecraft application-code change — invariant):** this spec
  MUST NOT add or modify stagecraft application code. Instrumentation is
  Encore-native; the only change to the *running service* is the `infra.config`
  metrics block (FR-001). Any diff to `platform/services/stagecraft/api/**`
  attributed to this spec is a contract violation, proven by SC-004 via
  `git diff`. The Grafana OIDC client is created manually in Rauthy (FR-004), so
  196 touches **no** stagecraft file at all — not `api/**`, and not the
  `scripts/seed-rauthy.mjs` seeder. The only stagecraft-side artifact 196 edits
  is the `infra.config` metrics block; Grafana's own OIDC config lives in its
  HelmRelease values under gitops, which 196 owns.

### Key Entities

- **Metrics stack** — the kube-prometheus-stack HelmRelease: Prometheus server,
  Grafana, node-exporter, kube-state-metrics; Alertmanager disabled (FR-008).
- **Cloud binding** — a (cloud, infra.config file, metrics `type`, endpoint)
  tuple. Hetzner: `prometheus` / in-cluster literal. Azure: declared, null,
  deferred (FR-009).
- **Grafana OIDC client** — a Rauthy client + group→role mapping (FR-004).

## Success Criteria *(mandatory)*

- **SC-001:** stagecraft series are queryable in Prometheus and rendered in
  Grafana within ~one `collection_interval` of a request (remote_write path
  works).
- **SC-002:** at least one known series from each of deployd-api, Flux, and
  ingress-nginx is queryable post-deploy — the previously-unconsumed
  annotations are now consumed (scrape path works).
- **SC-003:** a Rauthy "viewer"-group user lands in Grafana as Viewer and an
  "admin"-group user as Admin (OIDC role mapping works); **and** Grafana's
  built-in admin API is **unreachable with any known/default credential** (not
  merely the HTML login form hidden) — proving OIDC is the only auth path (FR-004).
- **SC-004:** `git diff` for the implementing branch shows **zero** changes
  under `platform/services/stagecraft/api/**` — the stagecraft-side change is
  confined to its `infra.config` metrics block (proves FR-011).
- **SC-005:** an un-allowed egress from `stagecraft-system` is still dropped
  after deploy — the default-deny posture is intact, only the named flows are
  open (proves FR-006 did not globally weaken the policy).
- **SC-006:** post-deploy, Alertmanager pods are **absent** and the
  `PrometheusRule`s **installed by the kube-prometheus-stack release** are
  absent/disabled (the phase-1 allowed set from this release is ∅; alerting is a
  separate later spec; FR-008) — proving the prune held and the ~100 bundled
  defaults were not silently inherited. (Pre-existing rules from other tooling
  are out of scope.)
- **SC-007:** the stack is reconciled by Flux (no imperative `helm install` in
  the deploy path), the Prometheus Operator CRDs are present at the
  HelmRelease-pinned version, Prometheus + Grafana retention is PVC-bounded, and
  the deployed `collection_interval` honours the FR-001 floor (≥`15s`) — proving
  FR-005, FR-007, FR-010, and guarding FR-001's bound against later drift.
- **SC-008 (inbound isolation, both ports):** Prometheus's remote_write ingest
  port is reachable **only** from `stagecraft-system`, and Grafana's port
  **only** from ingress-nginx (FR-006 (a)/(c)). The negative test covers a
  generic namespace **and every namespace granted `monitoring`-egress under
  FR-006 (b)** (`flux-system`, deployd-api, ingress-nginx) — those scrape-target
  namespaces are the ones most likely to be wrongly granted the reverse inbound.
  This is the test closure for the spec's primary security risk (the
  unauthenticated receiver): FR-006 isolates *inbound*, not merely leaving egress
  un-weakened (SC-005).

## Out of scope (MVP)

- **Logging pillar** — Encore's structured-log export. A later spec owns it.
- **Distributed tracing pillar** — Encore's trace export. A later spec owns it.
- **Alerting** — Alertmanager routing + SLO `PrometheusRule`s. Disabled and
  pruned here (FR-008); a **separate, later spec** owns alert routing and SLOs.
  It is a *distinct capability* — unlike the Azure binding (FR-009), which stays
  *in* 196 because it is the same capability on another cloud (only a value
  resolves; the structure is stable).
- **Azure implementation** — declared (FR-009) but not implemented; type
  deferred to the FR-009 trigger.
- **AWS / GCP / DigitalOcean bindings** — no environment instantiates these
  yet; out of scope until those environments exist.

## Relationships

- **→ spec 106 (identity) — runtime dependency only, no owned edge.** Grafana
  authenticates via Rauthy OIDC (FR-004). The Grafana OIDC client is **created
  manually in the Rauthy admin UI** — the documented `[manual]` convention used
  for every other client (`stagecraft-server`, SPA, M2M) and the upstream
  GitHub/Google providers (`platform/infra/hetzner/.env.example`). 196 therefore
  edits **no** identity-subsystem code path — not the seeder, not the Rauthy
  chart — so its sole identity relationship is the runtime `depends_on: 106`.
  **Why there is no seeder co-authorship or collision surface here.** An earlier
  draft modeled the Grafana client as an additive `extends` of the shared seeder
  `seed-rauthy.mjs`, which would have dragged in a sectioning prerequisite
  (provisional spec 197). Investigation retired that path on two grounds: (1) a
  Grafana addition cannot be confined to one self-contained block — the seeder
  dispatches client config by per-`clientId` branching spread across both 106's
  and 107's functions, and it does not even *create* clients (it converges drift
  on manually-created ones); and (2) this is an **admin-only, single, fixed**
  client, exactly the manual-creation profile of `stagecraft-server` and the
  upstream providers. Routing it through the per-client seeder machinery was
  never warranted. The one trade-off — a manually-created client's
  redirect/flows/scope are not seeder-drift-protected — is low-value here: a
  broken Grafana login is immediately visible to the operators whose own tool it
  is. (The seeder's pre-existing 106/107 whole-file co-ownership is untouched by
  196 and out of its scope.)
- **→ spec 151 (GitOps).** The Hetzner deploy mechanism — a Flux HelmRelease
  reconciled like every other infra release (FR-005). `depends_on: ["151"]`.
- **refines** (deferred to implementation, like `establishes` — not declared in
  this filing PR) the `metrics-export` aspect of the Hetzner infra config: 196
  adds its `metrics` block (FR-001) at implementation, and the edge lands in that
  PR, not this spec-only one (declaring it now would claim an untouched file).
  The Azure config is never refined by 196 (FR-009 deferred-null).
- **establishes** (at implementation, not in this draft's frontmatter) the
  gitops monitoring HelmRelease and **≥2 NetworkPolicy objects across two
  namespaces** (monitoring-ns ingress + stagecraft-system egress; see FR-006 and
  the frontmatter deferred-establishes note). The compiler existence-checks
  `kind: file` units (V-023), so these edges land in the PR that creates the
  files — the same PR where the coupling gate wants the spec↔code link.
- **extends** spec 034's featuregraph golden (mechanical corpus-fingerprint
  refresh only).

## Risks

`risk: medium` is a **deliberate** rating, not a default. This spec stands up a
new authenticated UI into platform internals, an *unauthenticated* in-cluster
remote_write receiver, cluster-wide CRDs, an amendment to a default-deny
posture, and a new Rauthy-OIDC-gated admin UI (Grafana). Medium holds because every
item below carries an in-spec mitigation and the blast radius is a pre-alpha,
single-tenant control plane with **no production user data**. Revisit to `high`
the moment this stack observes an environment that carries tenant data.

- **Operator CRD surface (FR-007).** Cluster-wide CRDs are a standing
  dependency and an upgrade-coupling risk; pinned via HelmRelease.
- **Default-rule noise.** Inheriting ~100 bundled alert rules would create
  alert fatigue and an implicit, ungoverned alerting policy — mitigated by the
  FR-008 prune.
- **remote_write receiver is unauthenticated.** Encore's exporter has no auth
  field, so the in-cluster receiver MUST be reachable only via the named
  NetworkPolicy flow (FR-006) and never exposed at ingress.
- **Grafana as a new attack surface.** A new authenticated UI into platform
  internals; mitigated by Rauthy OIDC + role mapping (FR-004) rather than local
  auth.

## Why this spec is filed as `draft`

The cloud-binding decision (FR-009) and the FR-010 retention values are the
load-bearing choices that benefit from one reviewer pass before lock-in. (The
Grafana OIDC client is created manually in Rauthy — FR-004 / Relationships § —
so 196 owns no identity-subsystem code path and the question of seeder
co-authorship does not arise.) Until this body locks, the gitops monitoring
HelmRelease, the monitoring + stagecraft-system NetworkPolicy objects, and the
`infra.config` `metrics` blocks **MUST NOT be created** — the spec's body drives
the implementation, not the other way around (CONST-005). This filing PR
declares **no** code-path edge except the mechanical featuregraph-golden
`extends: 034`: both the `establishes` (gitops + NetworkPolicy) and the
`refines` (infra.config.hetzner.json) edges are deferred to the implementation
PR that touches those files — the consistent rule being that an edge lands in
the PR that edits the code, not this spec-only filing. (V-023 independently
forbids the not-yet-existent gitops/NetworkPolicy paths; the existing
infra.config is deferred for consistency, not because it must be.) The filing
PR therefore changes no claimed code path except the golden.

## Implementation scope — a plan-time decision

- **Implemented now (Hetzner):** FR-001, FR-002, FR-003, FR-004, FR-005,
  FR-006, FR-007, FR-008, FR-010, FR-011.
- **Declared, deferred (Azure):** FR-009 — binding declared, metrics block
  null, type resolved at the FR-009 trigger.
- **Out-of-band follow-ups:** alerting (Alertmanager + SLO rules), logging
  pillar, tracing pillar — each a separate spec.

`plan.md` decides phasing (single-spec sequenced vs. split), carries the
concrete Grafana OIDC client values (client_id, redirect_uri, and the
Rauthy-group → Grafana-role map) as the manual Rauthy-admin setup step plus the
Grafana HelmRelease OIDC config, and pins the FR-010 retention values. No
`compliance:` mapping is asserted here: an OWASP-ASI detection/monitoring
mapping may apply but is a plan-time determination, not a frontmatter claim
made on faith.
