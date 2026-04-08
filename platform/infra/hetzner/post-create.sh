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

# --- ClusterIssuer ---
info "Creating Let's Encrypt ClusterIssuer..."
LETSENCRYPT_EMAIL="${LETSENCRYPT_EMAIL:-admin@stagecraft.ing}"
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

info "Infrastructure bootstrap complete"
