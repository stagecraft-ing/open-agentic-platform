# Implementation Plan: Platform metrics stack (Prometheus + Grafana)

**Branch**: `196-platform-metrics-stack` | **Date**: 2026-06-03 | **Spec**: [`spec.md`](./spec.md)
**Input**: Feature specification from `specs/196-platform-metrics-stack/spec.md`

## Summary

Stand up a single `kube-prometheus-stack` Flux `HelmRelease` (Prometheus server
with `--web.enable-remote-write-receiver`, Grafana, node-exporter,
kube-state-metrics; Alertmanager disabled) that **receives** stagecraft's Encore
`remote_write`, **scrapes** the already-annotated deployd-api/Flux/ingress
targets, and **serves** Grafana behind Rauthy OIDC. Operator-facing,
control-plane only — no per-project/tenant granularity (that is spec 175, at the
application layer). Phasing is **single-spec, sequenced**: a self-contained data
plane lands first; Grafana + its manually-created OIDC client land second as one
atomic phase.

## Technical Context

**Runtime**: Kubernetes (Hetzner k3s prod; Azure AKS dev — deferred per FR-009)
**Deploy mechanism**: Flux v2 `HelmRelease` + `HelmRepository` (spec 151), reconciled like the existing infra releases — no imperative `helm`
**Chart**: `kube-prometheus-stack` (prometheus-community), version pinned in the HelmRelease
**Metrics ingestion**: hybrid — Encore `remote_write` (push) + Prometheus scrape (pull). Encore's `prometheus` exporter has only `type`/`collection_interval`/`remote_write_url` and no auth field, so the receiver is reachable **only** over an in-cluster NetworkPolicy flow, never at ingress
**Identity**: Rauthy OIDC (spec 106), runtime dependency only. Grafana's OIDC **client is created manually** in the Rauthy admin UI per the documented `[manual]` convention (`platform/infra/hetzner/.env.example`) — 196 does not touch `seed-rauthy.mjs`
**Persistence**: PVCs for Prometheus TSDB + Grafana state
**Testing**: post-deploy success criteria SC-001…SC-007 (operator-observable: series present, scrape targets up, OIDC role mapping, default-deny intact, Alertmanager absent + rules pruned)

## Constitution Check

- **Spec-first (III)** — spec.md authored and locked before any chart/gitops file is created (CONST-005); this plan descends from it. ✓
- **Markdown truth / compiler JSON (I, II)** — the spec is markdown; the HelmRelease/NetworkPolicy/values are *deployment manifests* (machine config), not authored platform truth, so they are not a standalone-YAML-authoring violation. ✓
- **Spec/code coherence (CONST-005)** — 196 owns its forward paths via `establishes`/`refines` (declared at implementation per V-023); no drift engineered. ✓
- **No new identity-subsystem ownership** — Grafana client is manual; 196 carries no `seed-rauthy.mjs` edge, hence no co-authorship/collision surface. ✓

## Project Structure

```text
specs/196-platform-metrics-stack/
├── spec.md            # locked contract
├── plan.md            # this file
└── tasks.md           # NOT created here (separate step)

# Created at IMPLEMENTATION (establishes/refines edges land then, per V-023):
platform/gitops/clusters/hetzner-prod/infrastructure/monitoring.yaml      # HelmRepository + HelmRelease (establishes)
platform/k8s/policies/monitoring/networkpolicy-monitoring.yaml            # monitoring-ns ingress (establishes)
platform/k8s/policies/namespace-baseline/networkpolicy-allow-metrics-egress.yaml  # stagecraft-system egress (establishes)
platform/services/stagecraft/infra.config.hetzner.json                   # + metrics block (refines)
platform/services/stagecraft/infra.config.json                           # Azure: stays null, FR-009 (refines)
```

**Structure Decision**: deployment artifacts live where the existing
control-plane lives — gitops infrastructure for the HelmRelease (mirrors
reflector/cert-manager/ingress-nginx/rauthy), `k8s/policies/` for the
NetworkPolicies, and the stagecraft `infra.config` for the Encore metrics block.
Grafana's OIDC + dashboard config is HelmRelease `values`, owned by 196.

