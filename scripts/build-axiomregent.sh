#!/usr/bin/env bash
# Build axiomregent sidecar binaries for Tauri desktop bundling.
#
# Usage:
#   ./scripts/build-axiomregent.sh              # build for current host
#   ./scripts/build-axiomregent.sh --all        # build for all installed targets
#   ./scripts/build-axiomregent.sh <triple>...  # build for specific targets
#
# Binaries are placed in apps/desktop/src-tauri/binaries/ with Tauri's
# naming convention: axiomregent-{triple}[.exe]
#
# Prerequisites:
#   - Rust toolchain with target installed: rustup target add <triple>
#   - For C deps (rusqlite bundled, zstd): native C compiler per target
#   - On CI, prefer matrix builds (one runner per OS) over cross-compilation

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE_DIR="$REPO_ROOT/crates/axiomregent"
BIN_DIR="$REPO_ROOT/apps/desktop/src-tauri/binaries"

ALL_TARGETS=(
    "aarch64-apple-darwin"
    "x86_64-apple-darwin"
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
    "x86_64-pc-windows-msvc"
)

detect_host_target() {
    rustc -vV | grep '^host:' | awk '{print $2}'
}

build_target() {
    local target="$1"
    echo "==> Building axiomregent for $target"

    cd "$CRATE_DIR"
    cargo build --release --target "$target"

    local src="$CRATE_DIR/target/$target/release/axiomregent"
    local dst="$BIN_DIR/axiomregent-$target"

    # Windows: add .exe extension
    if [[ "$target" == *windows* ]]; then
        src="${src}.exe"
        dst="${dst}.exe"
    fi

    if [[ ! -f "$src" ]]; then
        echo "ERROR: expected binary not found at $src" >&2
        return 1
    fi

    cp "$src" "$dst"

    # Strip debug symbols on Unix targets (Windows MSVC release is already stripped)
    if [[ "$target" != *windows* ]] && command -v strip &>/dev/null; then
        strip "$dst" 2>/dev/null || true
    fi

    local size
    size=$(wc -c < "$dst" | tr -d ' ')
    local size_mb=$((size / 1048576))
    echo "    -> $dst ($size_mb MB)"
}

main() {
    local targets=()

    if [[ $# -eq 0 ]]; then
        targets+=("$(detect_host_target)")
    elif [[ "$1" == "--all" ]]; then
        targets=("${ALL_TARGETS[@]}")
    else
        targets=("$@")
    fi

    mkdir -p "$BIN_DIR"

    local failed=0
    for t in "${targets[@]}"; do
        if build_target "$t"; then
            echo "    [OK] $t"
        else
            echo "    [FAIL] $t" >&2
            failed=$((failed + 1))
        fi
    done

    echo ""
    echo "Built ${#targets[@]} target(s), $failed failure(s)."
    echo "Binaries in: $BIN_DIR"
    ls -lh "$BIN_DIR"/axiomregent-*

    if [[ $failed -gt 0 ]]; then
        exit 1
    fi
}

main "$@"
