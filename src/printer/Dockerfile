# Multi-stage build for RoboTorq Printer Service

# Build stage - use Rust 1.91 for edition2024 support
FROM rust:1.91-alpine AS builder

# Install build dependencies and build liboqs from source
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig \
    git \
    cmake \
    make \
    gcc \
    g++ \
    ninja

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

# Build release binary
RUN cargo build --release

# Runtime stage
FROM alpine:3.20

# Install runtime dependencies and build liboqs for runtime
RUN apk add --no-cache \
    ca-certificates \
    openssl \
    openssl-dev \
    libgcc \
    libstdc++ \
    git \
    cmake \
    make \
    gcc \
    g++ \
    ninja && \
    git clone --depth 1 --branch 0.12.0 https://github.com/open-quantum-safe/liboqs.git /tmp/liboqs && \
    cd /tmp/liboqs && \
    mkdir build && cd build && \
    cmake -GNinja -DCMAKE_INSTALL_PREFIX=/usr .. && \
    ninja && \
    ninja install && \
    cd / && rm -rf /tmp/liboqs && \
    apk del git cmake make gcc g++ ninja openssl-dev

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
