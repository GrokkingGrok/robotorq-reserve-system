# Multi-stage build for RoboTorq Printer Service

# Build stage - use Debian-based Rust for better libclang support
FROM rust:1.91-bookworm AS builder

# Install build dependencies and build liboqs from source
RUN apt-get update && apt-get install -y \
    build-essential \
    libssl-dev \
    pkg-config \
    git \
    cmake \
    ninja-build \
    clang \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

# Build liboqs from source
RUN git clone --depth 1 --branch 0.12.0 https://github.com/open-quantum-safe/liboqs.git /tmp/liboqs && \
    cd /tmp/liboqs && \
    mkdir build && cd build && \
    cmake -GNinja -DCMAKE_INSTALL_PREFIX=/usr .. && \
    ninja && \
    ninja install && \
    cd / && rm -rf /tmp/liboqs

WORKDIR /app

# Copy manifests
COPY Cargo.toml ./

# Copy source code
COPY src ./src

# Build release binary (Debian has proper libclang support)
RUN cargo build --release

# Runtime stage - use Debian slim for compatibility
FROM debian:bookworm-slim

# Install runtime dependencies and build liboqs for runtime
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libssl-dev \
    wget \
    git \
    cmake \
    ninja-build \
    build-essential \
    && git clone --depth 1 --branch 0.12.0 https://github.com/open-quantum-safe/liboqs.git /tmp/liboqs \
    && cd /tmp/liboqs \
    && mkdir build && cd build \
    && cmake -GNinja -DCMAKE_INSTALL_PREFIX=/usr .. \
    && ninja \
    && ninja install \
    && cd / && rm -rf /tmp/liboqs \
    && apt-get remove -y git cmake ninja-build build-essential libssl-dev \
    && apt-get autoremove -y \
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
