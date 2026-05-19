# Spec 151 — Disaster Recovery Runbook (SC-003 Stage 2)

> Phase 5 T-023 deliverable. The Stage 2 end-state DR exercise rerun
> against a throwaway Hetzner cluster, WITH a populated
> `platform/gitops/clusters/hetzner-prod/` tree to reconcile to. Step
> (d) — convergence to declared state — is now structurally
> measurable (it was absent at Phase 0). Cross-checks per-step
> timings against the Phase 0 anchors in [`dr-baseline.md`](./dr-baseline.md).
>
> **This file is the runbook the operator follows during the session
> AND the record of the session's measurements.** Slots marked
> `_____` are filled in-session; per-step `Status:` cells update from
> `PENDING` to `✓` / `✗` as each step closes.

**Session date:** _____ (target: 2026-05-19 — single Hetzner operator window).
**Operator:** bart.
**Cluster name:** `oap-dr-stage2-throwaway` (distinct from production
`oap-hetzner`; created + torn down within the session).
**Scope:** SC-003 Stage 2 full four-step DR sequence (a → b → c → d),
SC-005 drift-revert is independently closed in [`drift-detection.md`](./drift-detection.md) §"SC-005 live evidence (2026-05-18)" and is NOT re-measured here.

---

## Pre-flight (before opening the session window)

The pre-flight catches the named obstacles BEFORE the cost clock
starts. Each item below is a binary check the operator does once at
the start; failures are resolved before step (a) begins.

| Check | Mechanism | Status |
|---|---|---|
| `HCLOUD_TOKEN` exported (with cluster-create + image-create scopes) | `echo "${HCLOUD_TOKEN:0:8}…"` non-empty | _____ |
| `GITHUB_TOKEN` PAT with `Contents:read+write` on `stagecraft-ing/open-agentic-platform` | `gh auth status` lists the token, scopes include `repo` | _____ |
| `stagecraft-ing` org-level Deploy Keys ENABLED (F6) | Settings → Repository policies → Deploy keys = on, OR fall back to `--token-auth` below | _____ |
| Bitwarden vault accessible (F1 — required even if laptop key is present, since this session measures the Bitwarden path) | `bw status` returns `unlocked` after session-key login; item id `bd954307-a326-4376-a8ed-b44e00985759` in `OAP` org, key content in **notes field** (per §Clarification 9 (b) 2026-05-18 amendment — free tier does not support attachments) | _____ |
| Workstation tools present | `for c in kubectl helm hetzner-k3s flux sops age hcloud bw; do command -v $c || echo MISSING $c; done` | _____ |
| Throwaway `cluster.yaml` ready | `platform/infra/hetzner/cluster.yaml` copied to `/tmp/oap-dr-stage2/cluster.yaml`; `cluster_name: oap-dr-stage2-throwaway` substituted; production `oap-hetzner` cluster MUST NOT be touched | _____ |
| K3s version pinned ≥1.33 (F4) | `grep '^k3s_version:' /tmp/oap-dr-stage2/cluster.yaml` reports v1.33.x — Flux 2.8.7's `flux check --pre` is a hard gate inside `flux bootstrap` (dr-baseline F4 amendment) | _____ |
| Instance type validated (F2) | The 2026-05-17/18 cx43 capacity weather still applies until proven otherwise. Decision recorded before the session: cx43 in fsn1 OR fallback to cax21 (ARM). See "F2 resolution" below. | _____ |

Pre-flight failures stop the session BEFORE step (a). Resolve the
named obstacle, then restart pre-flight from the top.

---

## Step (a) — SOPS recipient restoration (CLOSES F1)

**Pinned model:** spec.md §Clarification 9 + dr-baseline.md §Step (a).
The Bitwarden-unlock+extract sub-step is the dominant cost; this
session measures it end-to-end on the real vault item.

### (a.1) Bitwarden-unlock + extract — measured 2026-05-18 (F1 closed)

The session deliberately starts from a state where the laptop key
is NOT on disk, so the Bitwarden path is the one exercised. After
the session closes, the laptop key is restored to disk (the
production default).

**Stash discipline.** Use a timestamp-suffixed stash path (NOT a
fixed `.session-stash` name). `mv` is destructive: a fixed name lets
a second stash silently overwrite a still-populated previous stash,
which lost the laptop key from operator disk during the 2026-05-18
session (recovered via cluster `sops-age` Secret pull — see
dr-baseline.md F1 Amendment, sub-finding 2). Timestamp-suffix
prevents the failure mode.

