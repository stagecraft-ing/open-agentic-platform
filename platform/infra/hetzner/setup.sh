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
# Prerequisites: kubectl, helm, hetzner-k3s (current pre-flight set).
#                Spec 151 Phase 1 closure (T-007) will add flux, sops, age to
#                the pre-flight when the bootstrap step migrates here. Full
#                operator prereq table: DEVELOPERS.md §"Hetzner GitOps
#                operator (spec 151)".
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
    MINIO_ROOT_USER MINIO_ROOT_PASSWORD
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

# MinIO in-cluster object store. Root user must be >= 3 chars, password
# must be >= 8 chars per the bitnami chart's validator.
generate_minio_user() { openssl rand -hex 6; }
auto_fill MINIO_ROOT_USER     generate_minio_user
auto_fill MINIO_ROOT_PASSWORD generate_secret

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
# Spec 137 Phase 4↔5 integration — kubernetes-reflector for wildcard
# cert replication into tenant namespaces.
#
# The `tenants-wildcard-tls` Secret created by cert-manager (next block)
# lives in `cert-manager` namespace. Tenant Ingresses (rendered by
# deployd-api in per-app namespaces) need a local copy of that Secret to
# terminate TLS. emberstack/kubernetes-reflector watches Secrets with
# reflector annotations and clones them into matching namespaces — no
# stagecraft/deployd-api code path required.
#
# Idempotent: `helm upgrade --install` reapplies if anything changed.
# Pin chart to avoid surprise upgrades; bumping is a deliberate edit.
# ---------------------------------------------------------------------------
info "Installing kubernetes-reflector for wildcard cert replication..."
helm repo add emberstack https://emberstack.github.io/helm-charts >/dev/null 2>&1 || true
helm repo update >/dev/null
helm upgrade --install reflector emberstack/reflector \
  --namespace kube-system \
  --version 9.1.6 \
  --wait --timeout 5m
ok "kubernetes-reflector installed"

# ---------------------------------------------------------------------------
# Spec 106 amendment (2026-05-17) — wildcard cert for tenant hostnames.
# When CLOUDFLARE_DNS_API_TOKEN is set, create the cert-manager
# Cloudflare-token Secret + apply the DNS-01 ClusterIssuer + the
# *.tenants.${DOMAIN} Certificate. cert-manager handles the renewal
# loop. Without the token, the existing HTTP-01 ClusterIssuer keeps
# handling stagecraft/deployd/rauthy/minio (it can't do wildcards), and
# tenant ingresses remain TLS-uncoverable until the operator provides
# the token.
#
# Spec 137 amendment (2026-05-17) — the Certificate manifest carries
# reflector annotations on its `spec.secretTemplate`. cert-manager
# propagates those to the generated `tenants-wildcard-tls` Secret, which
# reflector then clones into every namespace matching the
# `reflection-auto-namespaces` regex. Phase 4 tenant deploys mount the
# replicated Secret directly without a stagecraft side-write.
# ---------------------------------------------------------------------------
if [ -n "${CLOUDFLARE_DNS_API_TOKEN:-}" ]; then
  info "Creating cloudflare-api-token secret in cert-manager namespace..."
  kubectl create secret generic cloudflare-api-token \
    --namespace cert-manager \
    --from-literal=api-token="$CLOUDFLARE_DNS_API_TOKEN" \
    --dry-run=client -o yaml | kubectl apply -f -

  info "Applying tenants wildcard ClusterIssuer + Certificate..."
  for manifest in "$SCRIPT_DIR/manifests/letsencrypt-prod-dns01-cloudflare-issuer.yaml" \
                  "$SCRIPT_DIR/manifests/tenants-wildcard-certificate.yaml"; do
    envsubst < "$manifest" | kubectl apply -f -
  done

  info "Waiting for tenants-wildcard cert to reach Ready=True (up to 5m)..."
  if kubectl -n cert-manager wait --for=condition=Ready certificate/tenants-wildcard --timeout=300s; then
    ok "Wildcard tenant cert issued"
  else
    warn "Wildcard tenant cert did not reach Ready within 5m. Check:"
    warn "  kubectl -n cert-manager describe certificate tenants-wildcard"
    warn "  kubectl -n cert-manager get challenges,orders,certificaterequests"
    warn "  Common causes: Cloudflare token lacks Zone.DNS:Edit on the zone,"
    warn "  or zone selector mismatch (issuer.solvers[0].selector.dnsZones must match)."
  fi
