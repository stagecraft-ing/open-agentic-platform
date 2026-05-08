#!/usr/bin/env bash
# =============================================================================
# OAP Hetzner — Infrastructure Bootstrap
# =============================================================================
# Installs cluster add-ons and dependencies. Called by setup.sh.
# Idempotent — safe to re-run.
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
KUBECONFIG_PATH="${KUBECONFIG:-$SCRIPT_DIR/kubeconfig}"

if [ ! -f "$KUBECONFIG_PATH" ]; then
  echo "ERROR: Kubeconfig not found at $KUBECONFIG_PATH"
  echo "Run 'hetzner-k3s create --config cluster.yaml' first."
  exit 1
fi

export KUBECONFIG="$KUBECONFIG_PATH"

info()  { printf '\033[1;34m==> %s\033[0m\n' "$*"; }
err()   { printf '\033[1;31mERROR: %s\033[0m\n' "$*" >&2; exit 1; }

# --- Install ingress-nginx (idempotent) ---
if helm status ingress-nginx -n ingress-nginx >/dev/null 2>&1; then
  info "ingress-nginx already installed, skipping"
else
  info "Installing ingress-nginx..."
  helm upgrade --install ingress-nginx ingress-nginx \
    --repo https://kubernetes.github.io/ingress-nginx \
    --namespace ingress-nginx --create-namespace \
    --set controller.kind=DaemonSet \
    --set controller.hostPort.enabled=true \
    --set controller.service.type=ClusterIP \
    --set controller.config.use-forwarded-headers='"true"' \
    --set controller.config.compute-full-forwarded-for='"true"' \
    --wait --timeout 180s
fi

# --- Install cert-manager (idempotent) ---
if helm status cert-manager -n cert-manager >/dev/null 2>&1; then
  info "cert-manager already installed, skipping"
else
  info "Installing cert-manager..."
  helm upgrade --install cert-manager cert-manager \
    --repo https://charts.jetstack.io \
    --namespace cert-manager --create-namespace \
    --version v1.19.3 \
    --set crds.enabled=true \
    --wait --timeout 180s
fi

kubectl wait --for=condition=Available deployment/cert-manager-webhook \
  -n cert-manager --timeout=120s

# --- ClusterIssuer (HTTP-01) ---
# Default issuer for all platform hosts (stagecraft.${DOMAIN},
# auth.${DOMAIN}, deploy.${DOMAIN}). HTTP-01 challenge resolves via the
# nginx ingress, which is fine for hosts whose A records point at the
# cluster IP.
#
# Issuer selection policy (decided 2026-05-08, spec 143 step 7):
#
#   - letsencrypt-prod (HTTP-01) is the DEFAULT for new ingresses,
#     including any future apex subdomain (foo.${DOMAIN}). Stagecraft,
#     Rauthy, deployd-api all keep this issuer.
#
#   - letsencrypt-dns01 (DNS-01 via Hetzner webhook) is reserved for
#     ingresses that NEED DNS-01 specifically. Today that's the spec-143
#     MinIO public ingress (minio.${DOMAIN}); the rationale recorded
#     there generalises to "high-renewal-cost outages preferred to be
#     avoided" (e.g. anything where a 90-day cert failure would block
#     end-user data flow). For new public hosts that don't fit that
#     bill, prefer letsencrypt-prod (simpler, no DNS API token
#     dependency).
#
#   - Wildcard certs (when the platform grows past three public
#     subdomains and the per-host cert ceremony becomes load-bearing)
#     will live on letsencrypt-dns01 — HTTP-01 cannot solve wildcards.
#     That promotion is its own decision, not made today.
#
# When adding a new ingress, copy the cert-manager.io/cluster-issuer
# annotation from a similar existing ingress; if you find yourself
# guessing, document the choice in the new ingress's PR rather than
# assuming.
info "Creating Let's Encrypt HTTP-01 ClusterIssuer..."
LETSENCRYPT_EMAIL="${LETSENCRYPT_EMAIL:-admin@example.com}"
cat <<EOF | kubectl apply -f -
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: ${LETSENCRYPT_EMAIL}
    privateKeySecretRef:
      name: letsencrypt-prod
    solvers:
      - http01:
          ingress:
            class: nginx
EOF