```bash
# 0. Stash laptop key with unique-per-run name (avoids overwriting a prior stash)
STASH_PATH="$HOME/.config/sops/age/keys.txt.stash-$(date +%s)"
test -f ~/.config/sops/age/keys.txt && \
  mv ~/.config/sops/age/keys.txt "$STASH_PATH" && \
  echo "Stashed laptop key → $STASH_PATH"

# 1. START CLOCK
date -u +%s > /tmp/dr-stage2-step-a-start

# 2. Unlock Bitwarden (interactive; 2FA-prompted is the realistic path).
#    Run `bw unlock` interactively, then paste the printed
#    `export BW_SESSION="…"` line into the shell. The `--raw` capture
#    inside $(...) can corrupt the session value due to tty interaction;
#    interactive paste is the working pattern.
bw login           # if not already authenticated
bw unlock          # paste the printed BW_SESSION export line
bw status | jq -r .status   # expect: "unlocked"

# 3. Extract the notes-field content to the operator key file
#    (Item type doesn't matter: Login or Secure Note, the API surface
#    is the same; the `bd954307...` id is stable.)
ITEM_ID="bd954307-a326-4376-a8ed-b44e00985759"
mkdir -p ~/.config/sops/age
bw get item "$ITEM_ID" | jq -r '.notes' > ~/.config/sops/age/keys.txt
chmod 0600 ~/.config/sops/age/keys.txt

# 4. STOP CLOCK
date -u +%s > /tmp/dr-stage2-step-a-end
echo "Step (a) wall-clock: $(( $(cat /tmp/dr-stage2-step-a-end) - $(cat /tmp/dr-stage2-step-a-start) )) seconds"
```

| Sub-step | Wall-clock | dr-baseline anchor | Status |
|---|---|---|---|
| Bitwarden-unlock + extract end-to-end | ≤35 s (2026-05-18) | NOT MEASURED at Phase 0 (F1) | ✓ |
| `chmod 0600` + on-disk verification | <1 s | — | ✓ |
| **Step (a) total wall-clock** | ≤35 s | CLI portion 0.5s (dr-baseline.md §Step (a)) | ✓ |

**Threshold check (2026-05-18 outcome):** SC-003 step (a) ≤5 min →
≤35s measured → **F1 closes verbatim.** The Bitwarden custody choice
in Clarification #9 holds with ≥8× headroom. Dominant cost is the
master-password unlock; the CLI extraction is sub-second. See
dr-baseline.md F1 Amendment for the full closure record.

### (a.2) Verify the recipient key matches `.sops.yaml`

```bash
PUBKEY="$(age-keygen -y ~/.config/sops/age/keys.txt)"
echo "Derived pubkey: $PUBKEY"
# Expected: age1s8vhfm82rnqw8enq52c026qsxuxd6uh5ntd75wzdah39jws3cctsqp0ava
#           (the Bitwarden backup key — recipient #2 in .sops.yaml)
grep -F "$PUBKEY" "$(git rev-parse --show-toplevel)/.sops.yaml" \
  && echo "✓ recipient match — F1 closed" \
  || echo "✗ MISMATCH — vault holds a key not in .sops.yaml"
ls -l ~/.config/sops/age/keys.txt   # expect mode 0600, ~189 bytes
```

If `MISMATCH` — the vault holds a key that is no longer (or never
was) a `.sops.yaml` recipient. Stop the session and resolve before
step (b) — a mismatched recipient means `kustomize-controller` will
crash-loop on decryption regardless of how clean the rest of the
bootstrap is. Likely causes: vault holds an outdated key from a
prior rotation; the `.sops.yaml` recipients list was edited without
updating the vault entry; or the vault entry was created with a
different keygen invocation than the one whose pubkey is in
`.sops.yaml`.

### (a.3) Cluster-as-recovery-surface emergency path

If neither the laptop nor the Bitwarden backup is reachable (stash
overwrite, vault account locked, device failure mid-session) AND
the production cluster is still running, the operator can recover
the in-cluster private key directly:

