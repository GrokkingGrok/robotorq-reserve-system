FROM rust:1.88 AS builder
WORKDIR /app

# Install tools
RUN rustup target add wasm32-unknown-unknown && \
    cargo install wasm-pack

# Copy ALL source first
COPY . .

# Now fetch + build
RUN cargo fetch
RUN wasm-pack build --target web --out-dir pkg --release

# Final
FROM nginx:alpine
COPY web /usr/share/nginx/html
COPY --from=builder /app/pkg /usr/share/nginx/html/pkg
COPY nginx.conf /etc/nginx/nginx.conf
EXPOSE 3000
CMD ["nginx", "-g", "daemon off;"]