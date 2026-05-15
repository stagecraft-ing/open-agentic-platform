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

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl tar \
    && curl -fsSL "https://get.helm.sh/helm-${HELM_VERSION}-linux-amd64.tar.gz" -o /tmp/helm.tgz \
    && tar xzf /tmp/helm.tgz -C /tmp \
    && install -m 0755 /tmp/linux-amd64/helm /usr/local/bin/helm \
    && rm -rf /tmp/helm.tgz /tmp/linux-amd64 \
    && apt-get purge -y curl tar \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/sh deployd

COPY --from=builder /repo/services/deployd-api-rs/target/release/deployd-api /usr/local/bin/

USER deployd
EXPOSE 8080
CMD ["deployd-api"]
