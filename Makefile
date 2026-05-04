# Open Agentic Platform — Root Makefile
#
# Quick start:
#   make setup   # one-time: install deps, build tools, compile spec registry
#   make dev     # start desktop app (Vite + Tauri with hot-reload)
#
# Platform services (optional, for org policy/auth work):
#   make dev-platform   # start stagecraft + deployd-api in background
#   make dev-all        # desktop + platform services

.PHONY: setup dev dev-platform dev-all stop \
        axiomregent axiomregent-all fetch-axiomregent fetch-axiomregent-check \
        registry spec-compile spec-tools \
        index index-check index-render \
        check-deps \
        agent-frontmatter-ts ci-agent-frontmatter-ts \
        ci ci-rust ci-tools ci-desktop ci-stagecraft ci-schema-parity \
        ci-supply-chain ci-supply-chain-cargo ci-supply-chain-pnpm ci-supply-chain-npm \
        ci-spec-code-coupling \
        ci-cross ci-parity \
        ci-fast ci-fast-rust ci-fast-tools ci-fast-desktop \
        ci-fast-stagecraft ci-fast-schema-parity \
        ci-fast-spec-coupling ci-fast-supply-chain

# ============================================================
# Prerequisites check
# ============================================================

check-deps:
	@echo "Checking prerequisites..."
	@command -v rustc  >/dev/null 2>&1 || { echo "  MISSING: rust    — curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; exit 1; }
	@command -v pnpm   >/dev/null 2>&1 || { echo "  MISSING: pnpm    — brew install pnpm"; exit 1; }
	@command -v bun    >/dev/null 2>&1 || { echo "  MISSING: bun     — brew install bun"; exit 1; }
	@command -v node   >/dev/null 2>&1 || { echo "  MISSING: node    — brew install node"; exit 1; }
	@command -v gh     >/dev/null 2>&1 || { echo "  MISSING: gh      — brew install gh, then run: gh auth login"; exit 1; }
	@echo "All prerequisites found."

# ============================================================
# Setup (one-time)
# ============================================================

setup: check-deps
	@echo ""
	@echo "==> Installing pnpm workspace dependencies..."
	pnpm install
	@echo ""
	@echo "==> Building spec compiler..."
	cargo build --release --manifest-path tools/spec-compiler/Cargo.toml
	@echo ""
	@echo "==> Compiling spec registry..."
	./tools/spec-compiler/target/release/spec-compiler compile
	@echo ""
	@echo ""
	@echo "==> Building codebase indexer..."
	cargo build --release --manifest-path tools/codebase-indexer/Cargo.toml
	@echo ""
	@echo "==> Compiling codebase index..."
	./tools/codebase-indexer/target/release/codebase-indexer compile
	@echo ""
	@echo "==> Fetching axiomregent sidecar binary..."
	@$(MAKE) fetch-axiomregent-check || echo "  WARN: fetch failed. Run 'make axiomregent' to build from source."
	@echo ""
	@echo "==> Setup complete. Run 'make dev' to start."

# ============================================================
# axiomregent sidecar binary
# ============================================================

# Default repo for `gh release download`. Auto-detected from the local
# git remote when possible; otherwise falls back to the canonical path so
# fresh clones from a fork still resolve to the upstream releases.
AXIOMREGENT_REPO   ?= $(shell git config --get remote.origin.url 2>/dev/null | sed -E 's,.*github.com[:/](.+)\.git,\1,' | sed -E 's,.*github.com[:/](.+)$$,\1,' | head -1)
ifeq ($(AXIOMREGENT_REPO),)
AXIOMREGENT_REPO   := stagecraft-ing/open-agentic-platform
endif
AXIOMREGENT_BINDIR = apps/desktop/src-tauri/binaries

axiomregent:
	@echo "==> Building axiomregent from source..."
	cargo build --release --manifest-path crates/axiomregent/Cargo.toml
	@HOST_TRIPLE=$$(rustc -vV | grep '^host:' | awk '{print $$2}'); \
	EXT=""; \
	case "$$HOST_TRIPLE" in *windows*) EXT=".exe";; esac; \
	SRC="crates/axiomregent/target/release/axiomregent$$EXT"; \
	DST="$(AXIOMREGENT_BINDIR)/axiomregent-$$HOST_TRIPLE$$EXT"; \
	mkdir -p $(AXIOMREGENT_BINDIR); \
	cp "$$SRC" "$$DST"; \
	case "$$HOST_TRIPLE" in *windows*) ;; *) strip "$$DST" 2>/dev/null || true;; esac; \
	echo "    -> $$DST"

## Build axiomregent for every supported target and install into the sidecar dir.
## Replaces scripts/build-axiomregent.sh --all (spec 105 Phase 3).
## Prerequisite per target: `rustup target add <triple>`.
axiomregent-all:
	@set -e; mkdir -p $(AXIOMREGENT_BINDIR); \
	 for t in $(CI_CROSS_TARGETS); do \
	   echo "==> axiomregent-all: $$t"; \
	   cargo build --release --target $$t --manifest-path crates/axiomregent/Cargo.toml; \
	   EXT=""; case "$$t" in *windows*) EXT=".exe";; esac; \
	   SRC=crates/target/$$t/release/axiomregent$$EXT; \
	   DST=$(AXIOMREGENT_BINDIR)/axiomregent-$$t$$EXT; \
	   cp "$$SRC" "$$DST"; \
	   case "$$t" in *windows*) ;; *) strip "$$DST" 2>/dev/null || true;; esac; \
	   echo "    -> $$DST"; \
	 done