```bash
export KUBECONFIG="$HOME/Dev2/open-agentic-platform/platform/infra/hetzner/kubeconfig"
kubectl -n flux-system get secret sops-age \
  -o jsonpath='{.data.age\.agekey}' | base64 -d > ~/.config/sops/age/keys.txt
chmod 0600 ~/.config/sops/age/keys.txt
age-keygen -y ~/.config/sops/age/keys.txt   # whichever recipient the cluster was bootstrapped with
```

This validates the dr-baseline F1 sub-finding (2) cluster-as-third-
recipient-store property. Use only as an emergency path — the
healthy operator flow keeps two independent recovery surfaces
(laptop + Bitwarden); the cluster is a third de-facto surface that
the spec's FR-007 didn't explicitly claim but the multi-recipient
mechanism enables.

---

## Step (b) — Hetzner cluster create (CLOSES F2)

**Mechanism:** `hetzner-k3s create --config /tmp/oap-dr-stage2/cluster.yaml`.

```bash
# 1. START CLOCK
date -u +%s > /tmp/dr-stage2-step-b-start

# 2. Create cluster (foreground; tee log for finding-citation evidence)
cd /tmp/oap-dr-stage2
hetzner-k3s create --config cluster.yaml 2>&1 | tee b-create.log

# 3. Capture master-ready timestamp (kubeconfig file mtime is the durable anchor)
stat -f '%m' kubeconfig    # epoch when hetzner-k3s wrote the file

# 4. STOP CLOCK after BOTH master AND ≥1 worker Ready
export KUBECONFIG=$(pwd)/kubeconfig
kubectl wait --for=condition=Ready node --selector='!node-role.kubernetes.io/master' --timeout=20m
date -u +%s > /tmp/dr-stage2-step-b-end
echo "Step (b) wall-clock: $(( $(cat /tmp/dr-stage2-step-b-end) - $(cat /tmp/dr-stage2-step-b-start) )) seconds"
```

| Sub-step | Wall-clock | dr-baseline anchor | Status |
|---|---|---|---|
| Master ready (kubeconfig written) | _____ s | 124s fsn1 (dr-baseline.md §Step (b)) — should be stable across sessions | _____ |
| Worker pool Ready (first non-master node) | _____ s | NEVER converged at Phase 0 (F2) | _____ |
| **Step (b) total wall-clock** | _____ s | (subset of ≤20 min SC-003) | _____ |

**Threshold check:** SC-003 step (b) ≤20 min.
- If ≤20 min on the production-shape `cx23` + `cx43`: **F2 closes
  verbatim** — the 2026-05-17/18 capacity weather lifted.
- If >20 min on cx43 placement: **F2 closes with instance_type
  swap** — record the swap (e.g. cx43 → cax21 ARM) and rationale in
  §"F2 resolution" below.
- If `hetzner-k3s` itself hangs on `delete` after a partial bootstrap
  (F3): fall back to the `hcloud`-direct teardown documented in
  §Teardown — `hcloud server delete` + `hcloud network delete` +
  `hcloud firewall delete` (~40s, dr-baseline.md §Step (b) Attempt 1).

### (b.1) K3s version verification (F4 gate)

```bash
kubectl version --output=yaml | grep gitVersion
# Expected: server version ≥ v1.33.0; matches the cluster.yaml pin
flux check --pre
# Expected: no version-mismatch warning (Flux 2.8.7 wants K8s ≥1.33)
```

If `flux check --pre` reports a version mismatch, the bootstrap will
abort at the pre-check inside `flux bootstrap`. Resolve by either
re-creating the cluster against a higher k3s version (preferred) or
pinning a Flux version with explicit support for the cluster's k3s.

| Check | Result | Status |
|---|---|---|
| `kubectl version` reports k3s ≥1.33 | _____ | _____ |
| `flux check --pre` clean (no warnings, no errors) | _____ | _____ |

---

## Step (c) — `flux bootstrap` (CLOSES F5, F6 prerequisite re-confirmed)

**Mechanism:** the same `flux bootstrap github` invocation that
`platform/infra/hetzner/setup.sh` lines 204-210 use — but against
the throwaway cluster, with `--owner=stagecraft-ing
--repo=open-agentic-platform --branch=main
--path=platform/gitops/clusters/hetzner-prod`.

The setup.sh wait-for-worker-Ready gate (F5 mechanical answer,
setup.sh:184) is the same kubectl-wait that step (b) closes on. Step
(c) begins immediately after step (b)'s wall-clock stops.