else
  warn "CLOUDFLARE_DNS_API_TOKEN not set — skipping wildcard tenant cert."
  warn "  Spec 137 magic-link / federated-login evidence (E2/E3/E4) requires"
  warn "  TLS on tenant ingress hostnames. Set CLOUDFLARE_DNS_API_TOKEN in"
  warn "  .env and re-run setup.sh, or follow .env.example for the token shape."
fi

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

# ---------------------------------------------------------------------------
# Spec 106 amendment (2026-05-17) — optional SMTP for Rauthy magic-link.
# When SMTP_USERNAME is set in .env, materialise `rauthy-smtp-secret` and
# enable the chart's `smtp.enabled=true` overlay. The chart's statefulset
# pulls SMTP_FROM/URL/PORT/USERNAME/PASSWORD from this Secret.
#
# **Hetzner Cloud blocks outbound TCP on port 465** (implicit-TLS / SMTPS),
# confirmed empirically 2026-05-17 against smtp.gmail.com. Port 587 with
# STARTTLS is reachable and is the supported default. Setting SMTP_PORT=465
# yields a hard `warn` here because Rauthy will crash-loop at startup on
# the SMTP connection probe (mailer.rs panics after retry exhaustion).
# ---------------------------------------------------------------------------
RAUTHY_SMTP_HELM_ARGS=()
if [ -n "${SMTP_USERNAME:-}" ]; then
  SMTP_PORT_RESOLVED="${SMTP_PORT:-587}"
  if [ "$SMTP_PORT_RESOLVED" = "465" ]; then
    warn "SMTP_PORT=465 is blocked on Hetzner outbound — Rauthy will crash at startup."
    warn "  Override with SMTP_PORT=587 (STARTTLS submission) in .env, then re-run setup.sh."
    warn "  Skipping SMTP wire-up to keep Rauthy healthy."
  else
    # Default starttls_only=true when port is 587 (STARTTLS submission).
    # Rauthy 0.35's mailer defaults to "try implicit TLS first, fall back
    # to STARTTLS" but the fallback doesn't trigger on every error shape
    # (e.g. Gmail's plaintext-banner-then-STARTTLS-upgrade returns
    # `InvalidMessage(InvalidContentType)` from the implicit-TLS handshake,
    # bypassing the fallback). Setting SMTP_STARTTLS_ONLY=true is the
    # documented way to make Rauthy negotiate STARTTLS from the start.
    STARTTLS_ONLY_RESOLVED="${SMTP_STARTTLS_ONLY:-}"
    if [ -z "$STARTTLS_ONLY_RESOLVED" ] && [ "$SMTP_PORT_RESOLVED" = "587" ]; then
      STARTTLS_ONLY_RESOLVED="true"
    fi
    info "Creating rauthy-smtp-secret (SMTP_USERNAME=${SMTP_USERNAME}, SMTP_PORT=${SMTP_PORT_RESOLVED}, STARTTLS_ONLY=${STARTTLS_ONLY_RESOLVED:-<default>})..."
    kubectl create secret generic rauthy-smtp-secret \
      --namespace rauthy-system \
      --from-literal=from="${SMTP_FROM:-Rauthy <rauthy@${DOMAIN}>}" \
      --from-literal=url="${SMTP_URL:-}" \
      --from-literal=port="$SMTP_PORT_RESOLVED" \
      --from-literal=username="$SMTP_USERNAME" \
      --from-literal=password="${SMTP_PASSWORD:-}" \
      --from-literal=starttls_only="$STARTTLS_ONLY_RESOLVED" \
      --from-literal=danger_insecure="${SMTP_DANGER_INSECURE:-false}" \
      --dry-run=client -o yaml | kubectl apply -f -
    RAUTHY_SMTP_HELM_ARGS+=(--set "smtp.enabled=true")
    ok "SMTP enabled — Rauthy will send magic-link emails via $SMTP_URL:$SMTP_PORT_RESOLVED"
  fi
