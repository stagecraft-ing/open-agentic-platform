#!/usr/bin/env bash
# =============================================================================
# OAP Hetzner Setup — Single entrypoint for cluster + platform deployment
# =============================================================================
# Usage:
#   1. cp .env.example .env && $EDITOR .env   # set HCLOUD_TOKEN
#   2. ./setup.sh                              # Phase 1: cluster + rauthy
#   3. Fill in GitHub + OIDC values in .env
#   4. ./setup.sh                              # Phase 2: full platform
#
# Flags:
#   --clean   Destroy existing cluster, remove kubeconfig and auto-generated
#             secrets from .env, then start fresh from Phase 1.
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
GENERATORS="$PLATFORM_ROOT/scripts/generate"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
info()  { printf '\033[1;34m==> %s\033[0m\n' "$*"; }
warn()  { printf '\033[1;33m    %s\033[0m\n' "$*"; }
ok()    { printf '\033[1;32m    %s\033[0m\n' "$*"; }
err()   { printf '\033[1;31mERROR: %s\033[0m\n' "$*" >&2; exit 1; }

generate_secret()  { bash "$GENERATORS/generate-secret.sh"; }
generate_b64url()  { bash "$GENERATORS/generate-base64url-password.sh"; }
# Rauthy ENC_KEY_ID must match ^[a-zA-Z0-9:_-]{2,20}$
generate_enc_key_id() { openssl rand -hex 8; }
# Rauthy ENC_KEY must be exactly 32 random bytes, base64-encoded
generate_enc_key()    { openssl rand -base64 32 | tr -d '\n'; echo; }

# ---------------------------------------------------------------------------
# Load .env
# ---------------------------------------------------------------------------
ENV_FILE="$SCRIPT_DIR/.env"
if [ ! -f "$ENV_FILE" ]; then
  err ".env not found. Run:  cp .env.example .env  and set HCLOUD_TOKEN"
fi

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

[ -n "${HCLOUD_TOKEN:-}" ] || err "HCLOUD_TOKEN is empty in .env"
[ -n "${DOMAIN:-}" ]       || err "DOMAIN is empty in .env"

# ---------------------------------------------------------------------------
# --clean: destroy cluster and reset local state
# ---------------------------------------------------------------------------
if [ "${1:-}" = "--clean" ]; then
  info "Clean start requested"

  # Destroy Hetzner cluster if kubeconfig exists. If destroy fails, bail out
  # BEFORE clearing .env — otherwise the cluster's postgres keeps the old
  # password while a fresh one lands in .env, and every subsequent deploy
  # writes a stagecraft secret that can't authenticate (account_error in the
  # OAuth callback, etc).
  if [ -f "$SCRIPT_DIR/kubeconfig" ]; then
    warn "Destroying existing cluster..."
    hetzner-k3s delete --config "$SCRIPT_DIR/cluster.yaml" \
      || err "hetzner-k3s delete failed; cluster still exists. Refusing to clear .env secrets (would drift from live postgres)."
    rm -f "$SCRIPT_DIR/kubeconfig"
    ok "Cluster destroyed and kubeconfig removed"
  else
    ok "No kubeconfig found, nothing to destroy"
  fi

  # Clear auto-generated secrets from .env (preserve manual values)
  AUTO_SECRETS=(
    POSTGRES_PASSWORD SESSION_SECRET
    RAUTHY_RAFT_SECRET RAUTHY_API_SECRET RAUTHY_ADMIN_PASSWORD
    RAUTHY_ENC_KEY_ID RAUTHY_ENC_KEY
    HIQLITE_SECRET_RAFT HIQLITE_SECRET_API
    GITHUB_WEBHOOK_SECRET
  )
  for var in "${AUTO_SECRETS[@]}"; do
    sed -i.bak "s|^${var}=.*|${var}=|" "$ENV_FILE"
  done
  rm -f "$ENV_FILE.bak"
  ok "Auto-generated secrets cleared from .env"

  info "Clean complete — re-run ./setup.sh to start fresh"
  exit 0
