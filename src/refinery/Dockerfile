# -------------------------------
# Stage 1: Build the Go binary
# -------------------------------
FROM golang:1.24-alpine AS builder

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN go mod tidy

RUN CGO_ENABLED=0 GOOS=linux go build -o /refinery ./cmd/refinery

# -------------------------------
# Stage 2: Build the final runtime image
# -------------------------------
FROM alpine:3.20

RUN apk add --no-cache ca-certificates wget

COPY --from=builder /refinery /refinery

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:8080/health || exit 1

CMD ["/refinery"]