# --- ClusterIssuer (DNS-01 via Hetzner DNS) — spec 143 FR-008 ---
#
# Spec 143 commits to DNS-01 for the MinIO public ingress
# (minio.${DOMAIN}) to dodge HTTP-01's cluster-bootstrap problem
# permanently — HTTP-01 fails on first rollout if ingress isn't yet
# routing, and re-bites on every renewal during DNS maintenance.
# DNS-01 also unblocks wildcard certs as the platform adds more
# public subdomains.
#
# Prerequisites (one-time per cluster, NOT idempotent on every deploy):
#
#   (a) cert-manager-webhook-hetzner installed in cert-manager
#       namespace. Installed below (idempotent helm upgrade --install);
#       skip with SKIP_HETZNER_DNS_WEBHOOK=1 if the cluster has it
#       installed via another mechanism.
#
#   (b) HCLOUD_DNS_API_TOKEN set in .env. Generate at:
#         https://dns.hetzner.com/settings/api-token
#       Stored as a Kubernetes Secret in cert-manager namespace.
#
#   (c) DNS A record minio.${DOMAIN} → <cluster ingress IP>
#       MUST be created before the MinIO ingress in step 6 starts
#       requesting certificates. No DNS IaC exists for Hetzner DNS in
#       this repo (DNS is currently click-ops); the runbook step is:
#
#         Hetzner Cloud Console → DNS → ${DOMAIN} → Add record
#           Type: A
#           Name: minio
#           Value: $(kubectl get svc -n ingress-nginx ingress-nginx-controller -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
#           TTL: 60
#
#       Without this record, browser uploads still fail end-to-end
#       even after step 6 lands — the browser cannot resolve
#       minio.${DOMAIN}, and cert-manager cannot validate the
#       challenge response. Step 8 (e2e validation) verifies this.

if [ "${SKIP_HETZNER_DNS_WEBHOOK:-0}" != "1" ]; then
  if helm status cert-manager-webhook-hetzner -n cert-manager >/dev/null 2>&1; then
    info "cert-manager-webhook-hetzner already installed, skipping"
  else
    info "Installing cert-manager-webhook-hetzner..."
    helm upgrade --install cert-manager-webhook-hetzner cert-manager-webhook-hetzner \
      --repo https://vadimkim.github.io/cert-manager-webhook-hetzner \
      --namespace cert-manager \
      --wait --timeout 180s
  fi
fi

if [ -n "${HCLOUD_DNS_API_TOKEN:-}" ]; then
  info "Seeding hetzner-dns-secret for DNS-01 challenge..."
  kubectl create secret generic hetzner-dns-secret \
    --namespace cert-manager \
    --from-literal=api-key="$HCLOUD_DNS_API_TOKEN" \
    --dry-run=client -o yaml | kubectl apply -f -

  info "Creating Let's Encrypt DNS-01 ClusterIssuer..."
  cat <<EOF | kubectl apply -f -
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-dns01
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: ${LETSENCRYPT_EMAIL}
    privateKeySecretRef:
      name: letsencrypt-dns01
    solvers:
      - dns01:
          webhook:
            groupName: acme.${DOMAIN}
            solverName: hetzner
            config:
              secretName: hetzner-dns-secret
              zoneName: ${DOMAIN}
              apiUrl: https://dns.hetzner.com/api/v1
EOF
else
  info "HCLOUD_DNS_API_TOKEN not set — skipping letsencrypt-dns01 ClusterIssuer"
  info "  Set HCLOUD_DNS_API_TOKEN in .env and re-run post-create.sh to enable"
  info "  the DNS-01 challenge required by spec 143 (MinIO public ingress)."
fi

# --- Namespaces ---
info "Creating namespaces..."
for ns in stagecraft-system deployd-system rauthy-system; do
  kubectl create namespace "$ns" --dry-run=client -o yaml | kubectl apply -f -
done

# --- Resource quotas + limit ranges (skip default-deny for MVP) ---
info "Applying resource policies..."
for ns in stagecraft-system deployd-system rauthy-system; do
  kubectl apply -n "$ns" -f "$PLATFORM_ROOT/k8s/policies/namespace-baseline/resourcequota.yaml" 2>/dev/null || true
  kubectl apply -n "$ns" -f "$PLATFORM_ROOT/k8s/policies/namespace-baseline/limitrange.yaml" 2>/dev/null || true
done

