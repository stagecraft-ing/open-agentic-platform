# Manual Docker build for stagecraft.
# Bypasses `encore build docker` which has pathologically slow gzip
# layer compression in go-containerregistry (15+ min for 200MB).
#
# Prerequisites:
#   1. Run `encore build docker ...` briefly to compile main.mjs (it finishes in ~30s),
#      then cancel — OR ensure .encore/build/combined/combined/main.mjs is up to date.
#   2. Run `cd web && npx react-router build` to produce web/build/ if stale.
#   3. Copy the encore runtime into the build context:
#        cp ~/Library/Caches/encore/cache/bin/v1.56.5/linux/amd64/encore-runtime.node ./encore-runtime.node
#   4. Build the Rust helpers and stage them into the build context:
#        (cd ../../.. && cargo build --release --manifest-path crates/Cargo.toml \
#            -p artifact-extract -p factory-project-detect)
#        cp ../../../crates/target/release/artifact-extract ./artifact-extract
#        cp ../../../crates/target/release/factory-project-detect ./factory-project-detect

FROM node:22-slim AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev

FROM node:22-slim
WORKDIR /app

# Factory upstream sync shells out to `git clone` (api/factory/clone.ts),
# so the runtime needs a git binary. ca-certificates is pulled in as a
# transitive dep for HTTPS clones.
RUN apt-get update \
    && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Production node_modules only (~83 MB vs 197 MB)
COPY --from=deps /app/node_modules ./node_modules

# Compiled encore application (combined entrypoint)
COPY .encore/build/combined/combined/main.mjs ./main.mjs

# Encore native runtime for linux
COPY encore-runtime.node /app/encore-runtime.node

# Pre-built React Router frontend (static assets + SSR)
COPY web/build/client/ ./build/client/
COPY web/build/server/ ./build/server/

# Migration runner + SQL files (consumed by the Helm pre-install/pre-upgrade Job)
COPY scripts/migrate.mjs ./scripts/migrate.mjs
COPY api/db/migrations/ ./api/db/migrations/

# Rauthy seeder (spec 106 FR-002) — Helm pre-install/pre-upgrade hook.
COPY scripts/seed-rauthy.mjs ./scripts/seed-rauthy.mjs

# Rust helpers shelled out to by the API:
#   - artifact-extract (crates/artifact-extract)        — knowledge advance endpoint
#   - factory-project-detect (crates/factory-project-detect) — project import endpoint
# ARTIFACT_EXTRACT_BIN / FACTORY_PROJECT_DETECT_BIN in the chart point here.
COPY artifact-extract /usr/local/bin/artifact-extract
COPY factory-project-detect /usr/local/bin/factory-project-detect
RUN chmod +x /usr/local/bin/artifact-extract /usr/local/bin/factory-project-detect

ENV ENCORE_RUNTIME_LIB=/app/encore-runtime.node
ENV NODE_ENV=production
ENV ARTIFACT_EXTRACT_BIN=/usr/local/bin/artifact-extract
ENV FACTORY_PROJECT_DETECT_BIN=/usr/local/bin/factory-project-detect

EXPOSE 4000
CMD ["node", "main.mjs"]
