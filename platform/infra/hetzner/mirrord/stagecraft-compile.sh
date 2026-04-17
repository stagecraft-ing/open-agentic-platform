#!/usr/bin/env bash
# Compile stagecraft Encore bundle against infra.config.hetzner.json so the
# resulting main.mjs binds to the cluster's PostgreSQL + NSQ + secrets, not
# the ephemeral local infra that `encore run` provisions.
#
# Trick: `encore build docker` is the only command that honours --config, but
# its docker-image packaging is pathologically slow. We run it only long
# enough for main.mjs + encore-runtime.node to land on disk, then kill it.
# This mirrors services/stagecraft/scripts/docker-build.sh but skips the
# `docker build` step — we just want the JS bundle to run under mirrord.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(cd "$SCRIPT_DIR/../../../services/stagecraft" && pwd)"
cd "$APP_DIR"

MAIN_MJS=".encore/build/combined/combined/main.mjs"
CONFIG="./infra.config.hetzner.json"
MANIFEST=".encore/manifest.json"

ENCORE_VERSION=$(grep -o '"encore.dev": "[^"]*"' package.json | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+')

# Find the host-native encore-runtime.node.
#
# The canonical source is `encore daemon env`, which prints ENCORE_RUNTIME_LIB
# pointing at the brew-bundled addon (libexec/runtimes/js/encore-runtime.node
# on macOS). On linux dev hosts this points into the cache under
# ~/Library/Caches/encore/cache/bin/v${version}/linux/${arch}/.
RUNTIME_CACHE="$(encore daemon env 2>/dev/null | grep '^ENCORE_RUNTIME_LIB=' | cut -d= -f2-)"

# Decide whether to recompile. Skip if main.mjs is newer than every source
# file we care about.
needs_compile() {
  [[ ! -f "$MAIN_MJS" ]] && return 0
  # any .ts under api/ newer than main.mjs?
  if find api -name '*.ts' -newer "$MAIN_MJS" -print -quit 2>/dev/null | grep -q .; then
    return 0
  fi
  # infra config newer than main.mjs?
  [[ "$CONFIG" -nt "$MAIN_MJS" ]] && return 0
  return 1
}

if needs_compile; then
  echo "==> Compiling encore application (via transient encore build docker)..."
  # --arch amd64 forces linux build artifacts in .encore/, but main.mjs is
  # pure JS — the native runtime we load below is host-native.
  encore build docker --arch amd64 --config "$CONFIG" mirrord-dev:tag >/dev/null 2>&1 &
  BUILD_PID=$!

  for _ in $(seq 1 180); do
    if [[ -f "$MAIN_MJS" ]] && [[ $(find "$MAIN_MJS" -mmin -1 2>/dev/null) ]]; then
      sleep 2
      break
    fi
    sleep 1
  done

  kill "$BUILD_PID" 2>/dev/null || true
  pkill -f "encore daemon" 2>/dev/null || true

  if [[ ! -f "$MAIN_MJS" ]]; then
    echo "ERROR: compile did not produce $MAIN_MJS" >&2
    exit 1
  fi
  echo "    main.mjs: $(ls -la "$MAIN_MJS" | awk '{print $5 " bytes, " $6 " " $7 " " $8}')"
else
  echo "==> main.mjs is current, skipping recompile"
fi

if [[ -z "$RUNTIME_CACHE" || ! -f "$RUNTIME_CACHE" ]]; then
  cat >&2 <<EOF
ERROR: could not locate host-native encore-runtime.node.

  \`encore daemon env\` returned: "$RUNTIME_CACHE"

  On darwin the runtime is bundled with the brew install. If missing,
  reinstall encore:
      brew uninstall encore && brew install encore
  (verify version matches package.json's "encore.dev": v${ENCORE_VERSION})
EOF
  exit 1
fi

# main.mjs looks for ENCORE_RUNTIME_LIB at startup.
export ENCORE_RUNTIME_LIB="$RUNTIME_CACHE"
echo "    ENCORE_RUNTIME_LIB=$ENCORE_RUNTIME_LIB"

# The runtime also needs ENCORE_APP_META_PATH — a binary protobuf of the
# compiled app topology that the Docker image would normally place at
# /encore/meta. `encore build docker` writes it to the per-app cache dir
# keyed by the local_id in .encore/manifest.json.
LOCAL_ID=$(grep -o '"local_id":"[^"]*"' "$MANIFEST" | cut -d'"' -f4)
META_PATH="$HOME/Library/Caches/encore/cache/${LOCAL_ID}/metadata.pb"
if [[ ! -f "$META_PATH" ]]; then
  echo "ERROR: app metadata not found at $META_PATH" >&2
  echo "    Try deleting $MAIN_MJS and re-running to force a full build." >&2
  exit 1
fi
echo "    ENCORE_APP_META_PATH=$META_PATH"

# Write a tiny env file the Makefile will source into mirrord exec.
cat > "$SCRIPT_DIR/.stagecraft.env" <<EOF
ENCORE_RUNTIME_LIB=$RUNTIME_CACHE
ENCORE_INFRA_CONFIG_PATH=$APP_DIR/$CONFIG
ENCORE_APP_META_PATH=$META_PATH
EOF
echo "==> Ready. Bundle will bind to cluster infra (postgresql.stagecraft-system, nsqd.stagecraft-system)."
