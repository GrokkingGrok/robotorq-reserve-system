# Dockerfile for Digger Service
# Multi-stage build: Build Rust binary, then copy to minimal runtime image
# DEV BUILD: Uses Debian for 10x faster compilation (glibc vs musl)
# TODO: Switch back to Alpine for production (smaller image)

FROM rust:1.91-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release --bin digger

# Runtime stage - also use Debian for dev (consistency)
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/digger .

# Create directory for JTU storage
RUN mkdir -p /app/jtu_storage

# Expose HTTP API port
EXPOSE 3030

# Health check
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
  CMD curl -f http://localhost:3030/health || exit 1

# Run the binary
CMD ["./digger"]
