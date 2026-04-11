FROM rust:1.88 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/* \
    && useradd -ms /bin/sh deployd
COPY --from=builder /app/target/release/deployd-api /usr/local/bin/
USER deployd
EXPOSE 8080
CMD ["deployd-api"]
