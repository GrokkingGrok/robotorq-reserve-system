# src/wallet/Dockerfile — Rust WASM Wallet (Flutter Web)
# Build: Rust → WASM → Serve via Nginx
# Run: docker-compose up wallet
FROM rust:1.82 AS builder
WORKDIR /app

# Install wasm-pack + target
RUN rustup target add wasm32-unknown-unknown && \
    cargo install wasm-pack

# Copy wallet Rust code
COPY src/wallet/Cargo.toml src/wallet/Cargo.lock ./
RUN mkdir -p src && cargo fetch

COPY src/wallet/src ./src
COPY src/wallet/wasm ./wasm

# Build WASM
RUN wasm-pack build --target web --out-dir pkg --release

# Final image: Nginx serves Flutter + WASM
FROM nginx:alpine
COPY src/wallet/web /usr/share/nginx/html
COPY --from=builder /app/pkg /usr/share/nginx/html/pkg

# Nginx config: CORS + WASM MIME
COPY src/wallet/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://localhost:3000/health || exit 1

CMD ["nginx", "-g", "daemon off;"]