fi

# ---------------------------------------------------------------------------
# Auto-generate secrets (fill blanks, write back to .env)
# ---------------------------------------------------------------------------
info "Checking secrets..."
CHANGED=false

auto_fill() {
  local var_name="$1"
  local generator="${2:-generate_secret}"
  local current_val="${!var_name:-}"

  if [ -z "$current_val" ]; then
    local new_val
    new_val=$($generator)
    export "$var_name=$new_val"
    if grep -q "^${var_name}=" "$ENV_FILE"; then
      sed -i.bak "s|^${var_name}=.*|${var_name}=\"${new_val}\"|" "$ENV_FILE"
    else
      echo "${var_name}=\"${new_val}\"" >> "$ENV_FILE"
    fi
    ok "Generated $var_name"
    CHANGED=true
  fi
}

auto_fill POSTGRES_PASSWORD   generate_secret
auto_fill SESSION_SECRET      generate_secret
auto_fill RAUTHY_RAFT_SECRET  generate_secret
auto_fill RAUTHY_API_SECRET   generate_secret
auto_fill RAUTHY_ADMIN_PASSWORD generate_b64url
auto_fill RAUTHY_ENC_KEY_ID    generate_enc_key_id
auto_fill RAUTHY_ENC_KEY       generate_enc_key
auto_fill HIQLITE_SECRET_RAFT generate_secret
auto_fill HIQLITE_SECRET_API  generate_secret
auto_fill GITHUB_WEBHOOK_SECRET generate_secret

# Sync DB_PASSWORD = POSTGRES_PASSWORD
export DB_PASSWORD="$POSTGRES_PASSWORD"

rm -f "$ENV_FILE.bak"

if [ "$CHANGED" = true ]; then
  ok "Secrets written to .env — keep this file safe"
fi

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------
info "Pre-flight checks..."
for cmd in kubectl helm hetzner-k3s; do
  command -v "$cmd" >/dev/null 2>&1 || err "$cmd not found. Install it first."
done
ok "All tools present"

# ---------------------------------------------------------------------------
# Phase 1: Create K3s cluster (idempotent)
# ---------------------------------------------------------------------------
KUBECONFIG_PATH="$SCRIPT_DIR/kubeconfig"

if [ ! -f "$KUBECONFIG_PATH" ]; then
  info "Creating Hetzner K3s cluster..."
  hetzner-k3s create --config "$SCRIPT_DIR/cluster.yaml"
else
  info "Kubeconfig exists, skipping cluster creation"
fi

export KUBECONFIG="$KUBECONFIG_PATH"

info "Waiting for nodes..."
kubectl wait --for=condition=Ready nodes --all --timeout=300s
ok "All nodes ready"

# ---------------------------------------------------------------------------
# Phase 1: Bootstrap infrastructure
# ---------------------------------------------------------------------------
info "Bootstrapping infrastructure..."
"$SCRIPT_DIR/post-create.sh"

# ---------------------------------------------------------------------------
# Phase 1: Create K8s secrets + deploy Rauthy
# ---------------------------------------------------------------------------
CHARTS_ROOT="$PLATFORM_ROOT/charts"

info "Creating rauthy-secrets..."
# ENC_KEYS format: key_id/base64_of_32_random_bytes
RAUTHY_ENC_KEYS="${RAUTHY_ENC_KEY_ID}/${RAUTHY_ENC_KEY}"
kubectl create secret generic rauthy-secrets \
  --namespace rauthy-system \
  --from-literal=raft-secret="$RAUTHY_RAFT_SECRET" \
  --from-literal=api-secret="$RAUTHY_API_SECRET" \
  --from-literal=admin-password="$RAUTHY_ADMIN_PASSWORD" \
  --from-literal=enc-keys="$RAUTHY_ENC_KEYS" \
  --from-literal=enc-key-active="$RAUTHY_ENC_KEY_ID" \
  --dry-run=client -o yaml | kubectl apply -f -