# --- PostgreSQL ---
if helm status postgresql -n stagecraft-system >/dev/null 2>&1; then
  info "PostgreSQL already installed, skipping"

  # Drift guard: the bitnami chart only initializes the stagecraft user
  # password on first install (via initdb against an empty PVC). If .env's
  # POSTGRES_PASSWORD later diverges — e.g. `setup.sh --clean` ran without
  # destroying the cluster, or .env was hand-edited — Phase 2 would silently
  # write a stagecraft-api-secrets that can't authenticate, and every DB
  # query in stagecraft throws (OAuth callback surfaces as `account_error`).
  # Verify the current password actually works before continuing.
  info "Verifying POSTGRES_PASSWORD authenticates against live postgres..."
  POSTGRES_PASSWORD="${POSTGRES_PASSWORD:?POSTGRES_PASSWORD must be set}"
  kubectl delete pod pg-auth-check -n stagecraft-system --ignore-not-found=true >/dev/null
  # --rm requires an attached stream on newer kubectl; -i + </dev/null gives
  # us attached stdin without a TTY (TTYs drop output on fast-exit pods).
  if ! kubectl run pg-auth-check --rm -i --restart=Never --quiet \
       --namespace stagecraft-system \
       --image=bitnami/postgresql:latest \
       --env="PGPASSWORD=$POSTGRES_PASSWORD" \
       --command -- psql -h postgresql.stagecraft-system.svc.cluster.local \
       -U stagecraft -d auth -tAc 'SELECT 1' </dev/null >/dev/null 2>&1; then
    cat >&2 <<'EOF'
