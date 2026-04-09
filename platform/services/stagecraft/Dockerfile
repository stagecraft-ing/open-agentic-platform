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

FROM node:22-slim AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev

FROM node:22-slim
WORKDIR /app

# Production node_modules only (~83 MB vs 197 MB)
COPY --from=deps /app/node_modules ./node_modules

# Compiled encore application (combined entrypoint)
COPY .encore/build/combined/combined/main.mjs ./main.mjs

# Encore native runtime for linux
COPY encore-runtime.node /app/encore-runtime.node

# Pre-built React Router frontend (static assets + SSR)
COPY web/build/client/ ./build/client/
COPY web/build/server/ ./build/server/

ENV ENCORE_RUNTIME_LIB=/app/encore-runtime.node
ENV NODE_ENV=production

EXPOSE 8080
CMD ["node", "main.mjs"]
