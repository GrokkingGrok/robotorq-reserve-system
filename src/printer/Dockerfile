# Multi-stage build for RoboTorq Printer Service
# DEV BUILD: Uses Debian for 10x faster compilation (glibc vs musl)
# TODO: Switch back to Alpine for production (smaller image)

# Build stage - use Debian slim for FAST compilation
FROM rust:1.91-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage - also use Debian for dev (consistency)
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/printer .

# Copy config
COPY config.yaml .

# Create data directory for certificates
RUN mkdir -p /app/data

# Expose metrics port
EXPOSE 9091

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -qO- http://localhost:9091/metrics || exit 1

CMD ["./printer"]
