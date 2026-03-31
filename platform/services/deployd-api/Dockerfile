FROM node:20-alpine AS deps
WORKDIR /app
COPY package.json package-lock.json* ./
RUN npm ci || npm i

FROM node:20-alpine AS build
WORKDIR /app
COPY package.json ./
COPY --from=deps /app/node_modules ./node_modules
COPY tsconfig.json ./
COPY src ./src
RUN npm run build

FROM node:20-alpine
WORKDIR /app
ENV NODE_ENV=production

# Install helm (MVP)
RUN apk add --no-cache curl ca-certificates bash openssl \
  && curl -fsSL https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash

COPY package.json ./
COPY --from=build /app/dist ./dist
COPY --from=deps /app/node_modules ./node_modules
COPY charts ./charts

EXPOSE 8080
CMD ["node", "dist/index.js"]