```bash
# 1. START CLOCK
date -u +%s > /tmp/dr-stage2-step-c-start

# 2. Bootstrap Flux against the populated gitops tree
flux bootstrap github \
  --owner=stagecraft-ing \
  --repo=open-agentic-platform \
  --branch=main \
  --path=platform/gitops/clusters/hetzner-prod \
  --personal=false \
  --network-policy=true \
  2>&1 | tee c-bootstrap.log

# 3. Materialise the sops-age Secret (the kubectl create line that stays
#    in setup.sh per T-021 / spec 153 ownership — NOT a 151-removable line)
kubectl create secret generic sops-age \
  --namespace=flux-system \
  --from-file=age.agekey="$HOME/.config/sops/age/keys.txt" \
  --dry-run=client -o yaml | kubectl apply -f -

# 4. STOP CLOCK when all four controllers Ready
kubectl -n flux-system wait --for=condition=Ready pod \
  -l 'app.kubernetes.io/part-of=flux' --timeout=5m
date -u +%s > /tmp/dr-stage2-step-c-end
echo "Step (c) wall-clock: $(( $(cat /tmp/dr-stage2-step-c-end) - $(cat /tmp/dr-stage2-step-c-start) )) seconds"
```

| Sub-step | Wall-clock | dr-baseline anchor | Status |
|---|---|---|---|
| `flux bootstrap` returns (deploy-key created OR `--token-auth` Secret stored) | _____ s | install-only 5m 20s at Phase 0; controllers Ready blocked by F5 | _____ |
| `sops-age` Secret applied | _____ s | (new — not in Phase 0) | _____ |
| All four Flux controllers Ready | _____ s | blocked by F5 at Phase 0; should be ≤90s on workers present | _____ |
| **Step (c) total wall-clock** | _____ s | (no numeric SC-003 threshold; informational) | _____ |

**F6 fallback path** — if `flux bootstrap` aborts with HTTP 422
`Deploy keys are disabled`, the org-level toggle is OFF. Two
resolutions:
1. **Toggle on at the org level** — Settings → Repository policies →
   Deploy keys. `flux bootstrap` is idempotent on the already-pushed
   components, so re-run the same invocation after the toggle flips.
2. **`flux bootstrap --token-auth`** — bypasses deploy-key creation
   by storing the operator's PAT as a `flux-system/flux-system`
   Secret. Trade-off: PAT lives in-cluster long-lived; rotation
   becomes operator burden. Documented as the fallback when the org
   toggle is not available.

**F6 status this session:** _____ (toggle was ON / toggle was
flipped mid-session / `--token-auth` fallback was used)

---

## Step (d) — Cluster convergence to declared state (NEW vs Phase 0)

Step (d) was structurally absent at Phase 0 (no `platform/gitops/`
tree to reconcile against). With the tree populated by Phases 1–4,
step (d) is the measurement that confirms or refutes the
end-to-end SC-003 budget.

**What "converged" means here:** every `Kustomization` under the
gitops path is `READY: True`, AND every `HelmRelease` claimed by
those Kustomizations is `READY: True`, AND no `Warning` events
remain on any Flux controller. The Kustomization reconcile interval
is 10 min (drift-detection.md §"In-cluster reconciliation cadence")
— step (d) measures the time from `flux bootstrap` completion to
the first all-green Kustomization-set, NOT the steady-state cadence.

```bash
# 1. START CLOCK (immediately after step (c) stops)
date -u +%s > /tmp/dr-stage2-step-d-start

# 2. Force-reconcile the top-level flux-system Kustomization to pull
#    the gitops tree without waiting for the 10-min cadence
flux reconcile kustomization flux-system --with-source

# 3. Wait for the gitops Kustomizations to converge.
#    Production tree has one top-level (`flux-system`) + the cluster's
#    `infrastructure` + `manifests` Kustomizations that flux-system
#    composes (see platform/gitops/clusters/hetzner-prod/ tree).
kubectl wait --for=condition=Ready kustomization \
  --all --all-namespaces --timeout=30m

# 4. Wait for the HelmReleases (cert-manager, ingress-nginx, reflector,
#    rauthy) to converge. HelmRelease reconcile is 1h interval; the
#    initial install does not wait for the interval.
kubectl wait --for=condition=Ready helmrelease \
  --all --all-namespaces --timeout=30m

# 5. STOP CLOCK
date -u +%s > /tmp/dr-stage2-step-d-end
echo "Step (d) wall-clock: $(( $(cat /tmp/dr-stage2-step-d-end) - $(cat /tmp/dr-stage2-step-d-start) )) seconds"
```

