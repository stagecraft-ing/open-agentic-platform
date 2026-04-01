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
  --set installCRDs=true \
  --wait --timeout 180s

echo "==> Waiting for cert-manager webhook to be ready..."
kubectl wait --for=condition=Available deployment/cert-manager-webhook \
  -n cert-manager --timeout=120s

# --- Install Let's Encrypt ClusterIssuer ---
echo "==> Creating Let's Encrypt ClusterIssuer..."
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

# --- Create namespaces ---
echo "==> Creating namespaces..."
kubectl create namespace stagecraft-system --dry-run=client -o yaml | kubectl apply -f -
kubectl create namespace deployd-system --dry-run=client -o yaml | kubectl apply -f -

# --- Apply baseline policies ---
echo "==> Applying namespace baseline policies..."
for ns in stagecraft-system deployd-system; do
  kubectl apply -n "$ns" -f "$PLATFORM_ROOT/k8s/policies/namespace-baseline/"
done

# --- Seed secrets ---
echo "==> Seeding secrets..."
echo "NOTE: You must create secrets manually for Hetzner deployments."
echo "Example:"
echo "  kubectl create secret generic stagecraft-api-secrets \\"
echo "    --namespace stagecraft-system \\"
echo "    --from-literal=STAGECRAFT_DB_URL='postgres://...' \\"
echo "    --from-literal=LOGTO_M2M_CLIENT_ID='...' \\"
echo "    --from-literal=LOGTO_M2M_CLIENT_SECRET='...'"
echo ""
echo "  kubectl create secret generic deployd-api-secrets \\"
echo "    --namespace deployd-system \\"
echo "    --from-literal=DEPLOYD_DB_URL='postgres://...'"
echo ""

# --- Check if secrets exist before deploying ---
echo "==> Checking for required secrets..."
SECRETS_READY=true

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
  exit 0
fi

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
echo "Next steps:"
echo "  1. Point DNS records to the load balancer IP:"
echo "     kubectl get svc -n ingress-nginx ingress-nginx-controller -o jsonpath='{.status.loadBalancer.ingress[0].ip}'"
echo "  2. Services will be available at the configured ingress hosts"
echo ""
echo "To tear down:"
echo "  hetzner-k3s delete --config $SCRIPT_DIR/cluster.yaml"
