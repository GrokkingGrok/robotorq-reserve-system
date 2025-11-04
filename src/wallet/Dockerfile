FROM rust:1.82 AS builder
WORKDIR /app
RUN rustup target add wasm32-unknown-unknown && cargo install wasm-pack
COPY Cargo.* ./
RUN cargo fetch
COPY . .
RUN wasm-pack build --target web --out-dir pkg --release

FROM nginx:alpine
COPY web /usr/share/nginx/html
COPY --from=builder /app/pkg /usr/share/nginx/html/pkg
EXPOSE 3000
CMD ["nginx", "-g", "daemon off;"]
