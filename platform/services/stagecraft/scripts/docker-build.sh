#!/usr/bin/env bash
set -euo pipefail

# Build a Docker image for stagecraft, bypassing encore's slow Docker build.
#
# Usage:
#   ./scripts/docker-build.sh [IMAGE_TAG] [--arch amd64|arm64]
#
# Examples:
#   ./scripts/docker-build.sh ghcr.io/open-agentic-platform/stagecraft:latest
#   ./scripts/docker-build.sh ghcr.io/open-agentic-platform/stagecraft:latest --arch arm64

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$APP_DIR"

IMAGE_TAG="${1:-ghcr.io/open-agentic-platform/stagecraft:latest}"
ARCH="${3:-amd64}"
if [[ "${2:-}" == "--arch" ]]; then
  ARCH="${3:-amd64}"
fi

ENCORE_VERSION=$(grep -o '"encore.dev": "[^"]*"' package.json | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+')
RUNTIME_CACHE="$HOME/Library/Caches/encore/cache/bin/v${ENCORE_VERSION}/linux/${ARCH}/encore-runtime.node"

echo "==> Building stagecraft Docker image"
echo "    Image:   $IMAGE_TAG"
echo "    Arch:    linux/$ARCH"
echo "    Runtime: v$ENCORE_VERSION"

# Step 1: Ensure the encore runtime binary is available
if [[ ! -f "$RUNTIME_CACHE" ]]; then
  echo "==> Downloading encore runtime for linux/$ARCH..."
  # Trigger a brief encore build to download the runtime, then kill it
  timeout 60 encore build docker --arch "$ARCH" --config ./infra.config.hetzner.json dummy:tag 2>/dev/null || true
  sleep 2
  pkill -f "encore daemon" 2>/dev/null || true
fi

if [[ ! -f "$RUNTIME_CACHE" ]]; then
  echo "ERROR: encore-runtime.node not found at $RUNTIME_CACHE"
  echo "Run 'encore build docker --arch $ARCH ...' briefly to download it."
  exit 1
fi
cp "$RUNTIME_CACHE" ./encore-runtime.node

# Step 2: Ensure the compiled entrypoint exists
MAIN_MJS=".encore/build/combined/combined/main.mjs"
if [[ ! -f "$MAIN_MJS" ]]; then
  echo "==> Compiling encore application..."
  # Start encore build docker, wait for compilation to finish, then kill
  encore build docker --arch "$ARCH" --config ./infra.config.hetzner.json dummy:tag 2>/dev/null &
  BUILD_PID=$!

  # Wait for main.mjs to be written (compilation done)
  for i in $(seq 1 120); do
    if [[ -f "$MAIN_MJS" ]] && [[ $(find "$MAIN_MJS" -mmin -1 2>/dev/null) ]]; then
      sleep 2  # Brief grace period for write to complete
      break
    fi
    sleep 1
  done

  kill $BUILD_PID 2>/dev/null || true
  pkill -f "encore daemon" 2>/dev/null || true
fi

if [[ ! -f "$MAIN_MJS" ]]; then
  echo "ERROR: Compiled entrypoint not found at $MAIN_MJS"
  exit 1
fi

# Step 3: Ensure frontend is built
if [[ ! -d "web/build/client" ]]; then
  echo "==> Building frontend..."
  cd web && npx react-router build && cd ..
fi

# Step 4: Build the Docker image (Docker's own layer compression is fast)
echo "==> Building Docker image..."
docker build \
  --platform "linux/$ARCH" \
  -t "$IMAGE_TAG" \
  -f Dockerfile \
  .

# Cleanup
rm -f encore-runtime.node

echo "==> Done! Image: $IMAGE_TAG"
echo "    Size: $(docker image inspect "$IMAGE_TAG" --format '{{.Size}}' | numfmt --to=iec 2>/dev/null || docker image inspect "$IMAGE_TAG" --format '{{.Size}}')"