info "Creating deployd-api-secrets..."
kubectl create secret generic deployd-api-secrets \
  --namespace deployd-system \
  --from-literal=HIQLITE_SECRET_RAFT="$HIQLITE_SECRET_RAFT" \
  --from-literal=HIQLITE_SECRET_API="$HIQLITE_SECRET_API" \
  --dry-run=client -o yaml | kubectl apply -f -

info "Deploying Rauthy..."
helm upgrade --install rauthy "$CHARTS_ROOT/rauthy" \
  --namespace rauthy-system \
  -f "$CHARTS_ROOT/rauthy/values.yaml" \
  -f "$CHARTS_ROOT/rauthy/values-hetzner.yaml" \
  --set "ingress.host=auth.${DOMAIN}" \
  --set "oidc.issuer=https://auth.${DOMAIN}/auth/v1/" \
  --set "bootstrap.adminEmail=admin@${DOMAIN}" \
  --wait --timeout 300s

# ---------------------------------------------------------------------------
# Show Node IP + DNS instructions
# ---------------------------------------------------------------------------
echo ""
NODE_IP=$(kubectl get nodes -l '!node-role.kubernetes.io/master' \
  -o jsonpath='{.items[0].status.addresses[?(@.type=="ExternalIP")].address}' 2>/dev/null \
  || kubectl get nodes -o jsonpath='{.items[0].status.addresses[?(@.type=="ExternalIP")].address}' 2>/dev/null \
  || echo "pending")

info "Worker Node IP: $NODE_IP"
echo ""
echo "  DNS A records needed (if not already set):"
echo "    ${DOMAIN}            -> $NODE_IP"
echo "    deploy.${DOMAIN}     -> $NODE_IP"
echo "    auth.${DOMAIN}       -> $NODE_IP"
echo ""

# ---------------------------------------------------------------------------
# Check if Phase 2 values are ready
# ---------------------------------------------------------------------------
PHASE2_READY=true
MISSING=()

for var in GITHUB_UPSTREAM_CLIENT_ID GITHUB_UPSTREAM_CLIENT_SECRET \
           GITHUB_APP_ID GITHUB_APP_PRIVATE_KEY_B64 \
           OIDC_SPA_CLIENT_ID OIDC_M2M_CLIENT_ID OIDC_M2M_CLIENT_SECRET \
           RAUTHY_CLIENT_ID RAUTHY_CLIENT_SECRET RAUTHY_ADMIN_TOKEN; do
  if [ -z "${!var:-}" ]; then
    PHASE2_READY=false
    MISSING+=("$var")
  fi
done

if [ "$PHASE2_READY" = false ]; then
  echo "============================================"
  echo "  Phase 1 Complete"
  echo "============================================"
  echo ""
  echo "Rauthy admin panel:"
  echo "  URL:      https://auth.${DOMAIN}"
  echo "  Email:    admin@${DOMAIN}"
  echo "  Password: $RAUTHY_ADMIN_PASSWORD"
  echo ""
  echo "Next steps:"
  echo "  1. Point DNS A records to $NODE_IP (see above)"
  echo "  2. Log into Rauthy and create OIDC clients:"
  echo "     - SPA client (public, authorization_code, redirect: https://${DOMAIN}/auth/callback)"
  echo "     - M2M client (confidential, client_credentials, scope: deployd:deploy)"
  echo "     - Server client (confidential, for backend OIDC)"
  echo "  3. Create the GitHub OAuth App for Rauthy at https://github.com/settings/developers"
  echo "     (GITHUB_UPSTREAM_CLIENT_ID/_SECRET, spec 106)"
  echo "        - Homepage: https://auth.${DOMAIN}"
  echo "        - Callback: https://auth.${DOMAIN}/auth/v1/providers/callback"
  echo "  4. Create GitHub App at https://github.com/settings/apps/new"
  echo "     - Webhook URL: https://${DOMAIN}/api/github/webhook"
  echo "     - Webhook secret: $GITHUB_WEBHOOK_SECRET"
  echo "     - Store private key as base64:"
  echo "       base64 -i private-key.pem  (macOS)"
  echo "       base64 -w0 private-key.pem (Linux)"
  echo "  5. Fill in the values in .env, then re-run:"
  echo "     ./setup.sh"
  echo ""
  echo "Missing values:"
  for m in "${MISSING[@]}"; do
    echo "  - $m"
  done
  echo ""
  exit 0
