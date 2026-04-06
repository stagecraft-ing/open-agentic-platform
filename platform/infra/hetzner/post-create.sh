#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHARTS_ROOT="$PLATFORM_ROOT/charts"
KUBECONFIG_PATH="${KUBECONFIG:-$SCRIPT_DIR/kubeconfig}"

if [ ! -f "$KUBECONFIG_PATH" ]; then
  echo "ERROR: Kubeconfig not found at $KUBECONFIG_PATH"
  echo "Run 'hetzner-k3s create --config cluster.yaml' first."
  exit 1
fi

export KUBECONFIG="$KUBECONFIG_PATH"

echo "==> OAP Hetzner K3s Post-Create Bootstrap"

# --- Pre-flight checks ---
command -v kubectl >/dev/null 2>&1 || { echo "ERROR: kubectl not found"; exit 1; }
command -v helm >/dev/null 2>&1 || { echo "ERROR: helm not found"; exit 1; }

echo "==> Waiting for nodes to be ready..."
kubectl wait --for=condition=Ready nodes --all --timeout=300s

# --- Install ingress-nginx ---
echo "==> Installing ingress-nginx..."
helm upgrade --install ingress-nginx ingress-nginx \
  --repo https://kubernetes.github.io/ingress-nginx \
  --namespace ingress-nginx --create-namespace \
  --set controller.kind=DaemonSet \
  --set controller.hostPort.enabled=true \
  --set controller.service.type=LoadBalancer \
  --wait --timeout 180s

# --- Install cert-manager ---
echo "==> Installing cert-manager..."
helm upgrade --install cert-manager cert-manager \
  --repo https://charts.jetstack.io \
  --namespace cert-manager --create-namespace \
  --version v1.19.3 \
  --set crds.enabled=true \
  --wait --timeout 180s

echo "==> Waiting for cert-manager webhook to be ready..."
kubectl wait --for=condition=Available deployment/cert-manager-webhook \
  -n cert-manager --timeout=120s

# --- Install Let's Encrypt ClusterIssuer ---
echo "==> Creating Let's Encrypt ClusterIssuer..."
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

# --- Create namespaces ---
echo "==> Creating namespaces..."
kubectl create namespace stagecraft-system --dry-run=client -o yaml | kubectl apply -f -
kubectl create namespace deployd-system --dry-run=client -o yaml | kubectl apply -f -
kubectl create namespace rauthy-system --dry-run=client -o yaml | kubectl apply -f -

# --- Apply resource quotas and limit ranges (skip default-deny network policy for MVP) ---
echo "==> Applying resource quotas and limit ranges..."
for ns in stagecraft-system deployd-system rauthy-system; do
  kubectl apply -n "$ns" -f "$PLATFORM_ROOT/k8s/policies/namespace-baseline/resourcequota.yaml" 2>/dev/null || true
  kubectl apply -n "$ns" -f "$PLATFORM_ROOT/k8s/policies/namespace-baseline/limitrange.yaml" 2>/dev/null || true
done

# --- Install PostgreSQL (Bitnami) ---
echo "==> Installing PostgreSQL..."
if helm status postgresql -n stagecraft-system >/dev/null 2>&1; then
  echo "==> PostgreSQL already installed, skipping."
else
  POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-$(openssl rand -base64 24)}"
  echo "NOTE: PostgreSQL password is: $POSTGRES_PASSWORD" >&2
  echo "      Save this — you'll need it for the stagecraft-api-secrets." >&2

  helm upgrade --install postgresql oci://registry-1.docker.io/bitnamicharts/postgresql \
    --namespace stagecraft-system \
    --set auth.username=stagecraft \
    --set auth.password="$POSTGRES_PASSWORD" \
    --set auth.database=auth \
    --set primary.persistence.size=10Gi \
    --wait --timeout 300s

  echo "==> Creating additional PostgreSQL databases..."
  kubectl run pg-init --rm -i --restart=Never \
    --namespace stagecraft-system \
    --image=bitnami/postgresql:latest \
    --env="PGPASSWORD=$POSTGRES_PASSWORD" \
    -- bash -c "
      createdb -h postgresql.stagecraft-system.svc.cluster.local -U stagecraft monitor 2>/dev/null || true
      createdb -h postgresql.stagecraft-system.svc.cluster.local -U stagecraft site 2>/dev/null || true
    "
fi

# --- Install NSQ ---
echo "==> Installing NSQ..."
kubectl apply -f "$SCRIPT_DIR/nsq.yaml"

