# Spec 136 Phase 2.b: deployd-api now drives deploys through `helm` against
# charts embedded into the binary via include_str!. The Docker build context
# is broadened to `platform/` (set in .github/workflows/cd-deployd-api-rs.yml)
# so the relative include_str! paths from src/helm.rs reach
# ../../../charts/<chart>/... at compile time. The runtime image bundles the
# helm CLI so the binary can shell out at deploy time.

FROM rust:1.88 AS builder
WORKDIR /repo
COPY services/deployd-api-rs ./services/deployd-api-rs
COPY charts ./charts
WORKDIR /repo/services/deployd-api-rs
RUN cargo build --release

FROM debian:bookworm-slim

ARG HELM_VERSION=v3.16.4
# TARGETARCH is populated by BuildKit from --platform (e.g. amd64, arm64)
# and falls back to the host arch when --platform is omitted. Declaring the
# ARG inside this stage brings it into scope for the RUN below; the helm
# release tarball follows the same `amd64` / `arm64` naming.
ARG TARGETARCH

# `tar` is already present in bookworm-slim (essential dpkg dependency);
# adding it to apt-get install confuses the resolver, and purging it would
# break dpkg itself. Stream the tarball through curl|tar instead.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && curl -fsSL "https://get.helm.sh/helm-${HELM_VERSION}-linux-${TARGETARCH}.tar.gz" \
        | tar xz -C /tmp \
    && install -m 0755 "/tmp/linux-${TARGETARCH}/helm" /usr/local/bin/helm \
    && rm -rf "/tmp/linux-${TARGETARCH}" \
    && apt-get purge -y curl \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/sh deployd

COPY --from=builder /repo/services/deployd-api-rs/target/release/deployd-api /usr/local/bin/

USER deployd
EXPOSE 8080
CMD ["deployd-api"]
