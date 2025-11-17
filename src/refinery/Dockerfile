# -------------------------------
# Stage 1: Build the Go binary with liboqs support
# -------------------------------
FROM golang:1.24-alpine AS builder

WORKDIR /app

# Install liboqs for Falcon-1024 signature verification (Phase 5)
# Use pre-built package instead of compiling from source (faster builds)
RUN apk add --no-cache \
    build-base \
    pkgconfig \
    liboqs-dev

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
RUN apk add --no-cache ca-certificates wget liboqs

COPY --from=builder /refinery /refinery

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:8080/health || exit 1

CMD ["/refinery"]
