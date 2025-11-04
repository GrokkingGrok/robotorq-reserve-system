FROM golang:1.24-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN go mod tidy
RUN CGO_ENABLED=0 GOOS=linux go build -o /oracle ./cmd/oracle

FROM alpine:3.20
RUN apk add --no-cache ca-certificates
COPY --from=builder /oracle /oracle
# COPY --from=builder /app/certs /certs
EXPOSE 50052 8080
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:8080/health || exit 1
CMD ["/oracle"]
