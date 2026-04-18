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

# --- ClusterIssuer ---
info "Creating Let's Encrypt ClusterIssuer..."
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

info "Infrastructure bootstrap complete"