ERROR: POSTGRES_PASSWORD in .env does NOT authenticate against postgresql-0.
The live postgres still has its old password (persisted on its PVC) while
.env has drifted. Pick one of:

  a) Reset the DB user to match .env (dev clusters only; no data loss).
     Source .env first so bash and the DB use byte-identical values, and
     pipe the SQL on stdin so psql's :'pass' substitution runs (it does
     NOT run under `psql -c` with pure SQL, and shell-interpolating
     '$POSTGRES_PASSWORD' into a SQL literal breaks on $, `, \, or ':

       set -a; source .env; set +a
       PG_SUPER=$(kubectl -n stagecraft-system get secret postgresql \
         -o jsonpath='{.data.postgres-password}' | base64 -d)
       printf "ALTER USER stagecraft WITH PASSWORD :'pass';\n" | \
         kubectl -n stagecraft-system exec -i postgresql-0 -- \
           env PGPASSWORD="$PG_SUPER" psql -U postgres \
             -v ON_ERROR_STOP=1 -v pass="$POSTGRES_PASSWORD"

  b) Update .env POSTGRES_PASSWORD to what the DB actually has:
       kubectl -n stagecraft-system get secret postgresql \
         -o jsonpath='{.data.password}' | base64 -d
EOF
    exit 1
  fi
else
  info "Installing PostgreSQL..."
  POSTGRES_PASSWORD="${POSTGRES_PASSWORD:?POSTGRES_PASSWORD must be set}"

  helm upgrade --install postgresql oci://registry-1.docker.io/bitnamicharts/postgresql \
    --namespace stagecraft-system \
    --set auth.username=stagecraft \
    --set auth.password="$POSTGRES_PASSWORD" \
    --set auth.database=auth \
    --set primary.persistence.size=10Gi \
    --wait --timeout 300s

  info "Creating additional databases..."
  kubectl delete pod pg-init -n stagecraft-system --ignore-not-found=true
  kubectl run pg-init --rm -i --restart=Never \
    --namespace stagecraft-system \
    --image=bitnami/postgresql:latest \
    --env="PGPASSWORD=$POSTGRES_PASSWORD" \
    -- bash -c "
      createdb -h postgresql.stagecraft-system.svc.cluster.local -U stagecraft monitor 2>/dev/null || true
      createdb -h postgresql.stagecraft-system.svc.cluster.local -U stagecraft site 2>/dev/null || true
    "
fi

# --- NSQ ---
info "Installing NSQ..."
kubectl apply -f "$SCRIPT_DIR/nsq.yaml"

# --- MinIO (S3-compatible object store for knowledge intake) ---
# Uses the official MinIO chart at charts.min.io (quay.io/minio images).
# We deliberately avoid Bitnami's chart because Bitnami moved free MinIO
# images behind a paywall in mid-2025 — `docker.io/bitnami/minio:*` tags
# 404 today, leaving pods in ImagePullBackOff.
MINIO_CHART_INSTALLED=false
if helm status minio -n stagecraft-system >/dev/null 2>&1; then
  CURRENT_CHART=$(helm list -n stagecraft-system -o json \
    | grep -o '"chart":"minio-[^"]*"' | head -1 || true)
  # The official chart's name is `minio-<ver>`; Bitnami's is the same prefix
  # but pulls from a doomed registry. Re-install if the running pods are in
  # ImagePullBackOff regardless of which chart is recorded.
  if kubectl -n stagecraft-system get pods -l app=minio \
       -o jsonpath='{.items[*].status.containerStatuses[*].state.waiting.reason}' 2>/dev/null \
       | grep -q ImagePullBackOff; then
    info "MinIO release exists but pods are in ImagePullBackOff — reinstalling"
    helm uninstall minio -n stagecraft-system >/dev/null 2>&1 || true
    kubectl -n stagecraft-system delete pvc -l release=minio --ignore-not-found=true >/dev/null 2>&1 || true
  else
    info "MinIO already installed (${CURRENT_CHART}), skipping"
    MINIO_CHART_INSTALLED=true
  fi
fi

if [ "$MINIO_CHART_INSTALLED" = false ]; then
  info "Installing MinIO (official chart)..."
  MINIO_ROOT_USER="${MINIO_ROOT_USER:?MINIO_ROOT_USER must be set}"
  MINIO_ROOT_PASSWORD="${MINIO_ROOT_PASSWORD:?MINIO_ROOT_PASSWORD must be set}"

  helm repo add minio https://charts.min.io/ 2>/dev/null || true
  helm repo update minio >/dev/null

  # Standalone mode: single replica, single drive. Two services:
  # - API service: ClusterIP for in-cluster server ops + public ingress
  #   on minio.${DOMAIN} for browser presigned-PUT (spec 143 FR-005).
  # - Console service: ClusterIP only, no public ingress (operators
  #   reach it via kubectl port-forward).
  #
  # Spec 143 history: the original Hetzner deployment shipped with NO
  # ingress on either service because the platform was provisioned for
  # a server-side-proxy upload flow (option B) that the application
  # code never matched — storage.ts always issued presigned URLs to
  # the browser. The browser couldn't reach the cluster-internal
  # MinIO, so uploads silently failed for months. Spec 143 closed the
  # gap on the option-A side: dual-endpoint storage client + public
  # ingress here + CORS contract for the stagecraft origin.
  #
  # Required envs (FR-006a):
  #   MINIO_SERVER_URL — must match the public ingress hostname so
  #     SigV4 canonicalisation produces the same signature browser-side
  #     and server-side. Without this MinIO recomputes against its
  #     in-cluster hostname and rejects with SignatureDoesNotMatch.
  #   MINIO_API_CORS_ALLOW_ORIGIN — strict to the stagecraft origin;
  #     no wildcards.
  #
  # Recommended envs (defence-in-depth):
  #   MINIO_BROWSER=off — disables the console daemon globally. The
  #     console is also not exposed via ingress (consoleService.type
  #     remains ClusterIP), so this is belt-and-suspenders.
  #   MINIO_BROWSER_REDIRECT_URL — informational; coherent state if
  #     a future operator flips the console back on.
  #
  # Body-size annotation: 1g matches KNOWLEDGE_UPLOAD_MAX_BYTES in
  # platform/services/stagecraft/api/knowledge/uploadLimits.ts (spec
  # 143 FR-011). When that constant changes, this value MUST change
  # to match — uploadLimits.ts has the propagation comment pointing
  # back here.
  #
  # Cluster-issuer letsencrypt-dns01 is created by the post-create.sh
  # block below (cert-manager + Hetzner DNS webhook).
  helm upgrade --install minio minio/minio \
    --namespace stagecraft-system \
    --set rootUser="$MINIO_ROOT_USER" \
    --set rootPassword="$MINIO_ROOT_PASSWORD" \
    --set mode=standalone \
    --set replicas=1 \
    --set persistence.size=20Gi \
    --set resources.requests.memory=512Mi \
    --set resources.requests.cpu=100m \
    --set service.type=ClusterIP \
    --set consoleService.type=ClusterIP \
    --set environment.MINIO_SERVER_URL="https://minio.${DOMAIN}" \
    --set environment.MINIO_API_CORS_ALLOW_ORIGIN="${APP_BASE_URL}" \
    --set environment.MINIO_BROWSER="off" \
    --set environment.MINIO_BROWSER_REDIRECT_URL="https://minio.${DOMAIN}" \
    --set ingress.enabled=true \
    --set ingress.ingressClassName=nginx \
    --set "ingress.hosts[0]=minio.${DOMAIN}" \
    --set "ingress.tls[0].secretName=minio-tls" \
    --set "ingress.tls[0].hosts[0]=minio.${DOMAIN}" \
    --set "ingress.annotations.nginx\.ingress\.kubernetes\.io/proxy-body-size=1g" \
    --set "ingress.annotations.cert-manager\.io/cluster-issuer=letsencrypt-dns01" \
    --wait --timeout 300s
fi

info "Infrastructure bootstrap complete"