fi

# ---------------------------------------------------------------------------
# Phase 2: Create stagecraft secrets + deploy all services
# ---------------------------------------------------------------------------
info "Phase 2: All values present — deploying full platform"

STAGECRAFT_DB_URL="postgres://stagecraft:${POSTGRES_PASSWORD}@postgresql.stagecraft-system:5432/auth?sslmode=disable"

# Decode base64-encoded GitHub App private key
GITHUB_APP_PRIVATE_KEY=$(echo "$GITHUB_APP_PRIVATE_KEY_B64" | base64 -d 2>/dev/null) \
  || err "GITHUB_APP_PRIVATE_KEY_B64 is not valid base64. Encode with: base64 -i private-key.pem"

# Create GHCR image-pull secrets (required for pulling from private ghcr.io)
if [ -n "${GHCR_PAT:-}" ]; then
  info "Creating GHCR image-pull secrets..."
  for ns_secret in "stagecraft-system/ghcr-credentials" "deployd-system/ghcr-pull-secret"; do
    ns="${ns_secret%%/*}"
    secret_name="${ns_secret##*/}"
    kubectl create secret docker-registry "$secret_name" \
      --namespace "$ns" \
      --docker-server=ghcr.io \
      --docker-username=oap \
      --docker-password="$GHCR_PAT" \
      --dry-run=client -o yaml | kubectl apply -f -
  done
else
  warn "GHCR_PAT not set — skipping image-pull secrets (pods won't be able to pull from ghcr.io)"
fi

info "Creating stagecraft-api-secrets..."
kubectl create secret generic stagecraft-api-secrets \
  --namespace stagecraft-system \
  --from-literal=DOMAIN="$DOMAIN" \
  --from-literal=APP_BASE_URL="$APP_BASE_URL" \
  --from-literal=SESSION_SECRET="$SESSION_SECRET" \
  --from-literal=OIDC_SPA_CLIENT_ID="$OIDC_SPA_CLIENT_ID" \
  --from-literal=OIDC_M2M_CLIENT_ID="$OIDC_M2M_CLIENT_ID" \
  --from-literal=OIDC_M2M_CLIENT_SECRET="$OIDC_M2M_CLIENT_SECRET" \
  --from-literal=RAUTHY_URL="$RAUTHY_URL" \
  --from-literal=RAUTHY_CLIENT_ID="$RAUTHY_CLIENT_ID" \
  --from-literal=RAUTHY_CLIENT_SECRET="$RAUTHY_CLIENT_SECRET" \
  --from-literal=RAUTHY_ADMIN_TOKEN="$RAUTHY_ADMIN_TOKEN" \
  --from-literal=GITHUB_UPSTREAM_CLIENT_ID="$GITHUB_UPSTREAM_CLIENT_ID" \
  --from-literal=GITHUB_UPSTREAM_CLIENT_SECRET="$GITHUB_UPSTREAM_CLIENT_SECRET" \
  --from-literal=GITHUB_APP_ID="$GITHUB_APP_ID" \
  --from-literal=GITHUB_APP_PRIVATE_KEY="$GITHUB_APP_PRIVATE_KEY" \
  --from-literal=GITHUB_WEBHOOK_SECRET="$GITHUB_WEBHOOK_SECRET" \
  --from-literal=POSTGRES_PASSWORD="$POSTGRES_PASSWORD" \
  --from-literal=STAGECRAFT_DB_URL="$STAGECRAFT_DB_URL" \
  --from-literal=SLACK_WEBHOOK_URL="${SLACK_WEBHOOK_URL:-}" \
  --dry-run=client -o yaml | kubectl apply -f -

