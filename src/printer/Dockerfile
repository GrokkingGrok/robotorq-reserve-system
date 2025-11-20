# Multi-stage build for RoboTorq Printer Service

# Build stage - use Rust 1.91 Alpine
FROM rust:1.91-alpine AS builder

# Install build dependencies including static OpenSSL
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig

# Force static linking for OpenSSL
ENV OPENSSL_STATIC=1
ENV OPENSSL_LIB_DIR=/usr/lib
ENV OPENSSL_INCLUDE_DIR=/usr/include

WORKDIR /app

# Copy manifests
COPY Cargo.toml ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Runtime stage
FROM alpine:3.20

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    wget

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