## Fetch pre-built axiomregent sidecar for the host triple from a GitHub Release.
## Replaces scripts/fetch-axiomregent.js (spec 105 Phase 2).
fetch-axiomregent:
	@command -v gh >/dev/null 2>&1 || { echo "  MISSING: gh — brew install gh, then run: gh auth login"; exit 1; }
	@HOST=$$(rustc -vV | grep '^host:' | awk '{print $$2}'); \
	 EXT=""; case "$$HOST" in *windows*) EXT=".exe";; esac; \
	 mkdir -p $(AXIOMREGENT_BINDIR); \
	 echo "==> fetch-axiomregent: $$HOST"; \
	 gh release download --repo $(AXIOMREGENT_REPO) \
	    --pattern "axiomregent-$$HOST$$EXT" \
	    --dir $(AXIOMREGENT_BINDIR) \
	    --skip-existing

## Idempotent variant: skip fetch if the sidecar is already present for the host triple.
fetch-axiomregent-check:
	@HOST=$$(rustc -vV | grep '^host:' | awk '{print $$2}'); \
	 EXT=""; case "$$HOST" in *windows*) EXT=".exe";; esac; \
	 BIN=$(AXIOMREGENT_BINDIR)/axiomregent-$$HOST$$EXT; \
	 if [ -f "$$BIN" ]; then \
	   echo "  axiomregent sidecar present at $$BIN"; \
	 else \
	   $(MAKE) fetch-axiomregent; \
	 fi

# ============================================================
# Spec tools
# ============================================================

## Recompile spec registry + codebase index in one step (102 FR-026).
registry: spec-compile index ci-schema-parity
	@echo "==> Registry and index recompiled."

spec-compile:
	./tools/spec-compiler/target/release/spec-compiler compile

spec-tools:
	cargo build --release --manifest-path tools/spec-compiler/Cargo.toml
	cargo build --release --manifest-path tools/registry-consumer/Cargo.toml
	cargo build --release --manifest-path tools/spec-lint/Cargo.toml
	cargo build --release --manifest-path tools/codebase-indexer/Cargo.toml
	cargo build --release --manifest-path tools/stakeholder-doc-lint/Cargo.toml

# ============================================================
# agent-frontmatter TS mirror (spec 111 §2.1, Phase 2)
# ============================================================
#
# The `agent-frontmatter` crate (spec 054) owns the `UnifiedFrontmatter`
# type. `cargo test` on that crate regenerates the TypeScript mirror
# under platform/services/stagecraft/api/agents/frontmatter/ via ts-rs.
# Two targets:
#   agent-frontmatter-ts       regenerate the bindings (write-through)
#   ci-agent-frontmatter-ts    regenerate + fail if the working tree drifts

AGENT_FRONTMATTER_TS_DIR = platform/services/stagecraft/api/agents/frontmatter

agent-frontmatter-ts:
	cargo test --manifest-path crates/agent-frontmatter/Cargo.toml
	@echo "==> agent-frontmatter TS mirror regenerated at $(AGENT_FRONTMATTER_TS_DIR)/"

## CI drift gate: regenerate bindings, then require a clean working tree
## for the generated dir. Any modified or untracked file means the Rust
## type changed without a corresponding commit of the regenerated TS.
ci-agent-frontmatter-ts:
	cargo test --manifest-path crates/agent-frontmatter/Cargo.toml
	@git diff --exit-code -- $(AGENT_FRONTMATTER_TS_DIR) || { \
	    echo "ERROR: agent-frontmatter TS mirror has modified files."; \
	    echo "Run 'make agent-frontmatter-ts' and commit the result."; \
	    exit 1; \
	}
	@UNTRACKED=$$(git ls-files --others --exclude-standard -- $(AGENT_FRONTMATTER_TS_DIR)); \
	 if [ -n "$$UNTRACKED" ]; then \
	    echo "ERROR: agent-frontmatter TS mirror has untracked files:"; \
	    echo "$$UNTRACKED"; \
	    echo "A new #[derive(TS)] type was added without committing its generated .ts."; \
	    exit 1; \
	 fi

# ============================================================
# Codebase Index
# ============================================================
#
# All three targets ensure the binary is current before invoking it. A
# stale binary built against an older source tree silently produces a
# different content hash than the same source compiled fresh — that
# masquerades as a cross-platform determinism bug (see issue #46
# investigation). Rebuilding before each invocation costs nothing on
# warm cargo cache.

CODEBASE_INDEXER_BIN = tools/codebase-indexer/target/release/codebase-indexer

$(CODEBASE_INDEXER_BIN): tools/codebase-indexer/Cargo.toml tools/codebase-indexer/src/*.rs
	cargo build --release --manifest-path tools/codebase-indexer/Cargo.toml

index: $(CODEBASE_INDEXER_BIN)
	./$(CODEBASE_INDEXER_BIN) compile

index-check: $(CODEBASE_INDEXER_BIN)
	./$(CODEBASE_INDEXER_BIN) check

index-render: $(CODEBASE_INDEXER_BIN)
	./$(CODEBASE_INDEXER_BIN) render

# ============================================================
# Adapter Scopes (removed in spec 108 — see factory_adapters table)
# ============================================================
# adapter-scopes.json was compiled from factory/adapters/*/manifest.yaml.
# Spec 108 moves adapter manifests into the factory_adapters table, so the
# offline compiler is obsolete; the bundled snapshot in
# platform/services/stagecraft/api/factory/adapter-scopes.json is retained
# as a static fallback until the Phase 3 sync worker populates the table.