# --- Print secret creation instructions ---
echo ""
echo "============================================"
echo "  Secret Creation Instructions"
echo "============================================"
echo ""
echo "Before deploying services, create the following secrets:"
echo ""
echo "1. Rauthy secrets (deploy rauthy first, then configure clients):"
echo ""
echo "  kubectl create secret generic rauthy-secrets \\"
echo "    --namespace rauthy-system \\"
echo "    --from-literal=raft-secret=\"\$(openssl rand -hex 16)\" \\"
echo "    --from-literal=api-secret=\"\$(openssl rand -hex 16)\" \\"
echo "    --from-literal=admin-password=\"<choose-admin-password>\""
echo ""
echo "2. Stagecraft secrets (after configuring Rauthy OIDC clients + GitHub apps):"
echo ""
echo "  kubectl create secret generic stagecraft-api-secrets \\"
echo "    --namespace stagecraft-system \\"
echo "    --from-literal=DOMAIN='stagecraft.ing' \\"
echo "    --from-literal=APP_BASE_URL='https://stagecraft.ing' \\"
echo "    --from-literal=SESSION_SECRET=\"\$(openssl rand -hex 32)\" \\"
echo "    --from-literal=OIDC_SPA_CLIENT_ID='<from-rauthy>' \\"
echo "    --from-literal=OIDC_M2M_CLIENT_ID='<from-rauthy>' \\"
echo "    --from-literal=OIDC_M2M_CLIENT_SECRET='<from-rauthy>' \\"
echo "    --from-literal=RAUTHY_URL='https://auth.stagecraft.ing' \\"
echo "    --from-literal=RAUTHY_CLIENT_ID='<from-rauthy>' \\"
echo "    --from-literal=RAUTHY_CLIENT_SECRET='<from-rauthy>' \\"
echo "    --from-literal=RAUTHY_ADMIN_TOKEN='<from-rauthy>' \\"
echo "    --from-literal=GITHUB_OAUTH_CLIENT_ID='<from-github>' \\"
echo "    --from-literal=GITHUB_OAUTH_CLIENT_SECRET='<from-github>' \\"
echo "    --from-literal=GITHUB_APP_ID='<from-github>' \\"
echo "    --from-literal=GITHUB_APP_PRIVATE_KEY='<pem-contents>' \\"
echo "    --from-literal=GITHUB_WEBHOOK_SECRET='<from-github>' \\"
echo "    --from-literal=DB_PASSWORD='$POSTGRES_PASSWORD' \\"
echo "    --from-literal=STAGECRAFT_DB_URL='postgres://stagecraft:${POSTGRES_PASSWORD}@postgresql.stagecraft-system:5432/auth?sslmode=disable' \\"
echo "    --from-literal=SLACK_WEBHOOK_URL=''"
echo ""
echo "3. Deployd secrets:"
echo ""
echo "  kubectl create secret generic deployd-api-secrets \\"
echo "    --namespace deployd-system \\"
echo "    --from-literal=HIQLITE_SECRET_RAFT=\"\$(openssl rand -hex 16)\" \\"
echo "    --from-literal=HIQLITE_SECRET_API=\"\$(openssl rand -hex 16)\""
echo ""

# --- Check if secrets exist before deploying ---
echo "==> Checking for required secrets..."
SECRETS_READY=true

if ! kubectl get secret rauthy-secrets -n rauthy-system >/dev/null 2>&1; then
  echo "WARNING: rauthy-secrets not found in rauthy-system"
  SECRETS_READY=false
fi

if ! kubectl get secret stagecraft-api-secrets -n stagecraft-system >/dev/null 2>&1; then
  echo "WARNING: stagecraft-api-secrets not found in stagecraft-system"
  SECRETS_READY=false
fi

if ! kubectl get secret deployd-api-secrets -n deployd-system >/dev/null 2>&1; then
  echo "WARNING: deployd-api-secrets not found in deployd-system"
  SECRETS_READY=false
fi

if [ "$SECRETS_READY" = false ]; then
  echo ""
  echo "Secrets not found. Create them first, then deploy with:"
  echo "  make deploy TARGET=hetzner"
  echo ""
  echo "==> Load balancer IP for DNS configuration:"
  kubectl get svc -n ingress-nginx ingress-nginx-controller \
    -o jsonpath='{.status.loadBalancer.ingress[0].ip}' 2>/dev/null || echo "(pending)"
  echo ""
  echo "Point these A records to the IP above:"
  echo "  stagecraft.ing"
  echo "  deploy.stagecraft.ing"
  echo "  auth.stagecraft.ing"
  echo ""
  exit 0
fi

# --- Deploy Rauthy ---
echo "==> Deploying rauthy..."
helm upgrade --install rauthy "$CHARTS_ROOT/rauthy" \
  --namespace rauthy-system \
  -f "$CHARTS_ROOT/rauthy/values.yaml" \
  -f "$CHARTS_ROOT/rauthy/values-hetzner.yaml" \
  --wait --timeout 300s

# --- Deploy platform services ---
echo "==> Deploying stagecraft..."
helm upgrade --install stagecraft "$CHARTS_ROOT/stagecraft" \
  --namespace stagecraft-system \
  -f "$CHARTS_ROOT/stagecraft/values.yaml" \
  -f "$CHARTS_ROOT/stagecraft/values-hetzner.yaml" \
  --wait --timeout 300s

echo "==> Deploying deployd-api..."
helm upgrade --install deployd-api "$CHARTS_ROOT/deployd-api" \
  --namespace deployd-system \
  -f "$CHARTS_ROOT/deployd-api/values.yaml" \
  -f "$CHARTS_ROOT/deployd-api/values-hetzner.yaml" \
  --wait --timeout 300s

echo ""
echo "=== OAP Hetzner K3s Ready ==="
echo ""
echo "Services:"
echo "  Stagecraft: https://stagecraft.ing"
echo "  Deployd:    https://deploy.stagecraft.ing"
echo "  Rauthy:     https://auth.stagecraft.ing"
echo ""
echo "To tear down:"
echo "  hetzner-k3s delete --config $SCRIPT_DIR/cluster.yaml"
