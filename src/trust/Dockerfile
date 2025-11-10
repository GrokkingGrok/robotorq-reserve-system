# -------------------------------
# Stage 1: Build the Go binary
# -------------------------------
FROM golang:1.24-alpine AS builder
# Use an official lightweight Go image (based on Alpine Linux) as the build environment.
# This stage compiles the Go application into a statically linked binary.

WORKDIR /app
# Set the working directory inside the container to /app.
# All following commands will be run relative to this directory.

COPY go.mod go.sum ./
# Copy Go module files into the container.
# These files define module dependencies and checksums.

RUN go mod download
# Download all Go dependencies before copying the rest of the source code.
# Doing this early allows Docker to cache this layer for faster rebuilds when code changes.

COPY . .
# Copy the entire project source code into the container’s /app directory.

RUN go mod tidy
# Verify that go.mod and go.sum are in sync and remove any unused dependencies.
# This step ensures a clean and reproducible build.

RUN CGO_ENABLED=0 GOOS=linux go build -o /trust ./cmd/trust
# Compile the Go application for Linux with CGO disabled (static binary, no C dependencies).
# Output the binary to the root of the container as /trust.
# The build target "./cmd/trust" refers to your main package location.

# -------------------------------
# Stage 2: Build the final runtime image
# -------------------------------
FROM alpine:3.20
# Start a new image using a minimal Alpine base.
# This stage only includes what’s needed to run the built application — not Go or build tools.

RUN apk add --no-cache ca-certificates
# Install trusted CA certificates so HTTPS requests from the app will work.
# The --no-cache flag keeps the image small by not storing package index files.

COPY --from=builder /trust /trust
# Copy the compiled binary from the builder stage into this new lightweight image.

# COPY --from=builder /app/certs /certs
# (Optional) Example line for copying certificates if your app needs them.
# It’s commented out here but left as a reference for future secure setups.

EXPOSE 8080
# Document that the container listens on and 8080 (HTTP/health).
# EXPOSE doesn’t actually publish the ports — it’s metadata for users of this image.

HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:8080/health || exit 1
# Define a container health check to ensure the service is responding.
# Every 30 seconds, it makes an HTTP request to /health on port 8080.
# If the request fails or times out after 3s, Docker marks the container as unhealthy.

CMD ["/trust"]
# Set the default command to run the compiled binary.
# When the container starts, it executes the /trust program.
