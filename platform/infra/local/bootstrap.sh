#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHARTS_ROOT="$PLATFORM_ROOT/charts"
ENV_FILE="${SCRIPT_DIR}/.env"
RUNTIME="${1:-k3d}"

echo "==> OAP Local Dev Bootstrap (runtime: $RUNTIME)"

# --- Pre-flight checks ---
command -v kubectl >/dev/null 2>&1 || { echo "ERROR: kubectl not found"; exit 1; }
command -v helm >/dev/null 2>&1 || { echo "ERROR: helm not found"; exit 1; }

if [ ! -f "$ENV_FILE" ]; then
  echo "ERROR: $ENV_FILE not found. Copy .env.example to .env and fill in values."
  exit 1
fi

# --- Create cluster ---
case "$RUNTIME" in
  k3d)
    command -v k3d >/dev/null 2>&1 || { echo "ERROR: k3d not found. Install: brew install k3d"; exit 1; }
    if k3d cluster list | grep -q "oap-local"; then
      echo "==> Cluster 'oap-local' already exists, skipping creation"
    else
      echo "==> Creating k3d cluster..."
      k3d cluster create --config "$SCRIPT_DIR/k3d-config.yaml"
    fi
    ;;
  kind)
    command -v kind >/dev/null 2>&1 || { echo "ERROR: kind not found. Install: brew install kind"; exit 1; }
    if kind get clusters 2>/dev/null | grep -q "oap-local"; then
      echo "==> Cluster 'oap-local' already exists, skipping creation"
    else
      echo "==> Creating kind cluster..."
      kind create cluster --name oap-local --config "$SCRIPT_DIR/kind-config.yaml"
    fi
    ;;
  *)
    echo "ERROR: Unknown runtime '$RUNTIME'. Use 'k3d' or 'kind'."
    exit 1
    ;;
esac

echo "==> Waiting for nodes to be ready..."
kubectl wait --for=condition=Ready nodes --all --timeout=120s

# --- Install ingress-nginx ---
echo "==> Installing ingress-nginx..."
helm upgrade --install ingress-nginx ingress-nginx \
  --repo https://kubernetes.github.io/ingress-nginx \
  --namespace ingress-nginx --create-namespace \
  --set controller.watchIngressWithoutClass=true \
  --wait --timeout 120s

# --- Create namespaces ---
echo "==> Creating namespaces..."
kubectl create namespace stagecraft-system --dry-run=client -o yaml | kubectl apply -f -
kubectl create namespace deployd-system --dry-run=client -o yaml | kubectl apply -f -

# --- Seed secrets from .env ---
echo "==> Seeding secrets from .env..."

# Source the env file
set -a
# shellcheck source=/dev/null
source "$ENV_FILE"
set +a

# Create secrets for stagecraft
kubectl create secret generic stagecraft-api-secrets \
  --namespace stagecraft-system \
  --from-literal=STAGECRAFT_DB_URL="${STAGECRAFT_DB_URL}" \
  --from-literal=LOGTO_M2M_CLIENT_ID="${LOGTO_M2M_CLIENT_ID}" \
  --from-literal=LOGTO_M2M_CLIENT_SECRET="${LOGTO_M2M_CLIENT_SECRET}" \
  --dry-run=client -o yaml | kubectl apply -f -

# Create secrets for deployd
kubectl create secret generic deployd-api-secrets \
  --namespace deployd-system \
  --from-literal=DEPLOYD_DB_URL="${DEPLOYD_DB_URL}" \
  --dry-run=client -o yaml | kubectl apply -f -

# --- Deploy platform services ---
echo "==> Deploying stagecraft..."
helm upgrade --install stagecraft "$CHARTS_ROOT/stagecraft" \
  --namespace stagecraft-system \
  -f "$CHARTS_ROOT/stagecraft/values.yaml" \
  -f "$CHARTS_ROOT/stagecraft/values-local.yaml" \
  --wait --timeout 300s

echo "==> Deploying deployd-api..."
helm upgrade --install deployd-api "$CHARTS_ROOT/deployd-api" \
  --namespace deployd-system \
  -f "$CHARTS_ROOT/deployd-api/values.yaml" \
  -f "$CHARTS_ROOT/deployd-api/values-local.yaml" \
  --wait --timeout 300s

echo ""
echo "=== OAP Local Dev Ready ==="
echo ""
echo "Services:"
echo "  stagecraft: http://stagecraft.localhost"
echo "  deployd:    http://deployd.localhost"
echo ""
echo "To access services, add to /etc/hosts:"
echo "  127.0.0.1 stagecraft.localhost deployd.localhost"
echo ""
echo "To tear down:"
if [ "$RUNTIME" = "k3d" ]; then
  echo "  k3d cluster delete oap-local"
else
  echo "  kind delete cluster --name oap-local"
fi
