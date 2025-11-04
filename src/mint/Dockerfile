# src/mint/Dockerfile
FROM golang:1.23-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o /mint ./cmd/mint

FROM alpine:latest
RUN apk add --no-cache ca-certificates
COPY --from=builder /mint /mint
COPY --from=builder /app/certs /certs
EXPOSE 50051 8080
CMD ["/mint"]
