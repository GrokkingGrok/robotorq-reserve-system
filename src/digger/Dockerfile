# Dockerfile for Digger Service
# Multi-stage build: Build Rust binary, then copy to minimal runtime image

FROM rust:1.91-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

WORKDIR /app

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release --bin digger

# Runtime stage - minimal Alpine image
FROM alpine:3.20

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/digger .

# Create directory for JTU storage
RUN mkdir -p /app/jtu_storage

# Expose HTTP API port
EXPOSE 3030

# Health check
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
  CMD wget -qO- http://localhost:3030/health || exit 1

# Run the binary
CMD ["./digger"]