# ============================================================
# Development — Desktop App
# ============================================================

dev:
	@echo "==> Starting OPC desktop (Vite + Tauri)..."
	@echo "    This will compile Rust on first run (~2-3 min)."
	@echo ""
	cd apps/desktop && pnpm tauri dev

# ============================================================
# Development — Platform Services
# ============================================================

dev-stagecraft:
	@echo "==> Starting stagecraft (Encore.ts, port 4000)..."
	@command -v encore >/dev/null 2>&1 || { echo "  MISSING: encore — brew install encoredev/tap/encore"; exit 1; }
	cd platform/services/stagecraft && npm install --silent && npm run start

dev-deployd:
	@echo "==> Starting deployd-api (Rust/axum, port 8080)..."
	DEPLOYD_DATA_DIR=$(CURDIR)/.local/deployd DEPLOYD_AUDIENCE=deployd-local DEPLOYD_REQUIRED_SCOPE=deployd:admin cargo run --manifest-path platform/services/deployd-api-rs/Cargo.toml

dev-platform:
	@echo "==> Starting platform services in background..."
	@echo "    stagecraft → http://localhost:4000"
	@echo "    deployd    → http://localhost:8080"
	@echo ""
	@$(MAKE) dev-stagecraft &
	@$(MAKE) dev-deployd &
	@echo "Platform services starting. Use 'make stop' to kill them."

dev-all:
	@$(MAKE) dev-platform
	@sleep 2
	@$(MAKE) dev

stop:
	@echo "==> Stopping background services..."
	-@pkill -f "encore run" 2>/dev/null || true
	-@pkill -f "deployd-api" 2>/dev/null || true   # literal binary name; the prior `deployd.api` regex matched any character in place of `-`.
	@echo "Done."

# ============================================================
# Cloud deployment (delegates to platform/Makefile)
# ============================================================

deploy-%:
	cd platform && $(MAKE) deploy TARGET=$*

destroy-%:
	cd platform && $(MAKE) destroy TARGET=$*

# ============================================================
# CI parity — single source of truth for local end-to-end validation.
#
# Mirrors every gate enforced by .github/workflows/. If `make ci` passes
# locally, CI will pass too. Any new workflow gate MUST be added here in
# the same change — never a one-off script under scripts/.
#
# Composes:
#   ci-rust       — Rust per-manifest: check + clippy -D warnings + test
#                   (ci-axiomregent, ci-crates, ci-deployd-api-rs,
#                    ci-orchestrator, ci-policy-kernel)
#   ci-tools      — Tool crates + registry-consumer contract subsets +
#                   codebase-indexer staleness gate (spec-conformance.yml)
#   ci-desktop    — apps/desktop: tauri rust (custom clippy flags) +
#                   version alignment + tsc --noEmit + vitest (ci-desktop.yml)
#   ci-stagecraft — platform/services/stagecraft: npm ci + tsc + vitest
#                   (ci-stagecraft.yml)
#
# Opt-in (not part of `ci`):
#   ci-cross      — axiomregent cross-target matrix (build-axiomregent.yml);
#                   requires `rustup target add <triple>` per target.
# ============================================================

ci: ci-rust ci-tools ci-desktop ci-stagecraft ci-schema-parity ci-spec-code-coupling ci-supply-chain
	@echo ""
	@echo "==> Local CI parity: all gates passed."

# Rust manifests each validated with: check + clippy -D warnings + test.
# Desktop uses different clippy flags and is handled in ci-desktop.
# Tool crates have extra smoke/contract steps and are handled in ci-tools.
CI_RUST_MANIFESTS = \
    crates/artifact-extract/Cargo.toml \
    crates/axiomregent/Cargo.toml \
    crates/orchestrator/Cargo.toml \
    crates/policy-kernel/Cargo.toml \
    crates/tool-registry/Cargo.toml \
    crates/skill-factory/Cargo.toml \
    crates/factory-engine/Cargo.toml \
    crates/factory-contracts/Cargo.toml \
    crates/provider-registry/Cargo.toml \
    crates/agent-frontmatter/Cargo.toml \
    crates/standards-loader/Cargo.toml \
    platform/services/deployd-api-rs/Cargo.toml

ci-rust:
	@set -e; for m in $(CI_RUST_MANIFESTS); do \
	    echo ""; \
	    echo "==> ci-rust: $$m"; \
	    cargo check  --manifest-path $$m; \
	    cargo clippy --manifest-path $$m -- -D warnings; \
	    cargo test   --manifest-path $$m; \
	done

# registry-consumer contract gates (spec-conformance.yml)
CI_REGISTRY_CONSUMER_CONTRACTS = \
    readme_ \
    error_contract_ \
    shape_contract_ \
    help_contract_ \
    arg_contract_ \
    version_contract_ \
    default_path_contract_ \
    allow_invalid_contract_ \
    sorting_contract_ \
    channel_contract_

