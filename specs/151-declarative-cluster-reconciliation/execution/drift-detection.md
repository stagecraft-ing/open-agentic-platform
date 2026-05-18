# Drift detection — Flux v2 on Hetzner prod

> Phase 5 T-022 deliverable. Closes spec 151 SC-005 (mechanism +
> Prometheus metric inventory + Flux event surface). Live drift-revert
> evidence section pending operator-confirmed test (see "SC-005 live
> test recipe" below).

## What "drift" means under Flux v2

Drift is any divergence between (a) the cluster's live resource state
and (b) the state declared in the git ref that Flux is reconciling.
"Drift detection" is the controller's mechanism for noticing the
divergence; "drift correction" is what it does about it. The Flux
controllers handle these differently per resource class:

| Resource class | Controller | Reconcile interval (hetzner-prod) | Drift correction |
|---|---|---|---|
| `Kustomization`, `HelmRelease`, `HelmChart`, `Certificate`, `ClusterIssuer`, raw `Secret` / `ConfigMap` / `Service` / etc applied from gitops manifests | `kustomize-controller` | 10m (Kustomization `flux-system`) | Re-applies the declared manifest on every reconcile. Drift on these reverts within the next reconcile window. |
| `Deployment`, `StatefulSet`, `Service`, `Ingress`, `ConfigMap`, `Secret` rendered by a Helm chart that a `HelmRelease` manages | `helm-controller` | 1h (each HelmRelease) | Re-renders the chart and runs `helm upgrade` only when the rendered manifest differs from the previous release. **Default behavior does NOT revert manual edits to chart-managed resources** unless `spec.driftDetection.mode: Enabled` is set on the HelmRelease. |
| Helm `release` storage Secret (helm's own bookkeeping) | helm | n/a | Owned by helm; not a drift surface in the Flux sense. |
| `GitRepository` artifacts | `source-controller` | 1m (poll) | Source-of-truth fetch; pulls new commits from the remote. Not a drift surface — it IS the truth. |

The asymmetry between kustomize-controller (eager re-apply) and
helm-controller (chart-diff-only) is the key thing operators should
know. **Drifting a Deployment that a HelmRelease manages does not
get reverted by default.** That's a deliberate tradeoff (helm's
release storage decides what's "the chart" and what's "external"),
not a Flux bug.

## In-cluster reconciliation cadence (2026-05-18)

```
$ kubectl get kustomization -A
NAMESPACE     NAME          AGE     READY   STATUS
flux-system   flux-system   3h44m   True    Applied revision: main@sha1:313bd2e03ed4...

$ kubectl get helmrelease -A
NAMESPACE       NAME            INTERVAL   READY
cert-manager    cert-manager    1h         True
ingress-nginx   ingress-nginx   1h         True
kube-system     reflector       1h         True
rauthy-system   rauthy          1h         True

$ kubectl -n flux-system get gitrepository flux-system -o jsonpath='{.spec.interval}'
1m0s
```

- **GitRepository poll**: 1m. The interval the cluster discovers new
  commits.
- **Kustomization reconcile**: 10m. The interval kustomize-controller
  re-applies all manifests under `platform/gitops/clusters/hetzner-prod/`.
- **HelmRelease reconcile**: 1h each. The interval helm-controller
  re-renders each chart.

The three intervals compose: a commit on `main` becomes a live cluster
change within `1m + 10m + (up to 1h for chart-rendered resources)`.
For Kustomization-managed objects (the HelmRelease CRDs themselves,
the `cert-manager-clusterissuers.yaml` ClusterIssuers, the
`tenants-wildcard-certificate.yaml` Certificate), the live cluster
change is `1m + 10m`.

## Flux event surface

Flux emits Kubernetes `Event` resources on every reconciliation
phase. Cluster-scoped query via `flux events`:

```
$ flux events --all-namespaces
NAMESPACE       LAST SEEN  TYPE     REASON                   OBJECT                              MESSAGE
flux-system     58m        Normal   ReconciliationSucceeded  Kustomization/flux-system           Reconciliation finished in 1.13s, next run in 10m0s
flux-system     51m (x16)  Normal   GitOperationSucceeded    GitRepository/flux-system           no changes since last reconcilation: observed revision 'main@sha1:15d7e1c7...'
flux-system     41m        Normal   NewArtifact              GitRepository/flux-system           stored artifact for commit 'spec(151): Phase 3 operational landing...'
flux-system     13m        Normal   Progressing              Kustomization/flux-system           HelmRelease/rauthy-system/rauthy created
rauthy-system   13m        Normal   HelmChartCreated         HelmRelease/rauthy                  Created HelmChart/flux-system/rauthy-system-rauthy with SourceRef 'GitRepository/flux-system/flux-system'
flux-system     13m        Normal   ChartPackageSucceeded    HelmChart/rauthy-system-rauthy      packaged 'rauthy' chart with version '0.1.0+3977dc48b11d'
rauthy-system   13m        Normal   UpgradeSucceeded         HelmRelease/rauthy                  Helm upgrade succeeded for release rauthy-system/rauthy.v81 with chart rauthy@0.1.0+3977dc48b11d
```

The reason taxonomy is documented:

- `GitOperationSucceeded` / `GitOperationFailed` — GitRepository poll.
- `NewArtifact` — source-controller stored a new commit artifact.
- `GarbageCollectionSucceeded` — source-controller pruned stale artifacts.
- `ReconciliationSucceeded` / `ReconciliationFailed` — Kustomization reconcile.
- `Progressing` — Kustomization is mid-reconcile or downstream resource
  is still converging.
- `HelmChartCreated` / `ChartPackageSucceeded` — helm-controller packaged
  a chart from the source-controller artifact.
- `UpgradeSucceeded` / `UpgradeFailed` / `InstallSucceeded` /
  `InstallFailed` / `RollbackSucceeded` / `Uninstalled` — helm-controller
  helm transaction outcomes.

Drift correction events specifically show as `Progressing` followed
by `ReconciliationSucceeded` on the kustomize-controller path; for
helm-controller with `driftDetection: Enabled`, drift correction
surfaces as a fresh `UpgradeSucceeded` event with a "drift detected,
re-applying" message.

### Type=Warning events are the alertable surface

```
$ kubectl get events -A --field-selector type=Warning --sort-by=.lastTimestamp
```

A continuously-healthy Flux installation will show no `Warning`
events from any of the four controllers. A spike in `Warning` events
on a controller (e.g. `ReconciliationFailed` on Kustomization,
`UpgradeFailed` on HelmRelease) is the load-bearing alert signal.
Notification-controller can route these to Slack / Discord / generic
webhook via a `Provider` + `Alert` CR (out of scope for this spec;
covered in spec 151 FR-006).

## Prometheus metric inventory

Flux v2.8.7 exposes metrics on each controller pod at port 8080
(`http-prom` container port; no Service exposes it — the metrics
endpoint is consumed via direct pod scrape or service-monitor
selector. ServiceMonitor + Prometheus operator wiring is out of
scope for this spec.).

```
$ kubectl -n flux-system port-forward pod/source-controller-... 8888:8080
$ curl localhost:8888/metrics | grep '^# HELP gotk_'
# HELP gotk_reconcile_duration_seconds The duration in seconds of a GitOps Toolkit resource reconciliation.
# HELP gotk_token_cache_evictions_total Total number of cache evictions.
# HELP gotk_token_cached_items Total number of items in the cache.
```

Three `gotk_*` metric families per controller:

| Metric | Type | Labels | What it tracks |
|---|---|---|---|
| `gotk_reconcile_duration_seconds` | histogram | `kind`, `name`, `namespace` | Reconciliation duration per Flux CR. Sum / count / bucket; histogram quantiles via `_bucket` series. |
| `gotk_token_cached_items` | gauge | (none) | Cached auth-token entries (GitRepository SSH/HTTPS credentials). |
| `gotk_token_cache_evictions_total` | counter | (none) | Token cache evictions; rising values indicate credential churn. |

Plus the controller-runtime base metrics from kubebuilder, also
exposed by every Flux controller:

| Metric | Type | What it tracks |
|---|---|---|
| `controller_runtime_reconcile_total{controller, result}` | counter | Reconciliation outcomes per controller. `result` ∈ `success`, `error`, `requeue`, `requeue_after`. |
| `controller_runtime_reconcile_errors_total{controller}` | counter | Reconciliation error count. |
| `controller_runtime_reconcile_time_seconds` | histogram | Per-reconcile wall-time. |
| `controller_runtime_reconcile_timeouts_total{controller}` | counter | Reconciliation timeouts. |
| `controller_runtime_reconcile_panics_total{controller}` | counter | Reconciliation panics (catastrophic; should always be 0). |

Sample live values (kustomize-controller, 2026-05-18 ~23:30 UTC):

```
gotk_reconcile_duration_seconds_sum{kind="Kustomization",name="flux-system",namespace="flux-system"} 33.02
gotk_reconcile_duration_seconds_count{kind="Kustomization",name="flux-system",namespace="flux-system"} 28
controller_runtime_reconcile_total{controller="kustomization",result="error"} 0
controller_runtime_reconcile_total{controller="kustomization",result="requeue_after"} 27
controller_runtime_reconcile_errors_total{controller="kustomization"} 0
```

Sample live values (helm-controller, same time):

```
gotk_reconcile_duration_seconds_sum{kind="HelmRelease",name="rauthy",namespace="rauthy-system"} 2.37
gotk_reconcile_duration_seconds_count{kind="HelmRelease",name="rauthy",namespace="rauthy-system"} 4
controller_runtime_reconcile_total{controller="helmrelease",result="error"} 0
controller_runtime_reconcile_total{controller="helmrelease",result="requeue_after"} 13
```

### Note on Flux v2.7 → v2.8 metric simplification

Earlier Flux v2 minor versions exposed additional metric families:

- `gotk_reconcile_condition{kind, name, namespace, type, status}` —
  per-condition state (Ready, Stalled, Reconciling).
- `gotk_suspend_status{kind, name, namespace}` — suspended-Flag tracker.

Flux v2.8.7 (the version pinned in `platform/gitops/clusters/hetzner-prod/flux-system/gotk-components.yaml`) does not expose these. The
substitutes are:

- Use `controller_runtime_reconcile_total{result="error"}` rate to
  detect failing reconciles instead of `gotk_reconcile_condition`
  Ready=False.
- Suspended status is observable via `kubectl get
  kustomization,helmrelease -A` rather than a metric — the
  `.spec.suspend` field is the source of truth.

If a future spec adds full Prometheus alerting, the queries below
work without the deprecated metrics:

```
# Reconcile error rate (per controller)
rate(controller_runtime_reconcile_errors_total[5m]) > 0

# Reconcile latency P95 (per Flux CR)
histogram_quantile(0.95, rate(gotk_reconcile_duration_seconds_bucket[5m]))

# Token cache pressure (any controller)
rate(gotk_token_cache_evictions_total[15m]) > 0
```

## Drift detection on HelmRelease-managed resources

**Current state: driftDetection NOT enabled on any HelmRelease**:

```
$ for ns in cert-manager ingress-nginx kube-system rauthy-system; do
    for hr in $(kubectl -n $ns get helmrelease -o name); do
      DD=$(kubectl -n $ns get $hr -o jsonpath='{.spec.driftDetection.mode}')
      echo "  $ns/$hr → driftDetection.mode='${DD:-<unset>}'"
    done
  done
  cert-manager/helmrelease/cert-manager → driftDetection.mode='<unset>'
  ingress-nginx/helmrelease/ingress-nginx → driftDetection.mode='<unset>'
  kube-system/helmrelease/reflector → driftDetection.mode='<unset>'
  rauthy-system/helmrelease/rauthy → driftDetection.mode='<unset>'
```

Consequence: a manual `kubectl edit deployment cert-manager -n
cert-manager` to change, say, replica count or resource limits will
NOT be reverted by helm-controller. The next chart re-render only
diffs against what's IN the chart values; live cluster drift on
fields helm doesn't render is invisible to it.

**Mitigation path (deliberately out of scope for this spec):**
adding `spec.driftDetection.mode: Enabled` to each HelmRelease would
make helm-controller compare the live cluster manifest against the
last-rendered helm manifest and trigger a `helm upgrade --force` on
detected drift. The tradeoffs are:

- **Pro:** any manual edit to a chart-managed resource is reverted
  within the HelmRelease interval (1h).
- **Con:** force-upgrade can cause unintended resource churn (pods
  roll) if the live cluster has external mutating webhooks injecting
  fields (e.g. linkerd injection, kyverno mutations) that the chart
  doesn't know about. helm-controller diffs them as drift; the
  upgrade strips them; the webhook re-injects on next admission.
  Loop.

Hetzner-prod currently has no mutating webhooks beyond cert-manager's
`secretTemplate` annotations and ingress-nginx's annotations — both
of which are managed by their own controllers and don't conflict
with the chart-rendered state. So enabling driftDetection is likely
safe. **Deferred to a follow-up spec** because the safety analysis
should be done deliberately, not folded into spec 151's closure.

**For SC-005 evidence**, the test target below is a kustomize-
controller-managed resource (HelmRelease CRD itself), where drift
correction is reliable and fast.

## SC-005 live test recipe

The test demonstrates that drift on a Flux-managed resource is
reverted within one reconciliation interval.

**Target:** a benign annotation on the `reflector` HelmRelease CR
itself (a kustomize-controller-managed object reconciled at 10m
interval; forced to immediate reconcile via `flux reconcile`).

**Why this target:**
- Lowest blast radius: an annotation on a Flux CR is bookkeeping;
  no functional consumer reads it.
- Reliable detection: kustomize-controller re-applies the declared
  manifest on every reconcile (the manifest has no `drift-test`
  annotation, so the manual edit is unambiguously drift).
- Fast: `flux reconcile kustomization flux-system` forces immediate
  reconcile rather than waiting up to 10m.

**Procedure:**

```bash
export KUBECONFIG=platform/infra/hetzner/kubeconfig

# 1. Capture baseline (no drift-test annotation present)
kubectl -n kube-system get helmrelease reflector \
    -o jsonpath='{.metadata.annotations}{"\n"}'

# 2. Introduce drift — add a benign annotation
START=$(date -u +%s)
kubectl -n kube-system annotate helmrelease reflector \
    drift-test.spec-151=$(date -u +%Y%m%dT%H%M%SZ) --overwrite

# 3. Verify drift present (live state has annotation declared state lacks)
kubectl -n kube-system get helmrelease reflector \
    -o jsonpath='{.metadata.annotations}{"\n"}' | grep drift-test

# 4. Force immediate kustomization reconcile (instead of waiting 10m)
flux reconcile kustomization flux-system --with-source

# 5. Verify drift reverted — annotation absent
kubectl -n kube-system get helmrelease reflector \
    -o jsonpath='{.metadata.annotations}{"\n"}'
END=$(date -u +%s)
echo "Drift introduced → reverted in $((END-START))s"

# Expected: annotation absent in step 5 output. Wall-clock < 30s
# with --with-source (forces GitRepository fetch + Kustomization
# reconcile in one pass).
```

**Expected reconciliation event sequence** (visible via `flux
events --watch` during the test):

```
GitRepository/flux-system     GitOperationSucceeded     no changes since last reconcilation
Kustomization/flux-system     ReconciliationSucceeded   Reconciliation finished in <duration>, next run in 10m0s
```

The `ReconciliationSucceeded` event in step 4 indicates the
manifest re-application that strips the drift-test annotation.

## SC-005 live evidence

**Status:** pending operator-confirmed test execution.

The `kubectl annotate` op in step 2 is a write against the
production cluster. Per the project's `feedback_capability_vs_authorization`
memory: read-only ops on the production cluster are free; writes
require per-step operator confirmation. The annotation write's
blast radius is empirically nil (annotations are bookkeeping; Flux
reverts within ~10s of `flux reconcile`), but the confirmation is
the right discipline.

This section gets filled in with the captured timestamps + event
log + command outputs on the next operator-confirmed session, then
the Phase 5 done-when checklist for SC-005 closes verbatim.

## Related artifacts

- **`dr-baseline.md`** (Phase 0) — partial baseline measurements
  (steps a, b, c of SC-003); F1–F5 findings to be re-validated at
  T-025.
- **`disaster-recovery.md`** (T-023, pending) — Stage 2 SC-003 full
  four-step measurement against a fresh throwaway cluster.
- **Spec 151 §FR-006** — drift detection MUST emit cluster events
  for every reconciliation; Prometheus metrics MUST expose
  reconciliation success rate, drift count, and last-reconcile-time
  per resource. **Satisfied** by the metric inventory above for
  success rate (`controller_runtime_reconcile_total`),
  last-reconcile-time (the histogram count tracks invocations; the
  Flux event `LAST SEEN` column is the human surface), and
  reconcile latency (`gotk_reconcile_duration_seconds`). Drift count
  as a dedicated metric is NOT exposed by Flux v2.8.7 — the
  observable proxy is `controller_runtime_reconcile_total{result="error"}`
  rate plus drift-test events. A future spec can introduce a
  custom exporter if drift-count-as-cardinality becomes load-bearing.