## Phasing — single-spec, sequenced

**Phase 1 — data plane (no UI, no auth surface).** Implements FR-001, FR-002,
FR-003, FR-005, FR-006, FR-007, FR-010, FR-011 (and FR-009's *declaration*).
- HelmRelease: Prometheus server + node-exporter + kube-state-metrics, remote-write receiver on, Alertmanager **disabled** + bundled rules pruned (FR-008), retention/PVC per defaults below.
- Scrape config for deployd-api / Flux / ingress-nginx (FR-003).
- `infra.config.hetzner.json` gains the `metrics` block → literal in-cluster `remote_write_url` (FR-001).
- The two additive NetworkPolicy objects across `stagecraft-system` + `monitoring` (FR-006).
- **Exit:** SC-001 (stagecraft series present), SC-002 (scrape targets up), SC-005 (default-deny intact), SC-006 (Alertmanager absent + rules pruned), SC-007 (Flux-reconciled, CRDs pinned, PVC-bounded).

**Phase 2 — Grafana + OIDC (atomic).** Implements FR-004. **Must be one phase**:
SC-003 forbids a local Grafana password, so Grafana cannot come up before its
OIDC client exists.
- **Manual step (operator):** create the `grafana` OIDC client in the Rauthy admin UI (redirect `https://grafana.<DOMAIN>/login/generic_oauth`, `authorization_code`), capture `GRAFANA_OIDC_CLIENT_ID`/`_SECRET` into the env/secret set, re-run `setup.sh` — exactly the documented `[manual]` OIDC-client flow.
- **HelmRelease values (196-owned):** Grafana `generic_oauth` with `client_id`/`client_secret`/issuer + `role_attribute_path` mapping the Rauthy groups claim → Admin/Editor/Viewer; Grafana ingress at `grafana.<DOMAIN>`; the Grafana ingress NetworkPolicy.
- **Exit:** SC-003 (group→role mapping verified), SC-004 (`git diff` shows zero `stagecraft/api/**` change).

Phase 1 is independent and can land alone; Phase 2 depends only on Phase 1 (the
stack must exist) plus the manual client.

## Concrete values (proposed)

- **Grafana host**: `grafana.<DOMAIN>` (follows the `auth.`/`stagecraft.` convention; `<DOMAIN>` from `.env.example`)
- **OIDC client_id**: `grafana`
- **redirect_uri**: `https://grafana.<DOMAIN>/login/generic_oauth`
- **Prometheus retention**: `15d`, TSDB PVC `20Gi` *(tunable; pre-alpha single-cluster default; revisit at the FR-009 promotion trigger per FR-010)*
- **Grafana PVC**: `2Gi` *(dashboards provisioned as code; state only)*
- **Encore `collection_interval`**: `60s` — bounds remote_write volume; never sub-`15s` (FR-001)

## Open decision — yours, not a default

- **Rauthy-group → Grafana-role map (access policy).** Proposed starting point,
  to confirm or override: group `platform-admin` → **Admin**; `platform-operator`
  → **Editor**; any other authenticated Rauthy user → **Viewer** (or no access).
  This decides who holds Admin on the observability plane.
  **Hard Phase-2 precondition:** this map MUST be resolved (group names pinned)
  **before** the Phase-2 implementation PR opens — SC-003 is unverifiable until
  then, and it does not ride inside the implementation PR.

## Complexity Tracking

| Item | Why needed | Note |
|------|-----------|------|
| Prometheus Operator CRDs (cluster-wide) | kube-prometheus-stack ships them; `ServiceMonitor`/`PrometheusRule` are the declarative GitOps fit | New governed dependency surface (FR-007); version pinned in the HelmRelease |
| Unauthenticated remote_write receiver | Encore's exporter has no auth field | Contained by NetworkPolicy (FR-006); never exposed at ingress |

## Out of scope (this plan)

- `tasks.md` (separate step), alerting/SLO rules (later spec), logs + tracing
  pillars (later specs), Azure implementation (FR-009 deferred-null).