| Resource | Expected READY | Wall-clock | Status |
|---|---|---|---|
| `kustomization/flux-system` (root) | True | _____ s | _____ |
| `helmrelease/cert-manager/cert-manager` | True | _____ s | _____ |
| `helmrelease/ingress-nginx/ingress-nginx` | True | _____ s | _____ |
| `helmrelease/kube-system/reflector` | True | _____ s | _____ |
| `helmrelease/rauthy-system/rauthy` | True | _____ s | _____ |
| `certificate/tenants-wildcard-certificate` issued | Ready | _____ s | _____ |
| `clusterissuer/letsencrypt-prod` Ready | True | _____ s | _____ |
| **Step (d) total wall-clock** | — | _____ s | _____ |

**Note on the `rauthy` HelmRelease specifically:** the rauthy chart
references the `rauthy-secrets` Secret by name (T-020). Until spec
153 lands SOPS-encrypted per-purpose Secrets, the throwaway-cluster
DR cannot fully reproduce the production rauthy because the
`rauthy-secrets` Secret is not part of the gitops tree. Two
acceptable Stage 2 outcomes:
1. **Skip rauthy HelmRelease convergence** for this DR exercise —
   measure step (d) against the four other infrastructure
   HelmReleases (cert-manager, ingress-nginx, reflector,
   manifests/cert-manager-clusterissuers, manifests/tenants-wildcard-
   certificate). Record the limitation in §"F2/F1 resolution"
   below as "rauthy convergence deferred to spec 153 close".
2. **Imperatively materialise `rauthy-secrets`** against the
   throwaway cluster using the same setup.sh logic that production
   uses today (the `kubectl create secret` line that T-021 leaves
   in place pending spec 153). The DR exercise then measures all
   five HelmReleases.

The decision is operational: option (1) cleanly separates spec 151
closure from spec 153 work; option (2) measures the full bundle at
the cost of mixing the per-purpose-Secret materialisation timing
into step (d). **Pinned for this session: _____** (option 1 / option 2).

---

## SC-003 verdict (T-024 input)

```
Step (a) Bitwarden-unlock + extract:       _____ s   (threshold ≤5 min  / 300 s)
Step (b) cluster create (master + worker): _____ s   (threshold ≤20 min / 1200 s)
Step (c) flux bootstrap + controllers:     _____ s   (no numeric threshold; informational)
Step (d) gitops convergence:               _____ s   (no per-step threshold; budgeted ~5 min by bracket)
─────────────────────────────────────────────────────
Total operator wall-clock:                 _____ s
SC-003 30-min budget (1800 s):             ____ s under / over
```

**Apply the baseline-vs-target policy** (spec.md §SC-003):

- **If total ≤30 min (1800 s):** SC-003 closes verbatim. Phase 5
  done-when satisfies SC-003 (Stage 2).