ci-tools:
	@echo "==> ci-tools: spec-compiler"
	cargo build --release --manifest-path tools/spec-compiler/Cargo.toml
	./tools/spec-compiler/target/release/spec-compiler compile
	cargo test --manifest-path tools/spec-compiler/Cargo.toml
	@echo ""
	@echo "==> ci-tools: registry-consumer (+ contract subsets)"
	cargo build --release --manifest-path tools/registry-consumer/Cargo.toml
	./tools/registry-consumer/target/release/registry-consumer list | head -n 5
	cargo test --manifest-path tools/registry-consumer/Cargo.toml
	@set -e; for c in $(CI_REGISTRY_CONSUMER_CONTRACTS); do \
	    echo "  contract gate: $$c"; \
	    cargo test --manifest-path tools/registry-consumer/Cargo.toml --all $$c; \
	done
	@echo ""
	@echo "==> ci-tools: spec-lint"
	cargo build --release --manifest-path tools/spec-lint/Cargo.toml
	./tools/spec-lint/target/release/spec-lint --fail-on-warn   # spec 128: strict posture (amends spec 006)
	cargo test --manifest-path tools/spec-lint/Cargo.toml
	@echo ""
	@echo "==> ci-tools: stakeholder-doc-lint (spec 122 FR-035)"
	cargo build --release --manifest-path tools/stakeholder-doc-lint/Cargo.toml
	cargo clippy --manifest-path tools/stakeholder-doc-lint/Cargo.toml -- -D warnings
	cargo test --manifest-path tools/stakeholder-doc-lint/Cargo.toml
	./tools/stakeholder-doc-lint/target/release/stakeholder-doc-lint --project . || true   # warnings non-blocking by default (FR-035)
	@echo ""
	@echo "==> ci-tools: codebase-indexer (+ staleness gate)"
	cargo build --release --manifest-path tools/codebase-indexer/Cargo.toml
	./tools/codebase-indexer/target/release/codebase-indexer check
	./tools/codebase-indexer/target/release/codebase-indexer compile
	cargo test --manifest-path tools/codebase-indexer/Cargo.toml
	@echo ""
	@echo "==> ci-tools: policy-compiler"
	cargo build --release --manifest-path tools/policy-compiler/Cargo.toml
	cargo test --manifest-path tools/policy-compiler/Cargo.toml
	@echo ""
	@echo "==> ci-tools: assumption-cascade-check (spec 121 FR-034)"
	cargo build --release --manifest-path tools/assumption-cascade-check/Cargo.toml
	cargo test --manifest-path tools/assumption-cascade-check/Cargo.toml
	./tools/assumption-cascade-check/target/release/assumption-cascade-check --repo .

