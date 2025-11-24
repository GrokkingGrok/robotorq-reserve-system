FROM rust:1.73 as builder
WORKDIR /usr/src/trust
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release || true

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /usr/src/trust/target/release/rust-trust ./rust-trust
EXPOSE 8082
CMD ["/app/rust-trust"]
# -------------------------------
# Stage 1: Build the Go binary
# -------------------------------
FROM golang:1.25-alpine AS builder

WORKDIR /app

# Copy natsx shared module first (needed for replace directive)
COPY natsx /natsx

# Copy trust module files
COPY trust/go.mod trust/go.sum ./

RUN go mod download

COPY trust/ .

RUN go mod tidy

RUN CGO_ENABLED=0 GOOS=linux go build -o /trust ./cmd/trust

# -------------------------------
# Stage 2: Build the final runtime image
# -------------------------------
FROM alpine:3.20

RUN apk add --no-cache ca-certificates

COPY --from=builder /trust /trust

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:8080/health || exit 1

CMD ["/trust"]