else
  warn "SMTP_USERNAME not set in .env — Rauthy magic-link login will be unavailable"
fi

info "Deploying Rauthy..."
helm upgrade --install rauthy "$CHARTS_ROOT/rauthy" \
  --namespace rauthy-system \
  -f "$CHARTS_ROOT/rauthy/values.yaml" \
  -f "$CHARTS_ROOT/rauthy/values-hetzner.yaml" \
  --set "ingress.host=auth.${DOMAIN}" \
  --set "oidc.issuer=https://auth.${DOMAIN}/auth/v1/" \
  --set "bootstrap.adminEmail=admin@${DOMAIN}" \
  "${RAUTHY_SMTP_HELM_ARGS[@]}" \
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
           RAUTHY_CLIENT_ID RAUTHY_CLIENT_SECRET RAUTHY_ADMIN_TOKEN \
           STAGECRAFT_KNOWLEDGE_SWEEPER_CLIENT_ID STAGECRAFT_KNOWLEDGE_SWEEPER_CLIENT_SECRET; do
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
  echo "     - stagecraft-knowledge-sweeper-m2m-app (confidential, client_credentials)"
  echo "       Default Scopes: platform:knowledge:sweep   (spec 143 FR-010 + §12 L-006:"
  echo "       Rauthy 0.35 client_credentials mints Default Scopes regardless of scope=,"
  echo "       so Allowed Scopes alone is silently inert. Default Scopes is load-bearing.)"
  echo "       Fill STAGECRAFT_KNOWLEDGE_SWEEPER_CLIENT_ID/_SECRET in .env."
  echo "       (FU-003 will add stagecraft-factory-sweeper-m2m-app and"
  echo "        stagecraft-audit-sweeper-m2m-app for spec 115/087/124 sweepers; .env"
  echo "        already carries the placeholder slots.)"
  echo "  3. Create the GitHub OAuth App for Rauthy at https://github.com/settings/developers"
  echo "     (GITHUB_UPSTREAM_CLIENT_ID/_SECRET, spec 106)"
  echo "        - Homepage: https://auth.${DOMAIN}"
  echo "        - Callback: https://auth.${DOMAIN}/auth/v1/providers/callback"
  echo "  3a. (Optional, spec 137 federated upstream) Google upstream Auth Provider"
  echo "     for tenant gates. Create the Google OAuth client at"
  echo "     https://console.cloud.google.com/auth/clients and store the"
  echo "     credentials in .env as GOOGLE_UPSTREAM_CLIENT_ID/_SECRET."
  echo "     Then register the provider in Rauthy admin UI:"
  echo "        - URL:       https://auth.${DOMAIN}/auth/v1/admin/providers"
  echo "        - Type:      Google"
  echo "        - Issuer:    https://accounts.google.com"
  echo "        - Client ID: \$GOOGLE_UPSTREAM_CLIENT_ID"
  echo "        - Secret:    \$GOOGLE_UPSTREAM_CLIENT_SECRET"
  echo "        - Callback:  https://auth.${DOMAIN}/auth/v1/providers/callback"
  echo "     (Auto-provisioning the provider via the admin API is tracked as"
  echo "      a spec 106 follow-up; the .env values are loaded but currently"
  echo "      surfaced only as this manual instruction.)"
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
  --from-literal=S3_ENDPOINT="http://minio.stagecraft-system.svc.cluster.local:9000" \
  --from-literal=S3_PUBLIC_ENDPOINT="${S3_PUBLIC_ENDPOINT:-https://minio.${DOMAIN}}" \
  --from-literal=S3_REGION="us-east-1" \
  --from-literal=S3_ACCESS_KEY="$MINIO_ROOT_USER" \
  --from-literal=S3_SECRET_KEY="$MINIO_ROOT_PASSWORD" \
  --dry-run=client -o yaml | kubectl apply -f -

