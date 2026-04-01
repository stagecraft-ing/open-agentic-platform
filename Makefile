# Open Agentic Platform — Root Makefile
#
# Quick start:
#   make setup   # one-time: install deps, build tools, compile spec registry
#   make dev     # start desktop app (Vite + Tauri with hot-reload)
#
# Platform services (optional, for org policy/auth work):
#   make dev-platform   # start stagecraft + deployd-api in background
#   make dev-all        # desktop + platform services
#
# Full K8s local cluster (staging fidelity):
#   make k8s-up         # bootstrap k3d cluster and deploy everything
#   make k8s-down       # tear down local cluster

.PHONY: setup dev dev-platform dev-all stop \
        spec-compile spec-tools \
        k8s-up k8s-down \
        check-deps

# ============================================================
# Prerequisites check
# ============================================================

check-deps:
	@echo "Checking prerequisites..."
	@command -v rustc  >/dev/null 2>&1 || { echo "  MISSING: rust    — curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; exit 1; }
	@command -v pnpm   >/dev/null 2>&1 || { echo "  MISSING: pnpm    — brew install pnpm"; exit 1; }
	@command -v bun    >/dev/null 2>&1 || { echo "  MISSING: bun     — brew install bun"; exit 1; }
	@command -v node   >/dev/null 2>&1 || { echo "  MISSING: node    — brew install node"; exit 1; }
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
	@echo "==> Setup complete. Run 'make dev' to start."

# ============================================================
# Spec tools
# ============================================================

spec-compile:
	./tools/spec-compiler/target/release/spec-compiler compile

spec-tools:
	cargo build --release --manifest-path tools/spec-compiler/Cargo.toml
	cargo build --release --manifest-path tools/registry-consumer/Cargo.toml
	cargo build --release --manifest-path tools/spec-lint/Cargo.toml

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
	@echo "==> Starting deployd-api (Express.js, port 8080)..."
	cd platform/services/deployd-api && npm install --silent && npm run dev

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
	-@pkill -f "tsx watch.*deployd" 2>/dev/null || true
	@echo "Done."

# ============================================================
# Local K8s Cluster (k3d)
# ============================================================

k8s-up:
	@test -f platform/infra/local/.env || { echo "ERROR: Copy platform/infra/local/.env.example to .env first"; exit 1; }
	cd platform && $(MAKE) bootstrap TARGET=local
	cd platform && $(MAKE) deploy TARGET=local

k8s-down:
	cd platform && $(MAKE) destroy TARGET=local

# ============================================================
# Cloud deployment (delegates to platform/Makefile)
# ============================================================

deploy-%:
	cd platform && $(MAKE) deploy TARGET=$*

destroy-%:
	cd platform && $(MAKE) destroy TARGET=$*

# ============================================================
# Utility
# ============================================================

clean:
	@echo "==> Cleaning build artifacts..."
	rm -rf build/spec-registry
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
	@echo "  make spec-compile   Recompile spec registry"
	@echo "  make spec-tools     Build all spec CLI tools"
	@echo ""
	@echo "Kubernetes:"
	@echo "  make k8s-up         Bootstrap local k3d cluster + deploy"
	@echo "  make k8s-down       Tear down local cluster"
	@echo "  make deploy-azure   Deploy to Azure AKS"
	@echo "  make deploy-aws     Deploy to AWS EKS"
	@echo "  make deploy-hetzner Deploy to Hetzner K3s"
	@echo ""
	@echo "Other:"
	@echo "  make clean          Remove build artifacts"
	@echo "  make check-deps     Verify prerequisites are installed"
