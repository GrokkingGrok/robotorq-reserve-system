# Dockerfile for Vault Service (CertVault + StakeVault MVP)
# Multi-stage build: compile Rust binary, then run in minimal runtime image.
# Dev build uses Debian slim for faster linking (glibc). Switch to Alpine + musl if you need a smaller image later.

# ---------- Build Stage ----------
FROM rust:1.71-slim-bookworm AS builder

# Install build dependencies (add others if future crypto libs required)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifest first for dependency caching
COPY Cargo.toml Cargo.lock ./
# Create an empty src tree to satisfy cargo, then build deps
RUN mkdir src && echo "fn main(){}" > src/main.rs && \
    cargo build --release || true

# Remove placeholder and copy real source
RUN rm -rf src
COPY src ./src

# Build release binary
RUN cargo build --release --bin vault

# ---------- Runtime Stage ----------
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/vault /app/vault

# Non-root user (optional; comment out if permissions issues arise)
# RUN useradd -r -u 1000 vault && chown vault:vault /app/vault
# USER vault

ENV RUST_LOG=vault=info
EXPOSE 8088

# Health check hitting HTTP /health
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
  CMD curl -f http://localhost:8088/health || exit 1

CMD ["/app/vault"]