info "Deploying Stagecraft..."
helm upgrade --install stagecraft "$CHARTS_ROOT/stagecraft" \
  --namespace stagecraft-system \
  -f "$CHARTS_ROOT/stagecraft/values.yaml" \
  -f "$CHARTS_ROOT/stagecraft/values-hetzner.yaml" \
  --set "ingress.host=${DOMAIN}" \
  --set "oidc.endpoint=https://auth.${DOMAIN}" \
  --set "oidc.deploydAudience=https://deploy.${DOMAIN}" \
  --wait --timeout 600s

info "Deploying Deployd-API..."
helm upgrade --install deployd-api "$CHARTS_ROOT/deployd-api" \
  --namespace deployd-system \
  -f "$CHARTS_ROOT/deployd-api/values.yaml" \
  -f "$CHARTS_ROOT/deployd-api/values-hetzner.yaml" \
  --set "ingress.host=deploy.${DOMAIN}" \
  --set "oidc.endpoint=https://auth.${DOMAIN}" \
  --set "oidc.audience=https://deploy.${DOMAIN}" \
  --wait --timeout 300s

# ---------------------------------------------------------------------------
# Sync secrets to GitHub Actions (if gh CLI is available)
# ---------------------------------------------------------------------------
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
  info "Syncing secrets to GitHub Actions..."
  KUBECONFIG_B64=$(base64 < "$KUBECONFIG_PATH" | tr -d '\n')

  # Pin --repo so multiple git remotes don't trigger an interactive picker.
  GH_REPO="${GH_REPO:-stagecraft-ing/open-agentic-platform}"

  gh secret set KUBECONFIG_HETZNER --repo "$GH_REPO" --body "$KUBECONFIG_B64" 2>/dev/null && ok "KUBECONFIG_HETZNER synced" || warn "Failed to sync KUBECONFIG_HETZNER"
  gh secret set WEBHOOK_SECRET --repo "$GH_REPO" --body "$GITHUB_WEBHOOK_SECRET" 2>/dev/null && ok "WEBHOOK_SECRET synced" || warn "Failed to sync WEBHOOK_SECRET"

  if [ -n "${GHCR_PAT:-}" ]; then
    gh secret set GHCR_PAT --repo "$GH_REPO" --body "$GHCR_PAT" 2>/dev/null && ok "GHCR_PAT synced" || warn "Failed to sync GHCR_PAT"
  fi
else
  warn "gh CLI not available or not authenticated — skipping GitHub Actions secret sync"
  echo "  To sync manually:"
  echo "    gh secret set KUBECONFIG_HETZNER --repo stagecraft-ing/open-agentic-platform < <(base64 < $KUBECONFIG_PATH)"
  echo "    gh secret set WEBHOOK_SECRET --repo stagecraft-ing/open-agentic-platform --body \"\$GITHUB_WEBHOOK_SECRET\""
  echo "    gh secret set GHCR_PAT --repo stagecraft-ing/open-agentic-platform --body \"\$GHCR_PAT\""
fi

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
echo ""
echo "============================================"
echo "  OAP Platform Live on Hetzner"
echo "============================================"
echo ""
echo "  Stagecraft:  https://${DOMAIN}"
echo "  Deployd API: https://deploy.${DOMAIN}"
echo "  Rauthy OIDC: https://auth.${DOMAIN}"
echo ""
echo "  Tear down:   hetzner-k3s delete --config $SCRIPT_DIR/cluster.yaml"
echo ""
