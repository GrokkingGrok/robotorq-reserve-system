# -------------------------------
# Stage 1: Build the Go binary
# -------------------------------
FROM golang:1.25-alpine AS builder

WORKDIR /app

# Copy natsx shared module first (needed for replace directive)
COPY natsx /natsx

# Copy distodam module files
COPY distodam/go.mod distodam/go.sum ./

RUN go mod download

COPY distodam/ .

RUN go mod tidy

RUN CGO_ENABLED=0 GOOS=linux go build -o /distodam ./cmd/distodam

# -------------------------------
# Stage 2: Build the final runtime image
# -------------------------------
FROM alpine:3.20

RUN apk add --no-cache ca-certificates

COPY --from=builder /distodam /distodam

ENV PORT=8082

EXPOSE 8082

HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:8082/health || exit 1

CMD ["/distodam"]
