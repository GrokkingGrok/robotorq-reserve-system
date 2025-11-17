# -------------------------------
# Stage 1: Build the Go binary with liboqs support
# -------------------------------
FROM golang:1.24-alpine AS builder

WORKDIR /app

# Install liboqs for Falcon-1024 + SPHINCS+ signature verification (Phase 5)
# Compile from source with minimal algorithms (faster build)
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
    && cmake -GNinja \
        -DCMAKE_INSTALL_PREFIX=/usr \
        -DOQS_MINIMAL_BUILD="KEM_kyber_512;SIG_falcon_1024;SIG_sphincs_sha2_128f_simple" \
        -DOQS_USE_OPENSSL=ON \
        .. \
    && ninja install \
    && ldconfig /usr/lib \
    && rm -rf /tmp/liboqs

# Create fake liboqs-go.pc that liboqs-go wrapper expects
RUN echo "prefix=/usr" > /usr/lib/pkgconfig/liboqs-go.pc \
    && echo "exec_prefix=\${prefix}" >> /usr/lib/pkgconfig/liboqs-go.pc \
    && echo "libdir=\${exec_prefix}/lib" >> /usr/lib/pkgconfig/liboqs-go.pc \
    && echo "includedir=\${prefix}/include" >> /usr/lib/pkgconfig/liboqs-go.pc \
    && echo "" >> /usr/lib/pkgconfig/liboqs-go.pc \
    && echo "Name: liboqs-go" >> /usr/lib/pkgconfig/liboqs-go.pc \
    && echo "Description: liboqs wrapper for Go" >> /usr/lib/pkgconfig/liboqs-go.pc \
    && echo "Version: 0.0.1" >> /usr/lib/pkgconfig/liboqs-go.pc \
    && echo "Cflags: -I\${includedir}" >> /usr/lib/pkgconfig/liboqs-go.pc \
    && echo "Libs: -L\${libdir} -loqs -lcrypto" >> /usr/lib/pkgconfig/liboqs-go.pc

COPY go.mod go.sum ./

RUN go mod download

COPY . .

RUN go mod tidy

# Build with CGO enabled (pkg-config now finds our fake liboqs-go.pc)
RUN CGO_ENABLED=1 go build -o /mint ./cmd/mint

# -------------------------------
# Stage 2: Build the final runtime image
# -------------------------------
FROM alpine:3.20

# Install runtime dependencies for liboqs
RUN apk add --no-cache ca-certificates wget libstdc++ libgcc openssl

COPY --from=builder /mint /mint
COPY --from=builder /usr/lib/liboqs.so* /usr/lib/

EXPOSE 50051 8080 9090

HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:8080/health || exit 1

CMD ["/mint"]
