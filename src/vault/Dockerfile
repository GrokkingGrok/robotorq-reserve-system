# Dockerfile for Vault Service (aligns with digger pattern)
# Multi-stage: build Rust binary, minimal runtime.

FROM rust:1.91-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Dependency layer for cache
COPY Cargo.toml Cargo.lock ./

# Source (include extra bin file outside src)
COPY src ./src
COPY test_publisher.rs ./test_publisher.rs

# Build vault binary only
RUN cargo build --release --bin vault

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/vault .

ENV RUST_LOG=vault=info
EXPOSE 8088

HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
  CMD curl -f http://localhost:8088/health || exit 1

CMD ["./vault"]