ci-desktop:
	@# CI creates these stubs on fresh checkout; locally only if missing.
	@test -f apps/desktop/dist/index.html || { \
	    mkdir -p apps/desktop/dist; \
	    echo '<!doctype html><html><body>stub</body></html>' > apps/desktop/dist/index.html; \
	    echo "  (created dist stub)"; \
	}
	@HOST=$$(rustc -vV | grep '^host:' | awk '{print $$2}'); \
	 BIN=apps/desktop/src-tauri/binaries/axiomregent-$$HOST; \
	 if [ ! -f "$$BIN" ]; then \
	   mkdir -p apps/desktop/src-tauri/binaries; \
	   touch "$$BIN"; chmod +x "$$BIN"; \
	   echo "  (created sidecar stub: $$BIN)"; \
	 fi
	@echo "==> ci-desktop: rust (src-tauri)"
	cargo check  --manifest-path apps/desktop/src-tauri/Cargo.toml
	cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml -- -A dead_code -D warnings
	cargo test   --manifest-path apps/desktop/src-tauri/Cargo.toml --lib
	cargo test   --manifest-path apps/desktop/src-tauri/Cargo.toml --doc
	@echo ""
	@echo "==> ci-desktop: version alignment (Cargo.toml <-> package.json)"
	@CARGO_V=$$(grep '^version' apps/desktop/src-tauri/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	 PKG_V=$$(node -p "require('./apps/desktop/package.json').version"); \
	 if [ "$$CARGO_V" != "$$PKG_V" ]; then \
	   echo "ERROR: version mismatch — Cargo.toml=$$CARGO_V package.json=$$PKG_V"; exit 1; \
	 else \
	   echo "  versions aligned: $$CARGO_V"; \
	 fi
	@echo ""
	@echo "==> ci-desktop: typescript"
	pnpm install --frozen-lockfile
	pnpm --filter @opc/desktop exec tsc --noEmit
	pnpm --filter @opc/desktop test

ci-stagecraft: ci-agent-frontmatter-ts
	@echo "==> ci-stagecraft: npm ci + tsc + vitest"
	@# CI=true forces vitest to run-once instead of TTY watch mode.
	cd platform/services/stagecraft && CI=true npm ci && CI=true npx tsc --noEmit && CI=true npm test

# ============================================================
# Schema parity (spec 120 FR-003) — asserts the Rust mirror in
# `crates/factory-contracts/src/knowledge.rs` and the TS source-of-truth
# in `platform/services/stagecraft/api/knowledge/extractionOutput.ts`
# describe the same shape. Drift fails CI before any runtime divergence
# can ship.
#
# Step 1 emits the Rust-side fingerprints via `cargo test`. Step 2 walks
# the TS side with bun (which handles .ts natively): every schema walks
# a plain-data `SchemaNode` descriptor co-located with its hand-rolled
# validator (spec 125, no zod — Encore parser invariant). Provenance and
# stakeholder-doc surfaces are in reserved mode until their TS mirrors
# land at the paths spec 121 §8 / 122 reserve.
# ============================================================

ci-schema-parity:
	@echo "==> ci-schema-parity: emit rust fingerprints (knowledge + provenance + stakeholder_docs)"
	cargo test --manifest-path crates/factory-contracts/Cargo.toml --lib -- \
	    knowledge::tests::writes_fingerprint_file \
	    provenance::tests::writes_provenance_fingerprint_file \
	    stakeholder_docs::tests::writes_stakeholder_docs_fingerprint_file
	@echo ""
	@echo "==> ci-schema-parity: walk TS descriptors and compare"
	bun run tools/schema-parity-check/index.mjs

# ============================================================
# Spec/code coupling (spec 127) — mirrors
# .github/workflows/ci-spec-code-coupling.yml.
#
# PR-time gate: any diff path claimed by a spec's `implements:` list must
# be accompanied by an edit to that spec's spec.md. Locally this defaults
# to `origin/main...HEAD`; override BASE_REF/HEAD_REF on the command line
# (e.g. `make ci-spec-code-coupling BASE_REF=HEAD~3`).
# ============================================================

ci-spec-code-coupling:
	@echo "==> ci-spec-code-coupling: build + run gate"
	cargo build --release --manifest-path tools/spec-code-coupling-check/Cargo.toml
	cargo test --manifest-path tools/spec-code-coupling-check/Cargo.toml
	@# Local mirror of .github/workflows/ci-spec-code-coupling.yml. CI passes
	@# explicit base/head SHAs via --base/--head; locally we materialise the
	@# working-tree-vs-origin/main diff (committed + staged + unstaged) plus
	@# untracked-but-not-ignored new files so uncommitted edits AND new files
	@# participate in the self-test. Override BASE_REF on the command line.
	@paths_file=$$(mktemp); \
	  base=$(or $(BASE_REF),origin/main); \
	  { git diff --name-only $$base; git ls-files --others --exclude-standard; } \
	      | sort -u > $$paths_file; \
	  ./tools/spec-code-coupling-check/target/release/spec-code-coupling-check \
	      --base $$base --head HEAD --paths-from $$paths_file; \
	  status=$$?; rm -f $$paths_file; exit $$status

# ============================================================
# Supply chain (spec 116) — mirrors .github/workflows/ci-supply-chain.yml.
# Posture: blocking from day 0 (spec 116 §9 — warn window collapsed 2026-05-02).
# ============================================================

ci-supply-chain: ci-supply-chain-cargo ci-supply-chain-pnpm ci-supply-chain-npm
	@echo ""
	@echo "==> ci-supply-chain: all gates passed."

# cargo-deny scans every Rust manifest. No top-level Cargo.toml exists,
# so iterate; the workspace `crates/Cargo.toml` covers all 16 member crates.
SUPPLY_CHAIN_RUST_MANIFESTS = \
    crates/Cargo.toml \
    platform/services/deployd-api-rs/Cargo.toml \
    apps/desktop/src-tauri/Cargo.toml \
    tools/spec-compiler/Cargo.toml \
    tools/registry-consumer/Cargo.toml \
    tools/spec-lint/Cargo.toml \
    tools/stakeholder-doc-lint/Cargo.toml \
    tools/codebase-indexer/Cargo.toml \
    tools/policy-compiler/Cargo.toml \
    tools/adapter-scopes-compiler/Cargo.toml \
    tools/assumption-cascade-check/Cargo.toml \
    tools/ci-parity-check/Cargo.toml \
    tools/shared/frontmatter/Cargo.toml

ci-supply-chain-cargo:
	@echo "==> ci-supply-chain: cargo-deny"
	@command -v cargo-deny >/dev/null 2>&1 || cargo install cargo-deny --locked --version '^0.19'
	@for m in $(SUPPLY_CHAIN_RUST_MANIFESTS); do \
	    echo "  cargo deny --manifest-path $$m check"; \
	    cargo deny --manifest-path $$m check; \
	done

ci-supply-chain-pnpm:
	@echo "==> ci-supply-chain: pnpm audit"
	pnpm audit --audit-level=high

ci-supply-chain-npm:
	@echo "==> ci-supply-chain: npm audit (stagecraft)"
	cd platform/services/stagecraft && npm audit --audit-level=high

# axiomregent cross-target matrix (build-axiomregent.yml). Opt-in.
# Prerequisite per target: rustup target add <triple>
CI_CROSS_TARGETS = \
    aarch64-apple-darwin \
    x86_64-unknown-linux-gnu \
    x86_64-pc-windows-msvc \
    aarch64-unknown-linux-gnu

ci-cross:
	@set -e; for t in $(CI_CROSS_TARGETS); do \
	    echo "==> ci-cross: cargo build --release --target $$t --manifest-path crates/axiomregent/Cargo.toml"; \
	    cargo build --release --target $$t --manifest-path crates/axiomregent/Cargo.toml; \
	done

# Parity drift check (spec 104): asserts `make ci` mirrors every enforcing
# workflow's `run:` blocks. Not included in `ci` to avoid circular failure —
# CI runs it independently via .github/workflows/ci-parity.yml.
ci-parity:
	cargo build --release --manifest-path tools/ci-parity-check/Cargo.toml
	./tools/ci-parity-check/target/release/ci-parity-check

# BEGIN ci-fast (spec 134)
# ============================================================
# Fast local CI (spec 134) — performance-optimised local validation.
# Parity-exempt by design: lines between this BEGIN sentinel and the
# corresponding `# END ci-fast` are skipped by `tools/ci-parity-check`.
# Bound instead by the spec 134 §2.3 coverage invariant: the gate set
# performed here MUST be a superset of `make ci`.
#
# Reference hardware: M1 Pro 10c / 64 GB. Targets (aspirational, not
# pass/fail): cold ≤ 50 min, warm ≤ 25 min. Measurement commit pending
# at docs/ci-fast-bench.md (SC-01).
#
# Tunables (env or `make CIFAST_JOBS=N ci-fast`):
#   CIFAST_JOBS         outer concurrency (default 4)
#
# Auto-detected accelerators (no-op if absent):
#   sccache         shared compilation cache via RUSTC_WRAPPER
#   cargo-nextest   replaces cargo test (strict superset for execution)
# ============================================================

CIFAST_JOBS         ?= 4
CIFAST_TARGET_DIR   ?= $(CURDIR)/.target/cifast-tools

ifneq (,$(shell command -v sccache 2>/dev/null))
  export RUSTC_WRAPPER := $(shell command -v sccache)
endif
ifneq (,$(shell command -v cargo-nextest 2>/dev/null))
  # `--no-tests=pass` matches `cargo test` semantics: a binary with zero
  # `#[test]` functions exits 0 silently. Without this, nextest errors
  # with "no tests to run" on workspace members whose `tests/` dirs (or
  # `examples/`, `benches/` under --all-targets) contain no test fns.
  CIFAST_CARGO_TEST := nextest run --no-tests=pass
else
  CIFAST_CARGO_TEST := test
endif

ci-fast:
	@echo "==> ci-fast (spec 134): parallel local validation"
	@echo "    sccache:  $(if $(RUSTC_WRAPPER),enabled ($(RUSTC_WRAPPER)),absent — install: brew install sccache)"
	@echo "    nextest:  $(if $(filter nextest run,$(CIFAST_CARGO_TEST)),enabled,absent — install: cargo install cargo-nextest)"
	@echo ""
	@$(MAKE) -j$(CIFAST_JOBS) \
	    ci-fast-rust ci-fast-tools ci-fast-desktop \
	    ci-fast-stagecraft ci-fast-schema-parity \
	    ci-fast-spec-coupling ci-fast-supply-chain
	@echo ""
	@echo "==> ci-fast: all gates passed."

# Workspace-mode for crates/ collapses 11 of 12 CI_RUST_MANIFESTS entries
# to one clippy + one test invocation. deployd-api-rs (the 12th) runs as
# a concurrent sibling. `cargo clippy --all-targets -- -D warnings`
# subsumes the separate `cargo check` step (spec 134 §2.2(2)).
ci-fast-rust:
	@echo "==> ci-fast-rust: crates/ workspace + deployd-api-rs (concurrent)"
	@# Drop `--jobs` from cargo invocations: under `make -j` the jobserver
	@# already throttles, and explicit `--jobs` is silently ignored with a
	@# warning per invocation (matches the ci-fast-tools fix in PR #78).
	@( cargo clippy --workspace \
	      --manifest-path crates/Cargo.toml --all-targets -- -D warnings && \
	   cargo $(CIFAST_CARGO_TEST) --workspace \
	      --manifest-path crates/Cargo.toml ) & WS_PID=$$!; \
	  ( cargo clippy \
	      --manifest-path platform/services/deployd-api-rs/Cargo.toml \
	      --all-targets -- -D warnings && \
	    cargo $(CIFAST_CARGO_TEST) \
	      --manifest-path platform/services/deployd-api-rs/Cargo.toml ) & DA_PID=$$!; \
	  wait $$WS_PID; W=$$?; wait $$DA_PID; D=$$?; exit $$((W | D))

# Tools — parallel xargs fan-out, shared CARGO_TARGET_DIR so the 7 isolated
# manifests dedup deps. The 10× registry-consumer contract subset loop in
# `ci-tools` is dropped here per spec 134 §2.2(4): execution is subsumed
# by the unfiltered `cargo test --manifest-path tools/registry-consumer/Cargo.toml`,
# and the dropped loop's prefix-existence guarantee is preserved by the
# explicit `cargo test -- --list` post-pass below.
CIFAST_TOOL_MANIFESTS = \
    tools/spec-compiler/Cargo.toml \
    tools/registry-consumer/Cargo.toml \
    tools/spec-lint/Cargo.toml \
    tools/stakeholder-doc-lint/Cargo.toml \
    tools/codebase-indexer/Cargo.toml \
    tools/policy-compiler/Cargo.toml \
    tools/assumption-cascade-check/Cargo.toml

ci-fast-tools:
	@mkdir -p $(CIFAST_TARGET_DIR)
	@echo "==> ci-fast-tools: $(words $(CIFAST_TOOL_MANIFESTS)) manifests, shared target dir"
	@# BSD xargs (macOS) caps `-I{}` replacement at 255 bytes by default and
	@# fails this recipe with "command line cannot be assembled, too long".
	@# Pass the manifest as a positional arg (`$$1`) instead of substituting `{}`.
	@# Drop `--jobs` from cargo invocations: under `make -j` the jobserver
	@# already throttles, and explicit `--jobs` is silently ignored with a
	@# warning per invocation. && chains short-circuit on first failure.
	@printf '%s\n' $(CIFAST_TOOL_MANIFESTS) | \
	  xargs -n1 -P$(CIFAST_JOBS) sh -c '\
	    m="$$1"; \
	    echo "  [start] $$m"; \
	    CARGO_TARGET_DIR=$(CIFAST_TARGET_DIR) cargo clippy --manifest-path "$$m" --all-targets -- -D warnings && \
	    CARGO_TARGET_DIR=$(CIFAST_TARGET_DIR) cargo $(CIFAST_CARGO_TEST) --manifest-path "$$m" && \
	    echo "  [done ] $$m"' _
	@# Spec 134 §2.2(4): preserve the contract-prefix existence guarantee
	@# the dropped registry-consumer subset loop implicitly provided. Each
	@# prefix in CI_REGISTRY_CONSUMER_CONTRACTS MUST match ≥1 listed test.
	@TESTS=$$(mktemp); \
	 CARGO_TARGET_DIR=$(CIFAST_TARGET_DIR) cargo test \
	    --manifest-path tools/registry-consumer/Cargo.toml -- --list \
	    > $$TESTS 2>&1; \
	 status=0; \
	 for p in $(CI_REGISTRY_CONSUMER_CONTRACTS); do \
	    grep -q "^$$p" $$TESTS || { \
	      echo "ERROR: contract prefix '$$p' has no matching test"; status=1; \
	    }; \
	 done; \
	 rm -f $$TESTS; exit $$status
	@# Spec-lint smoke + codebase-indexer staleness gate (mirrors ci-tools).
	@CARGO_TARGET_DIR=$(CIFAST_TARGET_DIR) \
	  cargo run --release --manifest-path tools/spec-lint/Cargo.toml -- --fail-on-warn
	@CARGO_TARGET_DIR=$(CIFAST_TARGET_DIR) \
	  cargo run --release --manifest-path tools/codebase-indexer/Cargo.toml -- check

ci-fast-desktop:
	@test -f apps/desktop/dist/index.html || { mkdir -p apps/desktop/dist; \
	    echo '<!doctype html><html><body>stub</body></html>' > apps/desktop/dist/index.html; }
	@HOST=$$(rustc -vV | grep '^host:' | awk '{print $$2}'); \
	 BIN=apps/desktop/src-tauri/binaries/axiomregent-$$HOST; \
	 [ -f "$$BIN" ] || { mkdir -p $$(dirname "$$BIN"); touch "$$BIN"; chmod +x "$$BIN"; }
	@echo "==> ci-fast-desktop: rust + pnpm install (concurrent)"
	@# `--jobs` dropped: under `make -j` the jobserver throttles cargo;
	@# explicit `--jobs` is silently ignored with a warning (PR #78 precedent).
	@( cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml \
	     --all-targets -- -A dead_code -D warnings && \
	   cargo $(CIFAST_CARGO_TEST) --manifest-path apps/desktop/src-tauri/Cargo.toml --lib && \
	   cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --doc \
	) & RUST_PID=$$!; \
	  pnpm install --frozen-lockfile; PI=$$?; \
	  wait $$RUST_PID; R=$$?; exit $$((R | PI))
	@echo "==> ci-fast-desktop: tsc | vitest (concurrent)"
	@( pnpm --filter @opc/desktop exec tsc --noEmit ) & TSC_PID=$$!; \
	  ( pnpm --filter @opc/desktop test ) & VT_PID=$$!; \
	  wait $$TSC_PID; T=$$?; wait $$VT_PID; V=$$?; exit $$((T | V))
	@CARGO_V=$$(grep '^version' apps/desktop/src-tauri/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
	 PKG_V=$$(node -p "require('./apps/desktop/package.json').version"); \
	 [ "$$CARGO_V" = "$$PKG_V" ] || { echo "ERROR: version mismatch $$CARGO_V vs $$PKG_V"; exit 1; }

ci-fast-stagecraft: ci-agent-frontmatter-ts
	@echo "==> ci-fast-stagecraft: npm ci then (tsc | vitest)"
	cd platform/services/stagecraft && CI=true npm ci
	@# Each backgrounded compound needs its own `cd` — bash treats
	@# `cd X && cmd &` as a backgrounded subshell, so the parent shell's
	@# CWD doesn't change. Without this fix, only the first job runs in
	@# stagecraft/; the second runs from repo root and `npm test` fails
	@# with "Missing script: test" (the workspace root has no test script).
	@( cd platform/services/stagecraft && CI=true npx tsc --noEmit ) & TSC_PID=$$!; \
	  ( cd platform/services/stagecraft && CI=true npm test ) & VT_PID=$$!; \
	  wait $$TSC_PID; T=$$?; wait $$VT_PID; V=$$?; exit $$((T | V))

ci-fast-schema-parity:
	cargo test --manifest-path crates/factory-contracts/Cargo.toml --lib -- \
	    knowledge::tests::writes_fingerprint_file \
	    provenance::tests::writes_provenance_fingerprint_file \
	    stakeholder_docs::tests::writes_stakeholder_docs_fingerprint_file
	bun run tools/schema-parity-check/index.mjs

ci-fast-spec-coupling:
	cargo build --release --manifest-path tools/spec-code-coupling-check/Cargo.toml
	@paths_file=$$(mktemp); \
	  base=$(or $(BASE_REF),origin/main); \
	  { git diff --name-only $$base; git ls-files --others --exclude-standard; } \
	      | sort -u > $$paths_file; \
	  ./tools/spec-code-coupling-check/target/release/spec-code-coupling-check \
	      --base $$base --head HEAD --paths-from $$paths_file; \
	  status=$$?; rm -f $$paths_file; exit $$status

ci-fast-supply-chain:
	@command -v cargo-deny >/dev/null 2>&1 || cargo install cargo-deny --locked --version '^0.19'
	@echo "==> ci-fast-supply-chain: cargo-deny -P$(CIFAST_JOBS) | pnpm audit | npm audit"
	@( pnpm audit --audit-level=high ) & PNPM_PID=$$!; \
	  ( cd platform/services/stagecraft && npm audit --audit-level=high ) & NPM_PID=$$!; \
	  printf '%s\n' $(SUPPLY_CHAIN_RUST_MANIFESTS) | \
	    xargs -n1 -P$(CIFAST_JOBS) -I{} cargo deny --manifest-path {} check; \
	  CD=$$?; \
	  wait $$PNPM_PID; PA=$$?; wait $$NPM_PID; NA=$$?; \
	  exit $$((CD | PA | NA))

# END ci-fast

# ============================================================
# Utility
# ============================================================

## Remove build outputs the spec/index compilers and the desktop bundle write.
## Does NOT clean cargo target dirs under crates/ or tools/ — use
## `cargo clean --manifest-path <path>` for those (preserves cargo cache by default).
clean:
	@echo "==> Cleaning build artifacts..."
	rm -rf build/spec-registry
	rm -rf build/codebase-index
	rm -rf build/schema-parity
	rm -rf apps/desktop/dist
	rm -rf apps/desktop/src-tauri/target

help:
	@echo "Open Agentic Platform"
	@echo ""
	@echo "Quick start:"
	@echo "  make setup          One-time: install deps, build tools, compile specs"
	@echo "  make dev            Start desktop app (Vite + Tauri, hot-reload)"
	@echo ""
	@echo "Platform services (optional):"
	@echo "  make dev-platform   Start stagecraft + deployd-api in background"
	@echo "  make dev-all        Desktop + platform services"
	@echo "  make stop           Stop background platform services"
	@echo ""
	@echo "Specs:"
	@echo "  make registry             Recompile spec registry + codebase index"
	@echo "  make spec-compile         Recompile spec registry only"
	@echo "  make spec-tools           Build all spec CLI tools"
	@echo ""
	@echo "Index:"
	@echo "  make index                Recompile codebase index"
	@echo "  make index-check          Check if index is stale"
	@echo "  make index-render         Render CODEBASE-INDEX.md from index"
	@echo ""
	@echo "agent-frontmatter (ts-rs mirror, spec 111):"
	@echo "  make agent-frontmatter-ts     Regenerate the TS bindings (write-through)"
	@echo "  make ci-agent-frontmatter-ts  Regenerate + fail if working tree drifts"
	@echo ""
	@echo "CI parity (mirrors .github/workflows):"
	@echo "  make ci                 Run every CI gate locally (composes ci-rust, ci-tools, ci-desktop, ci-stagecraft, ci-supply-chain). Pre-push parity gate. ~90 min on M1 Pro."
	@echo "  make ci-fast            Spec 134 — parallel local validation, parity-exempt. Inner-loop default. Target ≤ 25 min warm on M1 Pro 10c / 64 GB."
	@echo "  make ci-rust            All Rust manifests: check + clippy -D warnings + test"
	@echo "  make ci-tools           Spec tool crates + registry-consumer contract subsets + staleness gate"
	@echo "  make ci-desktop         apps/desktop rust + version alignment + tsc + vitest"
	@echo "  make ci-stagecraft      platform/services/stagecraft: npm ci + tsc + vitest"
	@echo "  make ci-spec-code-coupling  PR-time spec/code coupling gate (spec 127)"
	@echo "  make ci-supply-chain    cargo-deny + pnpm/npm audit (spec 116; blocking)"
	@echo "  make ci-cross           axiomregent cross-target matrix (opt-in; requires rustup targets)"
	@echo "  make ci-parity          Drift check: Makefile mirrors enforcing workflows (spec 104)"
	@echo ""
	@echo "Kubernetes:"
	@echo "  make deploy-azure   Deploy to Azure AKS"
	@echo "  make deploy-aws     Deploy to AWS EKS"
	@echo "  make deploy-hetzner Deploy to Hetzner K3s"
	@echo ""
	@echo "Sidecar:"
	@echo "  make axiomregent             Build axiomregent sidecar for host triple"
	@echo "  make axiomregent-all         Build for every target and install into sidecar dir"
	@echo "  make fetch-axiomregent       Download pre-built sidecar from GitHub Release (gh CLI)"
	@echo "  make fetch-axiomregent-check Fetch only if sidecar is missing"
	@echo ""
	@echo "Other:"
	@echo "  make clean          Remove build artifacts"
	@echo "  make check-deps     Verify prerequisites are installed"