# Spec 143 FR-010 — per-purpose-credential mount discipline.
# `stagecraft-knowledge-sweeper-credentials` is the SOLE Secret the
# orphan-imported sweeper CronJob mounts. Materialised here directly
# (Hetzner uses `secrets.provider: "k8s"`); cloud deployments that
# enable ESO will get the same Secret name + key shape from the
# `external-secret-knowledge-sweeper.yaml` chart template instead.
# A leaked credential here is bounded to one Rauthy client's surface.
# FU-003 will add sibling Secrets for spec 115 / 087 / 124 sweepers,
# each separate, no cross-purpose mounts.
info "Creating stagecraft-knowledge-sweeper-credentials..."
kubectl create secret generic stagecraft-knowledge-sweeper-credentials \
  --namespace stagecraft-system \
  --from-literal=CLIENT_ID="$STAGECRAFT_KNOWLEDGE_SWEEPER_CLIENT_ID" \
  --from-literal=CLIENT_SECRET="$STAGECRAFT_KNOWLEDGE_SWEEPER_CLIENT_SECRET" \
  --dry-run=client -o yaml | kubectl apply -f -

info "Refreshing stagecraft pods to pick up new secrets..."
# Spec 143 §12 L-003 — CD owns the stagecraft helm release. setup.sh
# does NOT helm-upgrade stagecraft because doing so applied
# values-hetzner.yaml's `tag: latest` which clobbered CD's
# sha-pinned tag and caused a stale-pod-against-forward-DB
# regression on 2026-05-08. Single writer for the helm field-manager
# surface; restart is the right verb for "secrets rotated, re-read".
if kubectl get deploy stagecraft-api -n stagecraft-system >/dev/null 2>&1; then
  kubectl rollout restart deploy/stagecraft-api -n stagecraft-system
  kubectl rollout status deploy/stagecraft-api -n stagecraft-system --timeout=600s
  ok "stagecraft-api rollout complete"
else
  warn "stagecraft-api deployment not yet provisioned (fresh cluster). CD will create it on first push to main; re-run setup.sh after CD lands to refresh secrets."
fi

info "Refreshing deployd-api pods to pick up new secrets..."
# Spec 143 §12 L-003 / FU-002 closure — CD owns the deployd-api helm
# release (cd-deployd-api-rs.yml deploys with sha-pinned image.tag).
# setup.sh does NOT helm-upgrade deployd-api because doing so would
# apply values-hetzner.yaml's `tag: latest` and clobber CD's sha-pin —
# the same dual-writer shape L-003 documents for stagecraft. Single
# writer for the helm field-manager surface; restart is the right verb
# for "HIQLITE_SECRET_* rotated, re-read". The values-hetzner.yaml
# `tag: latest` line is now latent (no active racer); a future spec
# can mirror L-003's chart-side hardening (remove `tag:`, add
# `pullPolicy: Always`) as defence-in-depth.
if kubectl get deploy deployd-api -n deployd-system >/dev/null 2>&1; then
  kubectl rollout restart deploy/deployd-api -n deployd-system
  kubectl rollout status deploy/deployd-api -n deployd-system --timeout=600s
  ok "deployd-api rollout complete"
else
  warn "deployd-api deployment not yet provisioned (fresh cluster). CD will create it on first push to main touching platform/services/deployd-api-rs/** or platform/charts/deployd-api/** (or workflow_dispatch with deploy=true); re-run setup.sh after CD lands to refresh secrets."
fi

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
