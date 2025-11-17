# -------------------------------
# Stage 1: Build the Go binary with liboqs support
# -------------------------------
FROM golang:1.24-alpine AS builder

WORKDIR /app

# Install liboqs for Falcon-1024 signature verification (Phase 5)
RUN apk add --no-cache \
    git \
    cmake \
    ninja \
    build-base \
    pkgconfig \
    openssl-dev \
    linux-headers \
    && git clone --branch main --depth 1 https://github.com/open-quantum-safe/liboqs.git /tmp/liboqs \
    && cd /tmp/liboqs \
    && mkdir build && cd build \
    && cmake -GNinja -DCMAKE_INSTALL_PREFIX=/usr .. \
    && ninja install \
    && rm -rf /tmp/liboqs

COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN go mod tidy

# Build with CGO enabled for liboqs (Falcon-1024 verification)
RUN CGO_ENABLED=1 GOOS=linux go build -o /refinery ./cmd/refinery

# -------------------------------
# Stage 2: Build the final runtime image
# -------------------------------
FROM alpine:3.20

# Install runtime dependencies for liboqs
RUN apk add --no-cache ca-certificates wget libstdc++ libgcc

COPY --from=builder /refinery /refinery
COPY --from=builder /usr/lib/liboqs.so* /usr/lib/

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:8080/health || exit 1

CMD ["/refinery"]