- **If total >30 min:** the same commit that records this Stage 2
  measurement MUST amend SC-003 in spec.md with all three of:
  (a) the measured number, (b) the identified dominant cost (e.g.
  "Hetzner provisioning at ~25min", "Bitwarden human-action at
  ~6min"), and (c) the future-shrink path or its explicit absence.
  Silent acceptance of a larger number is NOT acceptable. Stalling
  closure on a number argument the spec body should resolve is ALSO
  not acceptable. The amendment-with-rationale path is the
  disciplined middle.

**Verdict this session:** _____ (verbatim close / amendment-with-rationale)

If amendment-with-rationale is the verdict, the amendment text MUST
be drafted in the same PR that lands this disaster-recovery.md.
Template:

```
**Amendment (YYYY-MM-DD, Stage 2 measurement).** SC-003's 30-min
target was exceeded by _____ s in the Stage 2 DR exercise
recorded in `execution/disaster-recovery.md`. Dominant cost:
_____. Future-shrink path: _____ (or "no v1 shrink path; deferred
to a future spec exploring _____").
```

---

## F1 resolution (T-025 — Bitwarden flow measured)

**Phase 0 finding (dr-baseline.md §Step (a) Finding F1):** the
Bitwarden-unlock+extract sub-step was the dominant cost of step
(a) and was NOT exercised at Phase 0 (operator workstation lacked
`bw` CLI).

**Stage 2 measurement (2026-05-18):** ≤35 seconds end-to-end.

**Closure status: ✓ F1 closes verbatim.** ≤5 min threshold met
with ≥8× headroom. The realistic operator cold-start path —
master-password unlock + CLI item lookup + `bw get item | jq -r
.notes` extraction + file write + `chmod 0600` — measured well
inside the threshold. Three sub-findings closed alongside:

1. **Custody shape correction (spec amendment).** Phase 0 spec said
   "attachment `keys.txt`"; reality is "notes field" because the
   Bitwarden free tier does NOT support attachments. spec.md
   §Clarification 9 (b) amended in the same commit. See
   dr-baseline.md F1 Amendment block.
2. **Cluster as emergent third recovery surface.** Validated when a
   stash/restore bug lost the laptop key from operator disk;
   `kubectl -n flux-system get secret sops-age -o jsonpath` restored
   recipient #1 cleanly. The cluster's `sops-age` Secret is a
   readable third recipient store complementing
   laptop + Bitwarden. Stronger DR property than FR-007 explicitly
   claims; consistent with multi-recipient SOPS mechanism.
3. **Runbook stash-pattern bug fixed.** `mv keys.txt →
   keys.txt.session-stash` clobbers a populated previous stash;
   replaced with timestamp-suffix pattern (`keys.txt.stash-$(date
   +%s)`). The 2026-05-18 incident itself surfaced and validated the
   sub-finding (2) cluster-recovery property.

---

## F2 resolution (T-025 — cx43 capacity re-validated)

**Phase 0 finding (dr-baseline.md §Step (b) Finding F2):** cx43
capacity was constrained in both nbg1 and fsn1 on 2026-05-17/18.
Two consecutive DC attempts failed worker-pool placement.

**Stage 2 re-validation (this session):**
- **DC tried first:** _____ (nbg1 / fsn1 / hel1)
- **Instance type used:** _____ (cx43 unchanged / cax21 ARM swap /
  other)
- **Worker pool reached Ready:** _____ (yes / no — fallback applied)
- **Fallback escalation, if any:** _____

**Closure status:**
- _____ ✓ cx43 placed Ready in <20 min on the chosen DC — F2 closes
  verbatim, the 2026-05-17/18 weather was transient.
- _____ ✓ cx43 again constrained, swapped to cax21 (ARM) — F2 closes
  with instance_type swap. Rationale: ARM cax21 has healthier
  capacity in EU DCs; the master+worker shape stays the same; the
  CPU architecture flips. Update `platform/infra/hetzner/cluster.yaml`
  in a follow-up PR (NOT this PR — instance_type is operator
  decision, not a spec edit).
- _____ ✗ both cx43 and cax21 failed — F2 escalates. Document the
  weather pattern and propose a longer-term capacity strategy
  (multi-DC fallback list in setup.sh, or a different cloud for v1).

---

## Teardown

Hard-delete the throwaway cluster within the session — leaving
Hetzner resources running outside the cost window inflates the cost
record below.

```bash
# Preferred: in-tool teardown (works because we have a healthy kubeconfig)
hetzner-k3s delete --config /tmp/oap-dr-stage2/cluster.yaml

# Fallback (F3 path): if hetzner-k3s delete hangs (partial-bootstrap
# kubeconfig), hcloud-direct teardown is 40s wall-clock:
hcloud server list -o noheader -o columns=name | grep '^oap-dr-stage2' | xargs -n1 hcloud server delete
hcloud network list -o noheader -o columns=name | grep '^oap-dr-stage2' | xargs -n1 hcloud network delete
hcloud firewall list -o noheader -o columns=name | grep '^oap-dr-stage2' | xargs -n1 hcloud firewall delete

# Verify no residual resources
hcloud server list | grep oap-dr-stage2     # expect empty
hcloud network list | grep oap-dr-stage2    # expect empty
hcloud firewall list | grep oap-dr-stage2   # expect empty

# Local cleanup
rm -rf /tmp/oap-dr-stage2

# Restore laptop key from the most recent stash-* file. The timestamp
# suffix means we pick the latest by sort order; safer than a fixed
# name. If keys.txt currently exists (e.g. the backup key from step a),
# stash IT too with a fresh timestamp before restoring, so neither
# laptop nor backup is silently overwritten.
LATEST_STASH="$(ls -1 ~/.config/sops/age/keys.txt.stash-* 2>/dev/null | sort | tail -1)"
if [ -n "$LATEST_STASH" ]; then
  test -f ~/.config/sops/age/keys.txt && \
    mv ~/.config/sops/age/keys.txt "$HOME/.config/sops/age/keys.txt.stash-$(date +%s)-pre-restore"
  mv "$LATEST_STASH" ~/.config/sops/age/keys.txt
  echo "Laptop key restored from $LATEST_STASH"
  # Verify derived pubkey is the laptop recipient (recipient #1)
  age-keygen -y ~/.config/sops/age/keys.txt
fi

# Emergency fallback: if no stash file exists (or restore produces wrong
# pubkey), pull recipient #1 from the cluster's sops-age Secret per
# §Step (a.3) Cluster-as-recovery-surface emergency path. The cluster
# is a third de-facto recipient store the multi-recipient SOPS model
# enables; validated 2026-05-18 (see dr-baseline.md F1 Amendment sub-
# finding 2).
```

| Teardown step | Wall-clock | Status |
|---|---|---|
| `hetzner-k3s delete` OR hcloud-direct fallback | _____ s | _____ |
| Residual `hcloud server/network/firewall list` empty | (verify) | _____ |
| `~/.config/sops/age/keys.txt` restored to laptop default | (verify) | _____ |

---

## Cost record

Two throwaway-cluster cost components against `HCLOUD_TOKEN` in
`platform/infra/hetzner/.env`:

| Resource | Hourly | Wall-time present | Cost |
|---|---|---|---|
| Master `cx23` (~€0.0067/h) | _____ h | _____ € | _____ |
| Worker `cx43` or `cax21` (~€0.0341/h cx43; ~€0.0061/h cax21) | _____ h | _____ € | _____ |
| **Combined throwaway cost** | — | — | **_____ €** |

Budget envelope authorised at session start: ~€/30min (single
operator window). Actual: _____.

---

## Provenance

- Spec amendment (if any): `specs/151-declarative-cluster-reconciliation/spec.md`
  §SC-003 (drafted in the same commit as this file if Stage 2 ran
  over budget per the baseline-vs-target policy).
- Phase 0 anchors: [`dr-baseline.md`](./dr-baseline.md) — per-step
  timings cross-checked above.
- Drift-revert evidence (independent of Stage 2): [`drift-detection.md`](./drift-detection.md) §"SC-005 live evidence (2026-05-18)".
- Raw logs: `/tmp/oap-dr-stage2/{b-create.log, c-bootstrap.log,
  step-{a,b,c,d}-{start,end}}` — session-local, not committed. The
  measurement tables in this document are the durable record.
- Cluster: `oap-dr-stage2-throwaway` — created + torn down within
  the session; no residual Hetzner resources at session close
  (verified via `hcloud server list` / `network list` / `firewall
  list`).
- Production `oap-hetzner` cluster in `platform/infra/hetzner/kubeconfig`
  was NOT used or read by this session.

---

## Phase 5 done-when cross-check (T-024 + T-025 + T-026 inputs)

After this runbook records its measurements, the Phase 5 done-when
clauses (tasks.md §"Phase 5 done-when") are evaluated:

| Clause | Closed by | Status |
|---|---|---|
| SC-001 (Flux reconciles edits in ≤5 min) | Demonstrated by Phase 2 reflector / wildcard-cert migrations | (verified pre-session) |
| SC-002 (setup.sh <100 lines) | T-026 — final setup.sh shrink, gated on this DR closure | _____ |
| SC-003 Stage 2 (≤30 min OR amendment-with-rationale) | T-024 verdict above | _____ |
| SC-005 (drift reverts within one reconciliation) | [`drift-detection.md`](./drift-detection.md) (closed 2026-05-18) | ✓ |
| SC-007 (spec 137 Phase 6 evidence against Flux-reconciled cluster) | Phase 2 reflector + wildcard-cert in gitops tree | (verified pre-session) |
| F4 + F5 + F6 resolutions recorded | This document + dr-baseline.md amendments | _____ |
| Spec 151 frontmatter `implementation: complete` | Same commit as T-024 verdict + T-026 shrink | _____ |

When all clauses above are ✓, the spec 151 → 152 gate (plan.md)
opens for spec 152 implementation.
